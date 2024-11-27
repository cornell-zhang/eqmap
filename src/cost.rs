/*!

  The code module contains a super simple cost function that extracts LUTs with at most `k` fan-in.

*/
use super::lut::LutLang;
use egg::{CostFunction, Id, Language};

/// A cost function that extracts LUTs with at most `k` fan-in.
/// Gates have cost [u64::MAX] to prevent their extraction.
/// Registers have cost zero.
pub struct KLUTCostFn {
    k: usize,
}

impl KLUTCostFn {
    /// Returns a new cost function with the given `k` value.
    pub fn new(k: usize) -> Self {
        if k < 1 || k > LutLang::MAX_LUT_SIZE {
            panic!("k must be between 1 and {}", LutLang::MAX_LUT_SIZE);
        }
        Self { k }
    }
}

impl CostFunction<LutLang> for KLUTCostFn {
    type Cost = u64;
    fn cost<C>(&mut self, enode: &LutLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let op_cost = match enode {
            LutLang::Lut(l) => {
                if l.len() <= self.k + 1 {
                    2 * l.len() as u64
                } else {
                    2 * l.len() as u64 * l.len() as u64
                }
            }
            LutLang::Program(_) => 0,
            LutLang::Bus(_) => 0,
            LutLang::Reg(_) => 1,
            LutLang::Const(_) => 0,
            LutLang::Var(_) => 1,
            LutLang::DC => 0,
            _ => u64::MAX,
        };
        enode.fold(op_cost, |sum, id| {
            if costs(id) > u64::MAX - sum {
                u64::MAX
            } else {
                sum + costs(id)
            }
        })
    }
}

/// A cost function that extracts a circuit with the least depth
pub struct DepthCostFn;

impl CostFunction<LutLang> for DepthCostFn {
    type Cost = i64;
    fn cost<C>(&mut self, enode: &LutLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let op_cost = match enode {
            LutLang::Lut(_)
            | LutLang::And(_)
            | LutLang::Mux(_)
            | LutLang::Nor(_)
            | LutLang::Not(_)
            | LutLang::Xor(_) => 1,
            _ => 0,
        };
        let rt = enode.fold(0, |l, id| l.max(costs(id)));
        rt + op_cost
    }
}

/// This takes the negative of the cost function and returns a new cost function
pub struct NegativeCostFn<C>
where
    C: CostFunction<LutLang>,
{
    c: C,
}

impl<C> NegativeCostFn<C>
where
    C: CostFunction<LutLang>,
{
    /// Returns a new cost function that takes the complement of the given cost function.
    pub fn new(c: C) -> Self {
        Self { c }
    }
}

impl<M> CostFunction<LutLang> for NegativeCostFn<M>
where
    M: CostFunction<LutLang, Cost = i64>,
{
    type Cost = i64;
    fn cost<C>(&mut self, enode: &LutLang, costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        -self.c.cost(enode, costs)
    }
}
