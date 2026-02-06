#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
/*!

`eqmap`: FPGA LUT Remapping using E-Graphs

This Verilog-to-Verilog tool seeks to evaluate the use of logic rewriting and equality saturation for improving FPGA technology mapping.

The LUT representation can be found in the [lut] module, whereas a lot of the bit-twiddling of truth tables happens in [rewrite]. [driver] has all the code related to tooling and end-to-end integration.

`eqmap_fpga` is the main CLI tool, and you can see how it is used in some of the CI [tests](https://github.com/matth2k/eqmap/tree/main/tests/verilog). Lastly, eqmap is bundled together with yosys in the [eqmap](https://github.com/matth2k/eqmap/blob/main/bin/eqmap) script. This is the script that should be used to process your own verilog:

```bash
$ eqmap --help
Technology Mapping Optimization with E-Graphs

Usage: eqmap_fpga [OPTIONS] [INPUT] [OUTPUT]

Arguments:
  [INPUT]   Verilog file to read from (or use stdin)
  [OUTPUT]  Verilog file to output to (or use stdout)

Options:
      --report <REPORT>            If provided, output a JSON file with result data
  -a, --assert-sat                 Return an error if the graph does not reach saturation
  -f, --no-verify                  Do not verify the functionality of the output
  -c, --no-canonicalize            Do not canonicalize the input into LUTs
  -d, --decomp                     Find new decompositions at runtime
      --disassemble <DISASSEMBLE>  Comma separated list of cell types to decompose into
  -r, --no-retime                  Do not use register retiming
  -v, --verbose                    Print explanations (generates a proof and runs slower)
      --min-depth                  Extract for minimum circuit depth
  -k, --k <K>                      Max fan in size allowed for extracted LUTs [default: 6]
  -w, --reg-weight <REG_WEIGHT>    Ratio of register cost to LUT cost [default: 1]
  -t, --timeout <TIMEOUT>          Build/extraction timeout in seconds
  -s, --node-limit <NODE_LIMIT>    Maximum number of nodes in graph
  -n, --iter-limit <ITER_LIMIT>    Maximum number of rewrite iterations
  -h, --help                       Print help
  -V, --version                    Print version
```

*/

pub mod analysis;
pub mod asic;
pub mod check;
pub mod cost;
pub mod driver;
#[cfg(feature = "rewrite_file")]
pub mod file_rewrites;
pub mod logic;
pub mod lut;
pub mod netlist;
pub mod rewrite;
#[cfg(feature = "graph_dumps")]
pub mod serialize;
pub mod verilog;
#[cfg(feature = "rewrite_file")]
pub mod rewrite_file;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use analysis::LutAnalysis;
    use asic::{CellAnalysis, CellLang, CellRpt, asic_rewrites};
    use driver::{Canonical, SynthRequest};
    use egg::{Analysis, Language, RecExpr};
    use lut::{LutExprInfo, LutLang};
    use verilog::{PrimitiveType, SVModule, sv_parse_wrapper};

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
        assert_eq!(6, info.get_inputs().len());
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
        let mut egraph = egg::EGraph::default();
        let const_analysis = LutAnalysis::make(&mut egraph, &const_true, egg::Id::default());
        let prog_analysis = LutAnalysis::make(&mut egraph, &prog_node, egg::Id::default());
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
        let mut egraph = egg::EGraph::default();
        let input_analysis = LutAnalysis::make(&mut egraph, &input_node, egg::Id::default());
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
        assert!(LutLang::verify_expr(&expr).is_err());
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

    fn get_const_verilog() -> String {
        "module mux_4_1 (
            a,
            b,
            c,
            d,
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
          wire s0;
          wire s1;
          output y;
          wire y;
          VCC VCC (.Y(s0));
          GND GND (.Y(s1));
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
            ce,
            rst,
            y
        );
          input d;
          wire d;
          input clk;
          wire clk;
          input ce;
          wire ce;
          input rst;
          wire rst;
          output y;
          wire y;
          FDRE #(
              .INIT(1'hx)
          ) _0_ (
              .C (clk),
              .CE(ce),
              .D (d),
              .Q (y),
              .R (rst)
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
endmodule\n"
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
            module.to_single_lut_expr().unwrap().to_string(),
            "d".to_string()
        );
    }

    #[test]
    fn test_assignment_emission() {
        let expr: RecExpr<LutLang> = "d".parse().unwrap();
        let module = SVModule::from_luts(expr, "passthru".to_string(), vec!["y".to_string()]);
        assert!(module.is_ok());
        let module = module.unwrap();
        assert_eq!(module.to_string(), get_assignment_verilog());
    }

    #[test]
    fn test_duplicate_assignment() {
        let expr: RecExpr<LutLang> = "(BUS d d)".parse().unwrap();
        let module = SVModule::from_luts(expr, "passthru".to_string(), vec![]);
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
endmodule\n"
            .to_string();
        assert_eq!(module.to_string(), correct);
    }

    #[test]
    fn test_constant_verilog() {
        let module = get_const_verilog();
        let ast = sv_parse_wrapper(&module, None).unwrap();
        let module = SVModule::from_ast(&ast);
        assert!(module.is_ok());
        let module = module.unwrap();
        assert_eq!(
            module.to_single_lut_expr().unwrap().to_string(),
            "(LUT 17361601744336890538 true false b a c d)".to_string()
        );
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
endmodule\n"
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
        assert!(module.get_name() == "mux_4_1");
        let expr: RecExpr<LutLang> = module.to_expr().unwrap();
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
            module.to_single_lut_expr().unwrap().to_string(),
            "(REG d clk ce rst)".to_string()
        );
        let output = module.to_string();
        let golden = "module mux_4_1 (
    d,
    clk,
    ce,
    rst,
    y
);
  input d;
  wire d;
  input clk;
  wire clk;
  input ce;
  wire ce;
  input rst;
  wire rst;
  output y;
  wire y;
  FDRE #(
      .INIT(1'hx)
  ) _0_ (
      .C(clk),
      .CE(ce),
      .D(d),
      .R(rst),
      .Q(y)
  );
endmodule\n"
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
          AND _0_ (
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
            module.to_single_lut_expr().unwrap().to_string(),
            "(AND a b)".to_string()
        );
    }

    #[test]
    fn test_cell_parse() {
        let module = "module my_cell (
            a,
            b,
            c,
            y
        );
          input a;
          wire a;
          input b;
          wire b;
          input c;
          wire c;
          output y;
          wire y;
          AOI21_X1 _0_ (
              .A(a),
              .B1(b),
              .B2(c),
              .ZN(y)
          );
        endmodule"
            .to_string();
        let ast = sv_parse_wrapper(&module, None).unwrap();
        let module = SVModule::from_ast(&ast);
        assert!(module.is_ok());
        let module = module.unwrap();
        assert_eq!(
            module.to_expr::<CellLang>().unwrap().to_string(),
            "(AOI21_X1 a b c)".to_string()
        );
    }

    #[test]
    fn test_cell_parse_and() {
        let module = "module my_cell (
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
          AND _0_ (
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
        let expr = module.to_expr::<CellLang>().unwrap();
        assert!(matches!(expr.last().unwrap(), CellLang::And(_)));
        assert_eq!(
            module.to_expr::<CellLang>().unwrap().to_string(),
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

        AND _0_ (
            .A(1'd1),
            .B(1'h01),
            .Y(tmp1)
        );

        AND _1_ (
            .A(1'b00),
            .B(1'd0),
            .Y(tmp2)
        );

        AND _2_ (
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
            module.to_single_lut_expr().unwrap().to_string(),
            "(AND (AND true true) (AND false false))".to_string()
        );
    }

    #[test]
    fn test_double_inverter() {
        let module = "module dinv (
            d,
            y
        );
          input d;
          wire d;
          output y;
          wire y;
          wire __0__;

          NOT _1_ (
              .A(d),
              .Y(__0__)
          );

          INV _2_ (
              .A (__0__),
              .ZN(y)
          );

        endmodule
        "
        .to_string();
        let ast = sv_parse_wrapper(&module, None).unwrap();
        let module = SVModule::from_ast(&ast);
        assert!(module.is_ok());
        let module = module.unwrap();
        assert_eq!(
            module.to_single_lut_expr().unwrap().to_string(),
            "(NOT (NOT d))".to_string()
        );
    }

    #[test]
    fn test_verilog_emitter() {
        let mux: RecExpr<LutLang> = "(LUT 202 s1 (LUT 202 s0 a b) (LUT 202 s0 c d))"
            .parse()
            .unwrap();
        let module = SVModule::from_luts(mux, "mux_4_1".to_string(), Vec::new());
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
  wire __0__;
  wire __1__;
  LUT3 #(
      .INIT(8'hca)
  ) __2__ (
      .I0(b),
      .I1(a),
      .I2(s0),
      .O(__0__)
  );
  LUT3 #(
      .INIT(8'hca)
  ) __3__ (
      .I0(d),
      .I1(c),
      .I2(s0),
      .O(__1__)
  );
  LUT3 #(
      .INIT(8'hca)
  ) __4__ (
      .I0(__1__),
      .I1(__0__),
      .I2(s1),
      .O(y)
  );
endmodule\n"
            .to_string();
        assert_eq!(module.to_string(), golden);
    }

    #[test]
    fn test_emit_reg() {
        let reg: RecExpr<LutLang> = "(REG a clk ce rst)".parse().unwrap();
        let module = SVModule::from_luts(reg, "my_reg".to_string(), Vec::new());
        assert!(module.is_ok());
        let module = module.unwrap();
        let golden = "module my_reg (
    a,
    clk,
    ce,
    rst,
    y
);
  input a;
  wire a;
  input clk;
  wire clk;
  input ce;
  wire ce;
  input rst;
  wire rst;
  output y;
  wire y;
  FDRE #(
      .INIT(1'hx)
  ) __0__ (
      .C(clk),
      .CE(ce),
      .D(a),
      .R(rst),
      .Q(y)
  );
endmodule\n"
            .to_string();
        assert_eq!(module.to_string(), golden);
    }

    #[test]
    fn test_emit_gates() {
        let expr: RecExpr<LutLang> = "(AND a (XOR b (NOR c (NOT (MUX s t false)))))"
            .parse()
            .unwrap();
        let module = SVModule::from_luts(expr, "gate_list".to_string(), Vec::new());
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
  wire __0__;
  wire __1__;
  wire __2__;
  wire __3__;
  wire __4__;
  assign __0__ = 1'b0;
  MUX #(
  ) __6__ (
      .A(t),
      .B(__0__),
      .S(s),
      .Y(__1__)
  );
  NOT #(
  ) __7__ (
      .A(__1__),
      .Y(__2__)
  );
  NOR #(
  ) __8__ (
      .A(c),
      .B(__2__),
      .Y(__3__)
  );
  XOR #(
  ) __9__ (
      .A(b),
      .B(__3__),
      .Y(__4__)
  );
  AND #(
  ) __10__ (
      .A(a),
      .B(__4__),
      .Y(y)
  );
endmodule\n"
            .to_string();
        assert_eq!(module.to_string(), golden);
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
        let simple_reg_expr: RecExpr<LutLang> = "(REG a clk ce rst)".parse().unwrap();
        assert!(
            LutLang::func_equiv(&simple_reg_expr, &"(REG a clk ce rst)".parse().unwrap())
                .is_equiv()
        );
        assert!(
            LutLang::func_equiv(&simple_reg_expr, &"(AND a b)".parse().unwrap()).is_inconclusive()
        );
        assert!(
            LutLang::func_equiv(
                &simple_reg_expr,
                &"(XOR c (REG d clk ce rst))".parse().unwrap()
            )
            .is_inconclusive()
        );
        let compicated_reg_expr: RecExpr<LutLang> =
            "AND (AND a b) (XOR (AND c (REG a clk ce rst)) d)"
                .parse()
                .unwrap();
        assert!(LutLang::func_equiv(&compicated_reg_expr, &simple_reg_expr).is_inconclusive());
    }

    #[test]
    fn test_cycle() {
        let simple_cycle_expr: RecExpr<LutLang> =
            "(CYCLE (REG (AND a (ARG 0)) clk ce rst))".parse().unwrap();
        assert!(
            LutLang::func_equiv(
                &simple_cycle_expr,
                &"(CYCLE (REG (AND a (ARG 0)) clk ce rst))".parse().unwrap()
            )
            .is_equiv()
        );
        let complex_cycle_expr: RecExpr<LutLang> =
            "(CYCLE (XOR (ARG 0) (CYCLE (REG (AND a (ARG 1)) clk ce rst))))"
                .parse()
                .unwrap();
        assert!(
            LutLang::func_equiv(
                &complex_cycle_expr,
                &"(CYCLE (XOR (ARG 0) (CYCLE (REG (AND a (ARG 1)) clk ce rst))))"
                    .parse()
                    .unwrap()
            )
            .is_equiv()
        );
        let eval_cycle_expr: RecExpr<LutLang> = "(CYCLE (REG in clk ce rst))".parse().unwrap();
        assert!(
            LutLang::func_equiv(&eval_cycle_expr, &"(REG in clk ce rst)".parse().unwrap())
                .is_inconclusive()
        );
        assert!(
            LutLang::func_equiv(
                &simple_cycle_expr,
                &"(CYCLE (REG (AND (XOR (AND a 1) (ARG 0)) (ARG 0)) clk ce rst))"
                    .parse()
                    .unwrap()
            )
            .is_inconclusive()
        );
    }

    #[test]
    fn test_cycle_verify() {
        let bad_cycle: RecExpr<LutLang> = "(CYCLE (REG (AND a (ARG myarg)) clk ce rst))"
            .parse()
            .unwrap();
        let root = bad_cycle.as_ref().last().unwrap();
        assert!(root.verify_rec(&bad_cycle).is_err());

        let good_cycle: RecExpr<LutLang> =
            "(CYCLE (REG (AND a (ARG 0)) clk ce rst))".parse().unwrap();
        let root = good_cycle.as_ref().last().unwrap();
        assert!(root.verify_rec(&good_cycle).is_ok());

        let bad_cycle: RecExpr<LutLang> =
            "(CYCLE (REG (AND a (ARG 1)) clk ce rst))".parse().unwrap();
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

    #[test]
    fn test_celllang() {
        let expr: RecExpr<CellLang> = "(LUT 202 s a b)".parse().unwrap();
        assert!(CellLang::verify_expr(&expr).is_err());
        let expr: RecExpr<CellLang> = "(MUX s a b)".parse().unwrap();
        assert!(CellLang::verify_expr(&expr).is_ok());
        let expr: RecExpr<CellLang> = "(AND s a b)".parse().unwrap();
        assert!(CellLang::verify_expr(&expr).is_err());
        let expr: RecExpr<CellLang> = "(AND a b)".parse().unwrap();
        assert!(CellLang::verify_expr(&expr).is_ok());
        assert!(matches!(expr.as_ref().last().unwrap(), CellLang::And(_)));
    }

    #[test]
    fn test_input_lists() {
        assert_eq!(
            PrimitiveType::AND4.get_input_list(),
            vec!["A1", "A2", "A3", "A4"]
        );
        assert_eq!(
            PrimitiveType::AOI22.get_input_list(),
            vec!["A1", "A2", "B1", "B2"]
        );

        assert_eq!(
            PrimitiveType::AOI211.get_input_list(),
            vec!["A", "B", "C1", "C2"]
        );

        assert_eq!(
            PrimitiveType::AOI221.get_input_list(),
            vec!["A", "B1", "B2", "C1", "C2"]
        );

        assert_eq!(PrimitiveType::XOR2.get_output(), "Z".to_string());

        // LUT input list is backwards relative to the IR
        assert_eq!(
            PrimitiveType::LUT5.get_input_list(),
            vec!["I4", "I3", "I2", "I1", "I0"]
        );

        assert_eq!(
            PrimitiveType::LUT6.get_input_list(),
            vec!["I5", "I4", "I3", "I2", "I1", "I0"]
        );

        assert_eq!(PrimitiveType::AOI222.get_num_inputs(), 6);

        assert_eq!(
            PrimitiveType::AOI222.get_input_list().len(),
            PrimitiveType::AOI222.get_num_inputs()
        );
    }

    #[test]
    fn test_emit_celllang() {
        let expr: RecExpr<CellLang> = "(NOR2_X1 b (NAND2_X1 a false))".parse().unwrap();
        let module = SVModule::from_cells(expr, "gate_list".to_string(), Vec::new());
        assert!(module.is_ok());
        let module = module.unwrap();
        let golden = "module gate_list (
    b,
    a,
    y
);
  input b;
  wire b;
  input a;
  wire a;
  output y;
  wire y;
  wire __0__;
  wire __1__;
  assign __0__ = 1'b0;
  NAND2_X1 #(
  ) __3__ (
      .A1(a),
      .A2(__0__),
      .ZN(__1__)
  );
  NOR2_X1 #(
  ) __4__ (
      .A1(b),
      .A2(__1__),
      .ZN(y)
  );
endmodule\n"
            .to_string();
        assert_eq!(module.to_string(), golden);
    }

    #[test]
    fn test_check_equiv() {
        let expr1: RecExpr<LutLang> = "(MUX s1 (MUX s0 a b) (MUX s0 c d))".parse().unwrap();
        let expr2: RecExpr<LutLang> = "(LUT 51952 s1 (LUT 61642 s1 s0 c d) a b)".parse().unwrap();
        let expr3: RecExpr<LutLang> = "(LUT 51952 s0 (LUT 61642 s0 s1 b d) a c)".parse().unwrap();
        assert!(LutLang::func_equiv(&expr1, &expr2).is_equiv());
        assert!(LutLang::func_equiv(&expr2, &expr3).is_equiv());
        let expr3: RecExpr<LutLang> = "(LUT 51952 s0 (LUT 61642 s1 s0 b d) a c)".parse().unwrap();
        assert!(LutLang::func_equiv(&expr2, &expr3).is_not_equiv());
        let expr3: RecExpr<LutLang> = "(LUT 51952 s0 (LUT 61643 s0 s1 b d) a c)".parse().unwrap();
        assert!(LutLang::func_equiv(&expr2, &expr3).is_not_equiv());
    }

    #[test]
    fn test_cell_area() {
        assert!(PrimitiveType::INV.get_min_area().is_some());
        assert_eq!(PrimitiveType::INV.get_min_area().unwrap(), 0.532);
        let expr: RecExpr<CellLang> = "(INV a)".parse().unwrap();
        let mut req: SynthRequest<CellLang, CellAnalysis> = SynthRequest::default()
            .with_expr(expr)
            .with_report()
            .with_rules(asic_rewrites())
            .without_progress_bar();
        let result = req
            .synth::<CellRpt>()
            .unwrap()
            .write_report_to_string()
            .unwrap();

        assert!(result.contains("area"));
        assert!(result.contains("\"area\": 1.064"));
    }
}
