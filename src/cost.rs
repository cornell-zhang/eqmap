/*!

  The code module contains a super simple cost function that extracts LUTs with at most `k` fan-in.

*/

use super::lut::LutLang;
use egg::{CostFunction, Id, Language};
#[cfg(feature = "exactness")]
use egg::{EGraph, LpCostFunction};
use std::collections::HashSet;

/// A cost function that extracts LUTs with at most `k` fan-in.
/// Gates have cost [u64::MAX] to prevent their extraction.
/// Registers have cost one.
pub struct KLUTCostFn {
    k: usize,
    reg_cost: u64,
}

impl KLUTCostFn {
    /// The default cost of a register
    pub const DEF_REG_COST: u64 = 1;

    /// Returns a new cost function with the given `k` value.
    /// Registers have a default weight of [Self::DEF_REG_COST].
    pub fn new(k: usize) -> Self {
        if k < 1 || k > LutLang::MAX_LUT_SIZE {
            panic!("k must be between 1 and {}", LutLang::MAX_LUT_SIZE);
        }
        Self {
            k,
            reg_cost: Self::DEF_REG_COST,
        }
    }

    /// Updates the cost of registers to `weight`
    pub fn with_reg_weight(self, weight: u64) -> Self {
        Self {
            reg_cost: weight,
            ..self
        }
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
                    1
                } else {
                    2 * l.len() as u64 * l.len() as u64
                }
            }
            LutLang::Program(_) => 0,
            LutLang::Bus(_) => 0,
            LutLang::Reg(_) => self.reg_cost,
            LutLang::Cycle(_) => 0,
            LutLang::Arg(_) => 0,
            LutLang::Const(_) => 0,
            LutLang::Var(_) => 1,
            LutLang::DC => 0,
            _ => u64::MAX,
        };
        enode.fold(op_cost, |sum, id| sum.saturating_add(costs(id)))
    }
}

#[cfg(feature = "exactness")]
impl LpCostFunction<LutLang, ()> for KLUTCostFn {
    fn node_cost(&mut self, _egraph: &EGraph<LutLang, ()>, _eclass: Id, enode: &LutLang) -> f64 {
        let op_cost: f64 = match enode {
            LutLang::Lut(l) => {
                if l.len() <= self.k + 1 {
                    1.0
                } else {
                    2.0 * l.len() as f64 * l.len() as f64
                }
            }
            LutLang::Program(_) => 0.0,
            LutLang::Bus(_) => 0.0,
            LutLang::Reg(_) => self.reg_cost as f64,
            LutLang::Cycle(_) => 0.0,
            LutLang::Arg(_) => 0.0,
            LutLang::Const(_) => 0.0,
            LutLang::Var(_) => 1.0,
            LutLang::DC => 0.0,
            _ => f64::MAX,
        };
        return op_cost;
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
            LutLang::Lut(l) => {
                if l.len() <= 2 {
                    0
                } else {
                    1
                }
            }
            LutLang::And(_) | LutLang::Mux(_) | LutLang::Nor(_) | LutLang::Xor(_) => 1,
            _ => 0,
        };
        let rt = enode.fold(0, |l, id| l.max(costs(id)));
        rt + op_cost
    }
}

/// This takes the negative of the cost function and returns a new cost function.
/// This will cause a RAM bomb whenever there is a cycle in the e-graph (which is often)
pub struct NegativeCostFn<C> {
    c: C,
}

impl<C> NegativeCostFn<C> {
    /// Returns a new cost function that takes the complement of the given cost function.
    pub fn new(c: C) -> Self {
        Self { c }
    }
}

impl<L, M> CostFunction<L> for NegativeCostFn<M>
where
    L: Language,
    M: CostFunction<L, Cost = i64>,
{
    type Cost = i64;
    fn cost<C>(&mut self, enode: &L, costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        -self.c.cost(enode, costs)
    }
}

/// This takes the negative of the cost function and returns a new cost function
pub struct ConjunctiveCostFn<A, B>
where
    A: CostFunction<LutLang>,
    B: CostFunction<LutLang>,
{
    a: A,
    b: B,
}

impl<A, B> ConjunctiveCostFn<A, B>
where
    A: CostFunction<LutLang>,
    B: CostFunction<LutLang>,
{
    /// Returns a new cost function that takes the product of the two given cost functions.
    pub fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<A, B> CostFunction<LutLang> for ConjunctiveCostFn<A, B>
where
    A: CostFunction<LutLang, Cost = i64>,
    B: CostFunction<LutLang, Cost = i64>,
{
    type Cost = i64;
    fn cost<C>(&mut self, enode: &LutLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let a = self.a.cost(enode, &mut costs);
        let b = self.b.cost(enode, &mut costs);
        a * b
    }
}

/// A cost function that attempts to extract only gates
pub struct GateCostFn {
    set: HashSet<String>,
}

impl GateCostFn {
    /// Returns a new cost function that extracts only the gates in `set`
    pub fn new(set: HashSet<String>) -> Self {
        Self { set }
    }
}

impl CostFunction<LutLang> for GateCostFn {
    type Cost = u64;
    fn cost<C>(&mut self, enode: &LutLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let op_cost = match enode {
            LutLang::Not(_) => {
                if self.set.contains("INV") || self.set.contains(&enode.get_prim_name().unwrap()) {
                    2
                } else {
                    u64::MAX
                }
            }
            LutLang::And(_) | LutLang::Nor(_) | LutLang::Xor(_) | LutLang::Mux(_) => {
                if self.set.contains(&enode.get_prim_name().unwrap()) {
                    4
                } else {
                    u64::MAX
                }
            }
            LutLang::Program(_) => 0,
            LutLang::Bus(_) => 0,
            LutLang::Reg(_) => 1,
            LutLang::Cycle(_) => 0,
            LutLang::Arg(_) => 0,
            LutLang::Const(_) => 0,
            LutLang::Var(_) => 1,
            LutLang::DC => 0,
            LutLang::Lut(l) => 10 * l.len() as u64 * l.len() as u64,
        };
        enode.fold(op_cost, |sum, id| sum.saturating_add(costs(id)))
    }
}
