/*!

  Simple cost functions that extracts LUTs with at most `k` fan-in.

*/
use super::analysis::LutAnalysis;
use super::asic::{CellAnalysis, CellLang};
use super::lut::LutLang;
use egg::{Analysis, CostFunction, Id, Language, LpCostFunction};
use std::collections::HashSet;

/// Folds over the deduplicated children of a node.
pub fn fold_deduped<L, F, T>(node: &L, init: T, mut f: F) -> T
where
    F: FnMut(T, Id) -> T,
    L: Language,
{
    let mut acc = init;
    let mut c = node.children().to_vec();
    c.dedup();
    for id in c {
        acc = f(acc, id)
    }
    acc
}

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

    /// Returns the cost of an e-node.
    pub fn op_cost(&self, enode: &LutLang) -> u64 {
        match enode {
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
        }
    }
}

impl LpCostFunction<LutLang, LutAnalysis> for KLUTCostFn {
    fn node_cost(
        &mut self,
        _egraph: &egg::EGraph<LutLang, LutAnalysis>,
        _eclass: Id,
        enode: &LutLang,
    ) -> f64 {
        if self.op_cost(enode) == u64::MAX {
            f64::INFINITY
        } else {
            self.op_cost(enode) as f64
        }
    }
}

impl CostFunction<LutLang> for KLUTCostFn {
    type Cost = u64;
    fn cost<C>(&mut self, enode: &LutLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        fold_deduped(enode, self.op_cost(enode), |sum, id| sum.saturating_add(costs(id)))
    }
}

/// A cost function that extracts a circuit with the least depth
pub struct DepthCostFn;

impl DepthCostFn {
    /// Returns the cost of an e-node.
    pub fn op_cost(&self, enode: &LutLang) -> i64 {
        match enode {
            LutLang::Lut(l) => {
                if l.len() <= 2 {
                    0
                } else {
                    1
                }
            }
            LutLang::And(_) | LutLang::Mux(_) | LutLang::Nor(_) | LutLang::Xor(_) => 1,
            _ => 0,
        }
    }
}

impl LpCostFunction<LutLang, LutAnalysis> for DepthCostFn {
    fn node_cost(
        &mut self,
        _egraph: &egg::EGraph<LutLang, LutAnalysis>,
        _eclass: Id,
        enode: &LutLang,
    ) -> f64 {
        self.op_cost(enode) as f64
    }
}

impl CostFunction<LutLang> for DepthCostFn {
    type Cost = i64;
    fn cost<C>(&mut self, enode: &LutLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let rt = fold_deduped(enode, 0, |l, id| l.max(costs(id)));
        rt + self.op_cost(enode)
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

impl<L, N, M> LpCostFunction<L, N> for NegativeCostFn<M>
where
    L: Language,
    N: Analysis<L>,
    M: LpCostFunction<L, N>,
{
    fn node_cost(&mut self, egraph: &egg::EGraph<L, N>, eclass: Id, enode: &L) -> f64 {
        -self.c.node_cost(egraph, eclass, enode)
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

    /// Returns the cost of a LutLang e-node.
    pub fn op_cost_lut(&self, enode: &LutLang) -> u64 {
        match enode {
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
            LutLang::Lut(_) => u64::MAX,
        }
    }

    /// Returns the cost of a CellLang e-node.
    pub fn op_cost_cell(&self, enode: &CellLang) -> u64 {
        match enode {
            CellLang::Inv(_) => {
                if self.set.contains("INV") {
                    1
                } else {
                    u64::MAX
                }
            }
            CellLang::And(_) => {
                if self.set.contains("AND") {
                    1
                } else {
                    u64::MAX
                }
            }
            CellLang::Or(_) => {
                if self.set.contains("OR") {
                    1
                } else {
                    u64::MAX
                }
            }
            CellLang::Var(_) | CellLang::Const(_) => 1,
            CellLang::Bus(_) => 0,
            CellLang::Cell(c, _) => {
                let pre = match c.as_str().split_once("_X") {
                    Some((p, _)) => p,
                    None => c.as_str(),
                };

                if self.set.contains(pre) { 1 } else { u64::MAX }
            }
        }
    }
}

impl CostFunction<LutLang> for GateCostFn {
    type Cost = u64;
    fn cost<C>(&mut self, enode: &LutLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        fold_deduped(enode, self.op_cost_lut(enode), |sum, id| sum.saturating_add(costs(id)))
    }
}

impl LpCostFunction<LutLang, LutAnalysis> for GateCostFn {
    fn node_cost(
        &mut self,
        _egraph: &egg::EGraph<LutLang, LutAnalysis>,
        _eclass: Id,
        enode: &LutLang,
    ) -> f64 {
        let node_cost = self.op_cost_lut(enode);
        if node_cost == u64::MAX {
            f64::INFINITY
        } else {
            node_cost as f64
        }
    }
}

impl CostFunction<CellLang> for GateCostFn {
    type Cost = u64;
    fn cost<C>(&mut self, enode: &CellLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
       fold_deduped(enode, self.op_cost_cell(enode), |sum, id| sum.saturating_add(costs(id)))
    }
}

impl LpCostFunction<CellLang, CellAnalysis> for GateCostFn {
    fn node_cost(
        &mut self,
        _egraph: &egg::EGraph<CellLang, CellAnalysis>,
        _eclass: Id,
        enode: &CellLang,
    ) -> f64 {
        let node_cost = self.op_cost_cell(enode);
        if node_cost == u64::MAX {
            f64::INFINITY
        } else {
            node_cost as f64
        }
    }
}
