#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
/*!

`lut-synth`: LUT Network Synthesis with E-Graphs

TODO: overview, tutorial, testing, research papers

*/

pub mod analysis;
pub mod check;
pub mod cost;
pub mod driver;
pub mod lut;
pub mod rewrite;
pub mod verilog;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use analysis::LutAnalysis;
    use egg::{Analysis, Language, RecExpr};
    use lut::{verify_expr, LutExprInfo, LutLang};
    use verilog::{sv_parse_wrapper, SVModule, SVPrimitive};

    use super::*;

    #[test]
    fn test_swap() {
        // Need to be able to represent 3
        assert_eq!(lut::from_bitvec(&lut::to_bitvec(3, 2).unwrap()), 3);
        let tt: u64 = 0b1010;
        let swapped = lut::swap_pos(&tt, 2, 0);
        assert_eq!(swapped, 12);
    }

    #[test]
    fn test_swap2() {
        assert_eq!(lut::swap_pos(&2, 2, 0), 4);
    }

    fn make_simple_nested_lut() -> RecExpr<lut::LutLang> {
        "(LUT 51952 s1 (LUT 61642 s1 s0 c d) a b)".parse().unwrap()
    }

    fn make_four_lut() -> RecExpr<lut::LutLang> {
        "(LUT 44234 s1 s0 b a)".parse().unwrap()
    }

    fn make_three_lut() -> RecExpr<lut::LutLang> {
        "(LUT 202 s0 a b)".parse().unwrap()
    }

    #[test]
    fn test_get_lut_count() {
        assert_eq!(
            2,
            LutExprInfo::new(&make_simple_nested_lut()).get_lut_count()
        );
        assert_eq!(1, LutExprInfo::new(&make_four_lut()).get_lut_count());
        assert_eq!(1, LutExprInfo::new(&make_three_lut()).get_lut_count());
    }

    #[test]
    fn test_get_lut_k_count() {
        let lut = make_simple_nested_lut();
        let info = LutExprInfo::new(&lut);
        assert_eq!(2, info.get_lut_count_k(4));
        assert_eq!(0, info.get_lut_count_k(3));
        assert_eq!(2, info.get_circuit_depth());
        let lut = make_four_lut();
        let info = LutExprInfo::new(&lut);
        assert_eq!(1, info.get_lut_count_k(4));
        assert_eq!(0, info.get_lut_count_k(6));
        let lut = make_three_lut();
        let info = LutExprInfo::new(&lut);
        assert_eq!(1, info.get_lut_count_k(3));
        assert_eq!(0, info.get_lut_count_k(6));
    }

    #[test]
    fn test_analysis() {
        let const_val = true;
        let prog = 1337;
        let const_true = LutLang::Const(const_val);
        let prog_node = LutLang::Program(prog);
        let egraph = egg::EGraph::default();
        let const_analysis = LutAnalysis::make(&egraph, &const_true);
        let prog_analysis = LutAnalysis::make(&egraph, &prog_node);
        assert_eq!(const_analysis.get_as_const(), Ok(const_val));
        assert_eq!(prog_analysis.get_program(), Ok(prog));
        assert!(const_analysis.get_program().is_err());
        assert!(prog_analysis.get_as_const().is_err());
        assert!(!const_analysis.is_an_input());
        assert!(!prog_analysis.is_an_input());
    }

    #[test]
    fn test_program_formats() {
        let prog = u64::MAX;
        assert!(lut::to_bitvec(prog, 63).is_err());
        let bv = lut::to_bitvec(prog, 64);
        assert!(bv.is_ok());
        assert_eq!(prog, lut::from_bitvec(&bv.unwrap()));
    }

    #[test]
    fn test_principal_inputs() {
        let input = "a";
        let input_node = LutLang::Var(input.to_string().into());
        let egraph = egg::EGraph::default();
        let input_analysis = LutAnalysis::make(&egraph, &input_node);
        assert!(input_analysis.is_an_input());
        assert!(input_analysis.get_as_const().is_err());
        assert!(input_analysis.get_program().is_err());
    }

    #[test]
    fn test_bad_var() {
        let input = "NOR";
        let input_node = LutLang::Var(input.to_string().into());
        let mut expr: RecExpr<LutLang> = RecExpr::default();
        expr.add(input_node.clone());
        assert!(input_node.verify_rec(&expr).is_err());
    }

    #[test]
    fn test_lut_too_big() {
        let input = "a";
        let input_node = LutLang::Var(input.to_string().into());
        let mut expr: RecExpr<LutLang> = RecExpr::default();
        let id = expr.add(input_node.clone());
        let lut =
            LutLang::Lut(vec![expr.add(LutLang::Program(0)), id, id, id, id, id, id, id].into());
        expr.add(lut.clone());
        assert!(lut.verify_rec(&expr).is_err());
        assert!(lut.get_program(&expr).is_err());
        assert!(lut.get_lut_size().is_err());
    }

    #[test]
    fn test_missing_program() {
        let input = "a";
        let input_node = LutLang::Var(input.to_string().into());
        let mut expr: RecExpr<LutLang> = RecExpr::default();
        let id = expr.add(input_node.clone());
        let lut = LutLang::Lut(vec![id, id, id, id, id, id].into());
        expr.add(lut.clone());
        assert!(lut.verify_rec(&expr).is_err());
        assert!(lut.get_program(&expr).is_err());
        assert!(verify_expr(&expr).is_err());
    }

    fn get_struct_verilog() -> String {
        "module mux_4_1 (
            a,
            b,
            c,
            d,
            s0,
            s1,
            y
        );
          input a;
          wire a;
          input b;
          wire b;
          input c;
          wire c;
          input d;
          wire d;
          input s0;
          wire s0;
          input s1;
          wire s1;
          output y;
          wire y;
          LUT6 #(
              .INIT(64'hf0f0ccccff00aaaa)
          ) _0_ (
              .I0(d),
              .I1(c),
              .I2(a),
              .I3(b),
              .I4(s1),
              .I5(s0),
              .O (y)
          );
        endmodule"
            .to_string()
    }

    fn get_fdre_verilog() -> String {
        "module mux_4_1 (
            d,
            clk,
            y
        );
          input d;
          wire d;
          input clk;
          wire clk;
          output y;
          wire y;
          FDRE #(
              .INIT(1'hx)
          ) _0_ (
              .C (clk),
              .CE(1'h1),
              .D (d),
              .Q (y),
              .R (1'h0)
          );
        endmodule"
            .to_string()
    }

    fn get_assignment_verilog() -> String {
        "module passthru (
    d,
    y
);
  input d;
  wire d;
  output y;
  wire y;
  assign y = d;
endmodule"
            .to_string()
    }

    #[test]
    fn test_assignment_verilog() {
        let module = get_assignment_verilog();
        let ast = sv_parse_wrapper(&module, None).unwrap();
        let module = SVModule::from_ast(&ast);
        assert!(module.is_ok());
        let module = module.unwrap();
        assert_eq!(
            module.to_single_expr().unwrap().to_string(),
            "d".to_string()
        );
    }

    #[test]
    fn test_assignment_emission() {
        let expr: RecExpr<LutLang> = "d".parse().unwrap();
        let module = SVModule::from_expr(expr, "passthru".to_string(), vec!["y".to_string()]);
        assert!(module.is_ok());
        let module = module.unwrap();
        assert_eq!(module.to_string(), get_assignment_verilog());
    }

    #[test]
    fn test_duplicate_assignment() {
        let expr: RecExpr<LutLang> = "(BUS d d)".parse().unwrap();
        let module = SVModule::from_expr(expr, "passthru".to_string(), vec![]);
        assert!(module.is_ok());
        let module = module.unwrap();
        let correct = "module passthru (
    d,
    y0,
    y1
);
  input d;
  wire d;
  output y0;
  wire y0;
  output y1;
  wire y1;
  assign y1 = y0;
  assign y0 = d;
endmodule"
            .to_string();
        assert_eq!(module.to_string(), correct);
    }

    #[test]
    fn test_parse_verilog() {
        let module = get_struct_verilog();
        let ast = sv_parse_wrapper(&module, None).unwrap();
        let module = SVModule::from_ast(&ast);
        assert!(module.is_ok());
        let module = module.unwrap();
        assert_eq!(module.instances.len(), 1);
        assert_eq!(module.inputs.len(), 6);
        assert_eq!(module.outputs.len(), 1);
        assert_eq!(module.name, "mux_4_1");
        let instance = module.instances.first().unwrap();
        assert_eq!(instance.prim, "LUT6");
        assert_eq!(instance.name, "_0_");
        assert_eq!(instance.attributes.len(), 1);
        assert_eq!(instance.attributes["INIT"], "64'hf0f0ccccff00aaaa");
    }

    #[test]
    fn test_verilog_roundtrip() {
        let module = get_struct_verilog();
        let ast = sv_parse_wrapper(&module, None).unwrap();
        let module = SVModule::from_ast(&ast).unwrap();
        let output = module.to_string();
        // This test is so ugly >:(
        let golden = "module mux_4_1 (
    a,
    b,
    c,
    d,
    s0,
    s1,
    y
);
  input a;
  wire a;
  input b;
  wire b;
  input c;
  wire c;
  input d;
  wire d;
  input s0;
  wire s0;
  input s1;
  wire s1;
  output y;
  wire y;
  LUT6 #(
      .INIT(64'hf0f0ccccff00aaaa)
  ) _0_ (
      .I0(d),
      .I1(c),
      .I2(a),
      .I3(b),
      .I4(s1),
      .I5(s0),
      .O(y)
  );
endmodule"
            .to_string();
        assert_eq!(output, golden);
    }

    #[test]
    fn test_verilog_to_expr() {
        let module = get_struct_verilog();
        let ast = sv_parse_wrapper(&module, None).unwrap();
        let module = SVModule::from_ast(&ast)
            .unwrap()
            .with_fname("mux_4_1".to_string());
        assert!(module.name == "mux_4_1");
        let expr = module.to_expr().unwrap();
        assert_eq!(
            expr.to_string(),
            "(LUT 17361601744336890538 s0 s1 b a c d)".to_string()
        );
    }

    #[test]
    fn test_fdre_verilog() {
        let module = get_fdre_verilog();
        let ast = sv_parse_wrapper(&module, None).unwrap();
        let module = SVModule::from_ast(&ast);
        assert!(module.is_ok());
        let module = module.unwrap();
        assert_eq!(
            module.to_single_expr().unwrap().to_string(),
            "(REG d)".to_string()
        );
        let output = module.to_string();
        let golden = "module mux_4_1 (
    d,
    clk,
    y
);
  input d;
  wire d;
  input clk;
  wire clk;
  output y;
  wire y;
  FDRE #(
      .INIT(1'hx)
  ) _0_ (
      .C(clk),
      .CE(1'h1),
      .D(d),
      .R(1'h0),
      .Q(y)
  );
endmodule"
            .to_string();
        assert_eq!(output, golden);
    }

    #[test]
    fn test_gate_parse() {
        let module = "module my_gate (
            a,
            b,
            y
        );
          input a;
          wire a;
          input b;
          wire b;
          output y;
          wire y;
          AND2 _0_ (
              .A(a),
              .B(b),
              .Y(y)
          );
        endmodule"
            .to_string();
        let ast = sv_parse_wrapper(&module, None).unwrap();
        let module = SVModule::from_ast(&ast);
        assert!(module.is_ok());
        let module = module.unwrap();
        assert_eq!(
            module.to_single_expr().unwrap().to_string(),
            "(AND a b)".to_string()
        );
    }

    #[test]
    fn test_constant_parse() {
        let module = "module my_gates (
            y
        );
        output y;
        wire y;
        wire tmp1;
        wire tmp2;

        AND2 _0_ (
            .A(1'd1),
            .B(1'h01),
            .Y(tmp1)
        );

        AND2 _1_ (
            .A(1'b00),
            .B(1'd0),
            .Y(tmp2)
        );

        AND2 _2_ (
            .A(tmp1),
            .B(tmp2),
            .Y(y)
        );

        endmodule"
            .to_string();
        let ast = sv_parse_wrapper(&module, None).unwrap();
        let module = SVModule::from_ast(&ast);
        assert!(module.is_ok());
        let module = module.unwrap();
        assert_eq!(
            module.to_single_expr().unwrap().to_string(),
            "(AND (AND true true) (AND false false))".to_string()
        );
    }

    #[test]
    fn test_verilog_emitter() {
        let mux: RecExpr<LutLang> = "(LUT 202 s1 (LUT 202 s0 a b) (LUT 202 s0 c d))"
            .parse()
            .unwrap();
        let module = SVModule::from_expr(mux, "mux_4_1".to_string(), Vec::new());
        assert!(module.is_ok());
        let module = module.unwrap();
        let golden = "module mux_4_1 (
    s1,
    s0,
    a,
    b,
    c,
    d,
    y
);
  input s1;
  wire s1;
  input s0;
  wire s0;
  input a;
  wire a;
  input b;
  wire b;
  input c;
  wire c;
  input d;
  wire d;
  output y;
  wire y;
  wire tmp5;
  wire tmp8;
  LUT3 #(
      .INIT(8'hca)
  ) __0__ (
      .I0(b),
      .I1(a),
      .I2(s0),
      .O(tmp5)
  );
  LUT3 #(
      .INIT(8'hca)
  ) __1__ (
      .I0(d),
      .I1(c),
      .I2(s0),
      .O(tmp8)
  );
  LUT3 #(
      .INIT(8'hca)
  ) __2__ (
      .I0(tmp8),
      .I1(tmp5),
      .I2(s1),
      .O(y)
  );
endmodule"
            .to_string();
        assert_eq!(module.to_string(), golden);
    }

    #[test]
    fn test_emit_reg() {
        let reg: RecExpr<LutLang> = "(REG a)".parse().unwrap();
        let module = SVModule::from_expr(reg, "my_reg".to_string(), Vec::new());
        assert!(module.is_ok());
        let module = module.unwrap();
        let golden = "module my_reg (
    a,
    clk,
    y
);
  input a;
  wire a;
  input clk;
  wire clk;
  output y;
  wire y;
  FDRE #(
      .INIT(1'hx)
  ) __0__ (
      .C(clk),
      .CE(1'h1),
      .D(a),
      .R(1'h0),
      .Q(y)
  );
endmodule"
            .to_string();
        assert_eq!(module.to_string(), golden);
    }

    #[test]
    fn test_emit_gates() {
        let expr: RecExpr<LutLang> = "(AND a (XOR b (NOR c (NOT (MUX s t false)))))"
            .parse()
            .unwrap();
        let module = SVModule::from_expr(expr, "gate_list".to_string(), Vec::new());
        assert!(module.is_ok());
        let module = module.unwrap();
        let golden = "module gate_list (
    a,
    b,
    c,
    s,
    t,
    y
);
  input a;
  wire a;
  input b;
  wire b;
  input c;
  wire c;
  input s;
  wire s;
  input t;
  wire t;
  output y;
  wire y;
  wire tmp6;
  wire tmp7;
  wire tmp8;
  wire tmp9;
  wire tmp10;
  assign tmp6 = 1'b0;
  MUX #(
  ) __1__ (
      .A(t),
      .B(tmp6),
      .S(s),
      .Y(tmp7)
  );
  NOT #(
  ) __2__ (
      .A(tmp7),
      .Y(tmp8)
  );
  NOR2 #(
  ) __3__ (
      .A(c),
      .B(tmp8),
      .Y(tmp9)
  );
  XOR2 #(
  ) __4__ (
      .A(b),
      .B(tmp9),
      .Y(tmp10)
  );
  AND2 #(
  ) __5__ (
      .A(a),
      .B(tmp10),
      .Y(y)
  );
endmodule"
            .to_string();
        assert_eq!(module.to_string(), golden);
    }

    #[test]
    fn test_primitive_connections() {
        let mut prim = SVPrimitive::new_lut(4, "_0_".to_string(), 1);
        assert!(prim.add_signal("I8".to_string(), "a".to_string()).is_err());
        assert!(prim.add_signal("I0".to_string(), "a".to_string()).is_ok());
        assert!(prim.add_signal("I0".to_string(), "a".to_string()).is_err());
        assert!(prim.add_signal("Y".to_string(), "b".to_string()).is_ok());
        assert!(prim.add_signal("Y".to_string(), "b".to_string()).is_err());
        assert!(prim.add_signal("bad".to_string(), "a".to_string()).is_err());
    }

    #[test]
    fn test_bus_type() {
        let bus: RecExpr<LutLang> = "(BUS (LUT 202 s0 a b) (MUX s0 a b))".parse().unwrap();
        let swapped: RecExpr<LutLang> = "(BUS (MUX s0 a b) (LUT 202 s0 a b))".parse().unwrap();
        assert!(LutLang::func_equiv(&bus, &swapped).unwrap());
    }

    #[test]
    fn test_bad_bus() {
        let bus: RecExpr<LutLang> =
            "(BUS (BUS (LUT 202 s0 a b) (MUX s0 a b)) (BUS (LUT 202 s0 a b) (MUX s0 a b)))"
                .parse()
                .unwrap();
        let root = bus.as_ref().last().unwrap();
        assert!(root.verify_rec(&bus).is_err());

        let bus: RecExpr<LutLang> = "(BUS 1234 12346)".parse().unwrap();
        let root = bus.as_ref().last().unwrap();
        assert!(root.verify_rec(&bus).is_err());

        let bus: RecExpr<LutLang> = "(BUS a1 a2)".parse().unwrap();
        let root = bus.as_ref().last().unwrap();
        assert!(root.verify_rec(&bus).is_ok());
    }

    #[test]
    fn test_not_equiv() {
        let xor: RecExpr<LutLang> = "(XOR a b)".parse().unwrap();
        let nor: RecExpr<LutLang> = "(NOR a b)".parse().unwrap();
        assert!(LutLang::func_equiv(&xor, &"(LUT 6 a b)".parse().unwrap()).unwrap());
        assert!(!LutLang::func_equiv(&xor, &"(LUT 4 a b)".parse().unwrap()).unwrap());
        assert!(!LutLang::func_equiv(&xor, &"(LUT 2 a b)".parse().unwrap()).unwrap());
        assert!(LutLang::func_equiv(&nor, &"(LUT 1 a b)".parse().unwrap()).unwrap());
        assert!(!LutLang::func_equiv(&xor, &nor).unwrap());
    }

    #[test]
    fn test_dominance() {
        let bus: RecExpr<LutLang> =
            "(BUS (XOR (XOR a b) cin) (NOT (NOR (AND a b) (AND cin (XOR a b)))) (XOR a b) (AND a b))"
                .parse()
                .unwrap();
        let bus_node = bus.as_ref().last().unwrap();
        let sum = bus_node.children()[0];
        let carry = bus_node.children()[1];
        let xor = bus_node.children()[2];
        let and = bus_node.children()[3];
        let info = LutExprInfo::new(&bus);
        assert!(!info.dominates(sum, carry).unwrap());
        assert!(!info.dominates(carry, sum).unwrap());
        assert!(info.dominates(sum, sum).unwrap());
        assert!(info.dominates(carry, carry).unwrap());
        assert!(info.dominates(xor, carry).unwrap());
        assert!(info.dominates(xor, sum).unwrap());
        assert!(info.dominates(and, carry).unwrap());
        assert!(!info.dominates(and, sum).unwrap());
    }

    #[test]
    fn test_canonical() {
        let bus: RecExpr<LutLang> =
            "(BUS (XOR (XOR a b) cin) (NOT (NOR (AND a b) (AND cin (XOR a b)))) (XOR a b))"
                .parse()
                .unwrap();
        let info = LutExprInfo::new(&bus);

        // The egg implementation of parsing does not reuse common expressions
        // This is annoying
        assert!(info.is_reduntant());
        assert!(info.contains_gates());
        assert!(!info.is_canonical());
    }

    #[test]
    fn test_dont_care() {
        let const_false: RecExpr<LutLang> = "false".parse().unwrap();
        let short_circuit: RecExpr<LutLang> = "(AND false x)".parse().unwrap();
        let res = LutLang::eval(&short_circuit, &HashMap::new());
        assert!(res.is_ok());
        let check = LutLang::func_equiv(&const_false, &short_circuit);
        assert!(check.is_equiv());
        let check = LutLang::func_equiv(
            &"true".parse().unwrap(),
            &"(NOT (NOR x true))".parse().unwrap(),
        );
        assert!(check.is_equiv());
    }

    #[test]
    fn test_reg() {
        // Make sure any expression that include reg return inconclusive equivalence
        let simple_reg_expr: RecExpr<LutLang> = "(REG a)".parse().unwrap();
        assert!(LutLang::func_equiv(&simple_reg_expr, &"(REG a)".parse().unwrap()).is_equiv());
        assert!(
            LutLang::func_equiv(&simple_reg_expr, &"(AND a b)".parse().unwrap()).is_inconclusive()
        );
        assert!(
            LutLang::func_equiv(&simple_reg_expr, &"(XOR c (REG d))".parse().unwrap())
                .is_inconclusive()
        );
        let compicated_reg_expr: RecExpr<LutLang> =
            "AND (AND a b) (XOR (AND c (REG a)) d)".parse().unwrap();
        assert!(LutLang::func_equiv(&compicated_reg_expr, &simple_reg_expr).is_inconclusive());
    }

    #[test]
    fn test_cycle() {
        let simple_cycle_expr: RecExpr<LutLang> = "(CYCLE (REG (AND a (ARG 0))))".parse().unwrap();
        assert!(LutLang::func_equiv(
            &simple_cycle_expr,
            &"(CYCLE (REG (AND a (ARG 0))))".parse().unwrap()
        )
        .is_equiv());
        let complex_cycle_expr: RecExpr<LutLang> =
            "(CYCLE (XOR (ARG 0) (CYCLE (REG (AND a (ARG 1))))))"
                .parse()
                .unwrap();
        assert!(LutLang::func_equiv(
            &complex_cycle_expr,
            &"(CYCLE (XOR (ARG 0) (CYCLE (REG (AND a (ARG 1))))))"
                .parse()
                .unwrap()
        )
        .is_equiv());
        let eval_cycle_expr: RecExpr<LutLang> = "(CYCLE (REG in))".parse().unwrap();
        assert!(
            LutLang::func_equiv(&eval_cycle_expr, &"(REG in)".parse().unwrap()).is_inconclusive()
        );
        assert!(LutLang::func_equiv(
            &simple_cycle_expr,
            &"(CYCLE (REG (AND (XOR (AND a 1) (ARG 0)) (ARG 0))))"
                .parse()
                .unwrap()
        )
        .is_inconclusive());
    }

    #[test]
    fn test_cycle_verify() {
        let bad_cycle: RecExpr<LutLang> = "(CYCLE (REG (AND a (ARG myarg))))".parse().unwrap();
        let root = bad_cycle.as_ref().last().unwrap();
        assert!(root.verify_rec(&bad_cycle).is_err());

        let good_cycle: RecExpr<LutLang> = "(CYCLE (REG (AND a (ARG 0))))".parse().unwrap();
        let root = good_cycle.as_ref().last().unwrap();
        assert!(root.verify_rec(&good_cycle).is_ok());

        let bad_cycle: RecExpr<LutLang> = "(CYCLE (REG (AND a (ARG 1))))".parse().unwrap();
        let root = bad_cycle.as_ref().last().unwrap();
        assert!(root.verify_rec(&bad_cycle).is_err());
    }

    #[test]
    fn test_interface_count() {
        let expr: RecExpr<LutLang> = "(MUX s a b)".parse().unwrap();
        let info = LutExprInfo::new(&expr);
        assert_eq!(info.get_num_inputs(), 3);
        assert_eq!(info.get_num_outputs(), 1);
        let canon = info.get_canonicalization();
        let info = LutExprInfo::new(&canon);
        assert_eq!(info.get_num_inputs(), 3);
        assert_eq!(info.get_num_outputs(), 1);
        assert_eq!(canon.to_string(), "(LUT 202 s a b)".to_string());
    }

    #[test]
    fn test_circuit_stats() {
        let expr: RecExpr<LutLang> = "(LUT 202 s a b)".parse().unwrap();
        let info = LutExprInfo::new(&expr);
        let stats = info.get_circuit_stats();
        assert_eq!(stats.depth, 1);
        assert_eq!(stats.lut_count, 1);
    }
}
