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
}
