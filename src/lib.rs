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
    use egg::{Analysis, RecExpr};
    use lut::{verify_expr, LutLang};

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
}
