/*!

  Module for ASIC technology mapping.

*/

use super::check::Check;
use super::driver::Comparison;
use super::driver::Report;
use super::driver::{Canonical, CircuitLang, EquivCheck, Explanable, Extractable};
use super::verilog::PrimitiveType;
#[cfg(feature = "exactness")]
use egg::LpCostFunction;
use egg::{
    Analysis, AstSize, CostFunction, DidMerge, EGraph, Id, Language, RecExpr, Rewrite, Symbol,
    define_language, rewrite,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::str::FromStr;

define_language! {
    /// Definitions of e-node types. Programs are the only node type that is not a net/signal.
    #[allow(missing_docs)]
    pub enum CellLang {
        Const(bool),
        Var(Symbol),
        "AND" = And([Id; 2]),
        "OR" = Or([Id; 2]),
        "INV" = Inv([Id; 1]),
        Cell(Symbol, Vec<Id>),
        "BUS" = Bus(Box<[Id]>),
    }
}

impl CellLang {
    /// Verify that an expression is well formed
    pub fn verify(&self) -> Result<(), String> {
        match self {
            CellLang::Var(s) | CellLang::Cell(s, _) => match s.as_str() {
                "AND" | "OR" | "INV" | "LUT" | "x" => {
                    Err(format!("Invalid cell/variable name: {}", s))
                }
                _ => {
                    if s.as_str().parse::<usize>().is_ok() {
                        Err(format!("Invalid cell/variable name: {}", s))
                    } else {
                        Ok(())
                    }
                }
            },
            _ => Ok(()),
        }
    }

    /// Get the drive strength of a cell
    pub fn get_drive_strength(&self) -> Option<usize> {
        match self {
            CellLang::Cell(s, _) => match s.as_str().split_once("_X") {
                Some((_, strength)) => strength.parse::<usize>().ok(),
                None => None,
            },
            _ => None,
        }
    }
}

/// A cost function that extracts a circuit with the least depth
pub struct DepthCostFn;

impl CostFunction<CellLang> for DepthCostFn {
    type Cost = i64;
    fn cost<C>(&mut self, enode: &CellLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let op_cost = match enode {
            CellLang::Const(_) => 0,
            CellLang::Var(_) => 0,
            CellLang::Cell(_, _) => 1,
            _ => i64::MAX,
        };
        let rt = enode.fold(0, |l, id| l.max(costs(id)));
        rt.saturating_add(op_cost)
    }
}

/// A cost function that extracts a circuit with the fewest mapped cells
pub struct CellCountFn {
    /// The cut size of the circuit
    cut_size: usize,
}

impl CellCountFn {
    /// Create a new `CellCountFn` that extracts cells with at most `cut_size` inputs
    pub fn new(cut_size: usize) -> Self {
        Self { cut_size }
    }
}

impl CostFunction<CellLang> for CellCountFn {
    type Cost = usize;
    fn cost<C>(&mut self, enode: &CellLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let op_cost = match enode {
            CellLang::Const(_) | CellLang::Bus(_) => 1,
            CellLang::Var(_) => 2,
            CellLang::Cell(n, l) => {
                if l.len() > self.cut_size {
                    usize::MAX
                } else if n.as_str().starts_with("N") || n.as_str().starts_with("INV") {
                    2
                } else {
                    3
                }
            }
            _ => usize::MAX,
        };

        enode.fold(op_cost, |sum, id| sum.saturating_add(costs(id)))
    }
}

#[cfg(feature = "exactness")]
impl<A> LpCostFunction<CellLang, A> for CellCountFn
where
    A: Analysis<CellLang>,
{
    fn node_cost(&mut self, _egraph: &EGraph<CellLang, A>, _eclass: Id, enode: &CellLang) -> f64 {
        let op_cost = match enode {
            CellLang::Const(_) => 1.0,
            CellLang::Var(_) => 2.0,
            CellLang::Cell(_, l) => {
                if l.len() > self.cut_size {
                    f64::MAX
                } else {
                    3.0
                }
            }
            _ => f64::MAX,
        };
        return op_cost;
    }
}

/// A cost function that extracts a circuit with the least area
pub struct AreaFn;

impl CostFunction<CellLang> for AreaFn {
    type Cost = f32;
    fn cost<C>(&mut self, enode: &CellLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let op_cost = match enode {
            CellLang::Const(_) | CellLang::Var(_) | CellLang::Bus(_) => {
                PrimitiveType::INV.get_min_area().unwrap()
            }
            CellLang::Cell(n, _l) => {
                let prim = PrimitiveType::from_str(n.as_str()).unwrap();
                prim.get_min_area().unwrap_or(1.33)
            }
            _ => f32::MAX,
        };

        enode.fold(op_cost, |sum, id| sum + costs(id))
    }
}

#[cfg(feature = "exactness")]
impl<A> LpCostFunction<CellLang, A> for AreaFn
where
    A: Analysis<CellLang>
{
    fn node_cost(&mut self, _egraph: &EGraph<CellLang, A>, _eclass: Id, enode: &CellLang) -> f64 {
        let op_cost: f64 = match enode {
            CellLang::Const(_) | CellLang::Var(_) => PrimitiveType::INV.get_min_area().unwrap() as f64,
            CellLang::Cell(n, _l) => {
                let prim = PrimitiveType::from_str(n.as_str()).unwrap();
                prim.get_min_area().unwrap_or(1.33) as f64
            }
            _ => f64::MAX,
        };
        return op_cost;
    }
}

impl Extractable for CellLang {
    fn depth_cost_fn() -> impl CostFunction<Self, Cost = i64> {
        DepthCostFn
    }

    fn cell_cost_with_reg_weight_fn(cut_size: usize, _w: u64) -> impl CostFunction<Self> {
        CellCountFn::new(cut_size)
    }

    fn exact_area_cost_fn() -> impl CostFunction<Self> {
        AreaFn
    }

    fn lp_exact_area_cost_fn<A: Analysis<Self>>() -> impl LpCostFunction<Self, A> {
        AreaFn
    }

    fn lp_cell_cost_with_reg_weight_fn<A: Analysis<Self>>(
        cut_size: usize,
        _w: u64,
    ) -> impl LpCostFunction<Self, A> {
        CellCountFn::new(cut_size)
    }

    fn filter_cost_fn(_set: std::collections::HashSet<String>) -> impl CostFunction<Self> {
        eprintln!("TODO: CellLang::filter_cost_fn");
        AstSize
    }
}

impl Canonical for CellLang {
    fn expr_is_canonical(_expr: &RecExpr<Self>) -> bool {
        true
    }

    fn canonicalize_expr(expr: RecExpr<Self>) -> RecExpr<Self> {
        expr
    }

    fn verify_expr(expr: &RecExpr<Self>) -> Result<(), String> {
        for c in expr {
            c.verify()?;
        }
        Ok(())
    }
}

impl EquivCheck for CellLang {
    fn check_expr(_expr: &RecExpr<Self>, _other: &RecExpr<Self>) -> Check {
        eprintln!("TODO: CellLang::check_expr");
        Check::Inconclusive
    }
}

impl Explanable for CellLang {
    fn get_explanations<A>(
        expr: &RecExpr<Self>,
        other: &RecExpr<Self>,
        runner: &mut egg::Runner<Self, A>,
    ) -> Result<Vec<egg::Explanation<Self>>, String>
    where
        A: egg::Analysis<Self>,
    {
        Ok(vec![runner.explain_equivalence(expr, other)])
    }
}

impl CircuitLang for CellLang {}

/// An empty analysis for CellLang
#[derive(Default, Clone)]
pub struct CellAnalysis;
impl Analysis<CellLang> for CellAnalysis {
    type Data = ();

    fn merge(&mut self, to: &mut Self::Data, from: Self::Data) -> DidMerge {
        egg::merge_max(to, from)
    }

    fn make(_egraph: &mut EGraph<CellLang, Self>, _enode: &CellLang) -> Self::Data {}
}

#[derive(Debug, Serialize)]
struct CircuitStats {
    /// AST size of the circuit
    ast_size: usize,
    // The total cell count
    cell_count: usize,
    /// Number of cells in the circuit
    cell_counts: BTreeMap<String, usize>,
    /// The area of the circuit
    area: f32,
}

impl CircuitStats {
    fn get_cell_counts(expr: &RecExpr<CellLang>) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for node in expr.iter() {
            if let CellLang::Cell(name, _) = node {
                *counts.entry(name.to_string()).or_insert(0) += 1;
            }
        }
        counts
    }

    fn get_area(expr: &RecExpr<CellLang>) -> f32 {
        expr.iter()
            .map(|n| {
                if let CellLang::Cell(name, _) = n {
                    PrimitiveType::from_str(name.as_str())
                        .unwrap()
                        .get_min_area()
                        .unwrap_or(1.33)
                } else if matches!(n, CellLang::Const(_) | CellLang::Var(_)) {
                    PrimitiveType::INV.get_min_area().unwrap()
                } else {
                    0.0
                }
            })
            .sum()
    }

    fn new(expr: &RecExpr<CellLang>) -> Self {
        let cell_counts = Self::get_cell_counts(expr);
        let cell_count = cell_counts.values().sum();
        Self {
            ast_size: expr.len(),
            cell_count,
            cell_counts,
            area: Self::get_area(expr),
        }
    }
}

/// An empty report struct for synthesizing CellLang
#[derive(Debug, Serialize)]
pub struct CellRpt {
    /// The name of the module
    name: String,
    /// Comparison of the original and mapped circuit
    stats: Comparison<CircuitStats>,
}

impl CellRpt {
    /// Create a new report
    fn new(name: String, before: CircuitStats, after: CircuitStats) -> Self {
        Self {
            name,
            stats: Comparison::new(before, after),
        }
    }
}

impl Report<CellLang> for CellRpt {
    fn new<A>(
        input: &RecExpr<CellLang>,
        output: &RecExpr<CellLang>,
        _extract_time: f64,
        _runner: &egg::Runner<CellLang, A>,
    ) -> Result<Self, String>
    where
        A: Analysis<CellLang>,
    {
        Ok(CellRpt::new(
            "top".to_string(),
            CircuitStats::new(input),
            CircuitStats::new(output),
        ))
    }

    fn with_name(self, name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..self
        }
    }
}

/// Returns true if the logic is fully mapped to cells
pub fn expr_is_mapped(expr: &RecExpr<CellLang>) -> bool {
    for n in expr {
        if matches!(n, CellLang::And(_) | CellLang::Or(_) | CellLang::Inv(_)) {
            return false;
        }
    }

    true
}

/// Return a list of forward and backwards rewrites that maps logic to their cells
/// The forwards rule is first in the array.
pub fn get_cell_logic_eqn<A>() -> Vec<[egg::Rewrite<CellLang, A>; 2]>
where
    A: Analysis<CellLang>,
{
    let mut rules: Vec<Vec<Rewrite<CellLang, A>>> = Vec::new();
    rules.push(rewrite!("and2_x1"; "(AND ?a ?b)" <=> "(AND2_X1 ?a ?b)"));
    rules.push(rewrite!("nand2_x1"; "(INV (AND ?a ?b))" <=> "(NAND2_X1 ?a ?b)"));
    rules.push(rewrite!("or2_x1"; "(OR ?a ?b)" <=> "(OR2_X1 ?a ?b)"));
    rules.push(rewrite!("nor2_x1"; "(INV (OR ?a ?b))" <=> "(NOR2_X1 ?a ?b)"));
    rules.push(
        rewrite!("xor2_x1"; "(OR (AND ?b (INV ?a)) (AND ?a (INV ?b)))" <=> "(XOR2_X1 ?a ?b)"),
    );
    rules.push(
        rewrite!("xnor2_x1"; "(OR (AND ?b ?a) (AND (INV ?a) (INV ?b)))" <=> "(XNOR2_X1 ?a ?b)"),
    );
    rules.push(rewrite!("and3_x1"; "(AND (AND ?a ?b) ?c)" <=> "(AND3_X1 ?a ?b ?c)"));
    rules.push(rewrite!("nand3_x1"; "(INV (AND (AND ?a ?b) ?c))" <=> "(NAND3_X1 ?a ?b ?c)"));
    rules.push(rewrite!("or3_x1"; "(OR (OR ?a ?b) ?c)" <=> "(OR3_X1 ?a ?b ?c)"));
    rules.push(rewrite!("nor3_x1"; "(INV (OR (OR ?a ?b) ?c))" <=> "(NOR3_X1 ?a ?b ?c)"));
    rules.push(rewrite!("and4_x1"; "(AND (AND ?a ?b) (AND ?c ?d))" <=> "(AND4_X1 ?a ?b ?c ?d)"));
    rules.push(
        rewrite!("nand4_x1"; "(INV (AND (AND ?a ?b) (AND ?c ?d)))" <=> "(NAND4_X1 ?a ?b ?c ?d)"),
    );
    rules.push(rewrite!("or4_x1"; "(OR (OR ?a ?b) (OR ?c ?d))" <=> "(OR4_X1 ?a ?b ?c ?d)"));
    rules.push(rewrite!("nor4_x1"; "(INV (OR (OR ?a ?b) (OR ?c ?d)))" <=> "(NOR4_X1 ?a ?b ?c ?d)"));
    rules.push(rewrite!("inv_x1"; "(INV ?a)" <=> "(INV_X1 ?a)"));
    rules.push(rewrite!("aoi21_x1"; "(INV (OR (AND ?b ?c) ?a))" <=> "(AOI21_X1 ?a ?b ?c)"));
    rules.push(rewrite!("oai21_x1"; "(INV (AND (OR ?b ?c) ?a))" <=> "(OAI21_X1 ?a ?b ?c)"));
    rules.push(
        rewrite!("aoi22_x1"; "(INV (OR (AND ?c ?d) (AND ?a ?b)))" <=> "(AOI22_X1 ?a ?b ?c ?d)"),
    );
    rules.push(
        rewrite!("oai22_x1"; "(INV (AND (OR ?c ?d) (OR ?a ?b)))" <=> "(OAI22_X1 ?a ?b ?c ?d)"),
    );
    rules.push(
        rewrite!("aoi211_x1"; "(INV (OR ?a (OR (AND ?c ?d) ?b)))" <=> "(AOI211_X1 ?a ?b ?c ?d)"),
    );
    rules.push(
        rewrite!("oai211_x1"; "(INV (AND ?a (AND (OR ?c ?d) ?b)))" <=> "(OAI211_X1 ?a ?b ?c ?d)"),
    );
    rules.push(
        rewrite!("aoi221_x1"; "(INV (OR (AND ?b ?c) (OR ?a (AND ?d ?e))))" <=> "(AOI221_X1 ?a ?b ?c ?d ?e)"),
    );
    rules.push(
        rewrite!("oai221_x1"; "(INV (AND (OR ?b ?c) (AND ?a (OR ?d ?e))))" <=> "(OAI221_X1 ?a ?b ?c ?d ?e)"),
    );
    rules.push(
        rewrite!("aoi222_x1"; "(INV (OR (AND ?e ?f) (OR (AND ?a ?b) (AND ?c ?d))))" <=> "(AOI222_X1 ?a ?b ?c ?d ?e ?f)"),
    );
    rules.push(
        rewrite!("oai222_x1"; "(INV (AND (OR ?e ?f) (AND (OR ?a ?b) (OR ?c ?d))))" <=> "(OAI222_X1 ?a ?b ?c ?d ?e ?f)"),
    );
    rules.push(rewrite!("mux2_x1"; "(OR (AND (INV ?s) ?b) (AND ?s ?a))" <=> "(MUX2_X1 ?s ?a ?b)"));

    rules
        .into_iter()
        .map(|mut v| {
            let bwd = v.pop().unwrap();
            let fwd = v.pop().unwrap();
            [fwd, bwd]
        })
        .collect()
}

/// Get the complete set of rewrites for a Boolean Algebra
pub fn get_boolean_algebra_rewrites<A>() -> Vec<egg::Rewrite<CellLang, A>>
where
    A: Analysis<CellLang>,
{
    let mut rules: Vec<Rewrite<CellLang, A>> = Vec::new();

    // Short circuit logic
    rules.push(rewrite!("or-true"; "(OR ?a true)" => "true"));
    rules.push(rewrite!("and-false"; "(AND ?a false)" => "false"));
    rules.push(rewrite!("or-false"; "(OR ?a false)" => "?a"));
    rules.push(rewrite!("and-true"; "(AND ?a true)" => "?a"));

    // Commutativity
    rules.push(rewrite!("or-commutative"; "(OR ?a ?b)" => "(OR ?b ?a)"));
    rules.push(rewrite!("and-commutative"; "(AND ?a ?b)" => "(AND ?b ?a)"));

    // Associativity
    rules.push(rewrite!("or-associative"; "(OR (OR ?a ?b) ?c)" => "(OR ?a (OR ?b ?c))"));
    rules.push(rewrite!("and-associative"; "(AND (AND ?a ?b) ?c)" => "(AND ?a (AND ?b ?c))"));

    // Distributivity
    rules.append(
        &mut rewrite!("or-distributive"; "(OR ?a (AND ?b ?c))" <=> "(AND (OR ?a ?b) (OR ?a ?c))"),
    );
    rules.append(
        &mut rewrite!("and-distributive"; "(AND ?a (OR ?b ?c))" <=> "(OR (AND ?a ?b) (AND ?a ?c))"),
    );

    // Complement Rules
    rules.push(rewrite!("or-complement"; "(OR ?a (INV ?a))" => "true"));
    rules.push(rewrite!("and-complement"; "(AND ?a (INV ?a))" => "false"));

    // De Morgan's Laws
    rules.append(&mut rewrite!("de-morgan-1"; "(INV (AND ?a ?b))" <=> "(OR (INV ?a) (INV ?b))"));
    rules.append(&mut rewrite!("de-morgan-2"; "(INV (OR ?a ?b))" <=> "(AND (INV ?a) (INV ?b))"));

    // Idempotent Laws
    rules.push(rewrite!("or-idempotent"; "(OR ?a ?a)" => "?a"));
    rules.push(rewrite!("and-idempotent"; "(AND ?a ?a)" => "?a"));

    // Absorption Laws
    rules.push(rewrite!("or-absorption"; "(OR ?a (AND ?a ?b))" => "?a"));
    rules.push(rewrite!("and-absorption"; "(AND ?a (OR ?a ?b))" => "?a"));

    // Consensus Rule
    rules.append(
        &mut rewrite!("consensus-rule"; "(OR (AND ?x ?y) (OR (AND (INV ?x) ?z) (AND ?y ?z)))" <=> "(OR (AND ?x ?y) (AND (INV ?x) ?z))"),
    );

    // Negation Rules
    rules.append(&mut rewrite!("negation"; "?a" <=> "(INV (INV ?a))"));
    rules
}

/// All the Boolean algebra and cell mapping rules for [CellLang].
pub fn asic_rewrites() -> Vec<egg::Rewrite<CellLang, CellAnalysis>> {
    let mut rules: Vec<Rewrite<CellLang, CellAnalysis>> = Vec::new();

    // Boolean Algebra
    rules.append(&mut get_boolean_algebra_rewrites());

    // Standard Cells
    get_cell_logic_eqn().into_iter().for_each(|r| {
        let [f, _] = r;
        rules.push(f)
    });

    rules
}
