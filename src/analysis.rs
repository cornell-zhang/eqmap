/*!

  This module defines the analysis stored with signals in the LUT network.
  In most respects, the analysis enforces some level of consistency when rewrite rules are applied.
  For example, we should not be merging programs with non-programs.
  This analysis can also assist in constant propagation and pruning nodes.
  There is more work to be done in [LutAnalysis::modify] to handle pruning.

*/

use super::lut;
use egg::{Analysis, DidMerge};

/// An e-class is typically a boolean signal.
/// However, we store constants and input aliases for folding.
/// A [lut::LutLang::Program] should never really be rewritten, so storing programs allow us to quickly check if a class is a program and extract the program.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LutAnalysisData {
    /// If a class is a Program(u64), it should be by itself
    program: Option<u64>,
    /// Is Some(bool) when the class is equivalent to a constant `true` or `false`
    const_val: Option<bool>,
    /// Eventually, this should be a vector so we can store aliases
    input: Option<String>,
    /// The bus size of the node (if it is a bus)
    size: Option<usize>,
}

impl LutAnalysisData {
    /// Create a new LutAnalysisData struct
    pub fn new(
        program: Option<u64>,
        const_val: Option<bool>,
        input: Option<String>,
        size: Option<usize>,
    ) -> Self {
        Self {
            program,
            const_val,
            input,
            size,
        }
    }

    /// Extract the LUT program in this class. If it is an input or gate, throw an error
    pub fn get_program(&self) -> Result<u64, String> {
        match self.program {
            Some(p) => Ok(p),
            None => Err("No program found".to_string()),
        }
    }

    /// Return the constant associated with this class. If it is not a constant, throw an error.
    pub fn get_as_const(&self) -> Result<bool, String> {
        match self.const_val {
            Some(v) => Ok(v),
            None => Err("No constant value found".to_string()),
        }
    }

    /// Returns true if the class is an input
    pub fn is_an_input(&self) -> bool {
        self.input.is_some()
    }
}

/// The analysis struct allows for discovering when signals are equivalent to constants or leaf inputs.
/// Additonally, the struct assists in folding constant inputs to smaller LUTs.
#[derive(Default)]
pub struct LutAnalysis;
impl Analysis<lut::LutLang> for LutAnalysis {
    type Data = LutAnalysisData;
    fn merge(&mut self, to: &mut Self::Data, from: Self::Data) -> DidMerge {
        if to.program != from.program {
            panic!("Tried to merge two different programs");
        }
        if to.size != from.size {
            panic!("Tried to merge two conflicting bus sizes");
        }
        if !(to.const_val == from.const_val || to.const_val.is_none() || from.const_val.is_none()) {
            // Later we will want to relax this condition for constant folding.
            // For now, it is a good sanity check.
            panic!("Cannot merge constant type with non-constant type.");
        }
        if !(to.input == from.input || to.input.is_none() || from.input.is_none()) {
            // Later, we will want to relax this condition when we're okay with input aliasing.
            panic!("Cannot merge input type with non-input type.");
        }
        let mut merged = to.clone();
        merged.const_val = from.const_val.or(to.const_val);
        merged.input = from.input.clone().or(to.input.clone());
        let merged_to = merged != *to;
        *to = merged;
        DidMerge(merged_to, *to != from)
    }
    fn make(_egraph: &egg::EGraph<lut::LutLang, Self>, enode: &lut::LutLang) -> Self::Data {
        match enode {
            lut::LutLang::Program(p) => LutAnalysisData::new(Some(*p), None, None, None),
            lut::LutLang::Const(c) => LutAnalysisData::new(None, Some(*c), None, None),
            lut::LutLang::Var(v) => LutAnalysisData::new(None, None, Some(v.to_string()), None),
            lut::LutLang::Bus(b) => LutAnalysisData::new(None, None, None, Some(b.len())),
            _ => LutAnalysisData::default(),
        }
    }
    fn modify(egraph: &mut egg::EGraph<lut::LutLang, Self>, id: egg::Id) {
        // Evaluate constant input at the msb
        let nodes = egraph[id].nodes.clone();
        for node in nodes {
            if let lut::LutLang::Lut(_) = node {
                let operands = node.get_operand_classes(egraph).expect("Expected operands");
                let msb_const = egraph[operands[0]].data.get_as_const();
                if msb_const.is_ok() {
                    let program = node
                        .get_program_in_egraph(egraph)
                        .expect("Expected program");
                    if operands.len() > 1 {
                        let mod_program = lut::eval_lut_const_input(
                            &program,
                            operands.len() - 1,
                            msb_const.unwrap(),
                        );
                        let pi = egraph.add(lut::LutLang::Program(mod_program));
                        let mut c = operands.clone();
                        c[0] = pi;
                        let repl = egraph.add(lut::LutLang::Lut(c.into()));
                        egraph.union(id, repl);
                    } else {
                        let const_val = msb_const.unwrap();
                        match program & 3 {
                            0 => {
                                let repl = egraph.add(lut::LutLang::Const(false));
                                egraph.union(id, repl);
                            }
                            3 => {
                                let repl = egraph.add(lut::LutLang::Const(true));
                                egraph.union(id, repl);
                            }
                            2 => {
                                let repl = egraph.add(lut::LutLang::Const(const_val));
                                egraph.union(id, repl);
                            }
                            1 => {
                                let repl = egraph.add(lut::LutLang::Const(!const_val));
                                egraph.union(id, repl);
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
        }
    }
}
