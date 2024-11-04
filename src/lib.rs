#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
/*!

`lut-synth`: LUT Network Synthesis with E-Graphs

TODO: overview, tutorial, testing, research papers

*/

pub mod analysis;
pub mod cost;
pub mod lut;
pub mod parse;
pub mod rewrite;

#[cfg(test)]
mod tests {
    use analysis::LutAnalysis;
    use egg::{Analysis, Language, RecExpr};
    use lut::{node_dominates, verify_expr, LutLang};
    use parse::{sv_parse_wrapper, SVModule, SVPrimitive};

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
        assert_eq!(2, lut::get_lut_count(&make_simple_nested_lut()));
        assert_eq!(1, lut::get_lut_count(&make_four_lut()));
        assert_eq!(1, lut::get_lut_count(&make_three_lut()));
    }

    #[test]
    fn test_get_lut_k_count() {
        assert_eq!(2, lut::get_lut_count_k(&make_simple_nested_lut(), 4));
        assert_eq!(0, lut::get_lut_count_k(&make_simple_nested_lut(), 3));
        assert_eq!(1, lut::get_lut_count_k(&make_four_lut(), 4));
        assert_eq!(0, lut::get_lut_count_k(&make_four_lut(), 6));
        assert_eq!(1, lut::get_lut_count_k(&make_three_lut(), 3));
        assert_eq!(0, lut::get_lut_count_k(&make_three_lut(), 6));
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
        assert_eq!(instance.attributes.len(), 2);
        assert_eq!(instance.attributes["program"], "17361601744336890538");
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
        assert!(LutLang::func_equiv_always(&bus, &swapped));
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
        assert!(LutLang::func_equiv_always(
            &xor,
            &"(LUT 6 a b)".parse().unwrap()
        ));
        assert!(!LutLang::func_equiv_always(
            &xor,
            &"(LUT 4 a b)".parse().unwrap()
        ));
        assert!(!LutLang::func_equiv_always(
            &xor,
            &"(LUT 2 a b)".parse().unwrap()
        ));
        assert!(LutLang::func_equiv_always(
            &nor,
            &"(LUT 1 a b)".parse().unwrap()
        ));
        assert!(!LutLang::func_equiv_always(&xor, &nor));
    }

    #[test]
    fn test_dominance() {
        let bus: RecExpr<LutLang> =
            "(BUS (XOR (XOR a b) cin) (NOT (NOR (AND a b) (AND cin (XOR a b)))) (XOR a b))"
                .parse()
                .unwrap();
        let bus_node = bus.as_ref().last().unwrap();
        let sum = bus_node.children()[0];
        let carry = bus_node.children()[1];
        let xor = bus_node.children()[2];
        assert!(!node_dominates(&bus, sum, carry).unwrap());
        assert!(!node_dominates(&bus, carry, sum).unwrap());
        assert!(node_dominates(&bus, sum, sum).unwrap());
        assert!(node_dominates(&bus, carry, carry).unwrap());
        assert!(node_dominates(&bus, xor, carry).unwrap());
        assert!(node_dominates(&bus, xor, sum).unwrap());
    }
}
