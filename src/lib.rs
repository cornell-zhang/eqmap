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
    use egg::RecExpr;

    use super::*;

    #[test]
    fn test_swap() {
        // Need to be able to represent 3
        assert_eq!(lut::from_bitvec(&lut::to_bitvec(3, 2)), 3);
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
}
