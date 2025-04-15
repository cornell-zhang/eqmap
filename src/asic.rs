/*!

  Module for ASIC technology mapping.

*/

use super::check::Check;
use super::driver::Comparison;
use super::driver::Report;
use super::driver::{Canonical, CircuitLang, EquivCheck, Explanable, Extractable};
use egg::{
    Analysis, AstSize, CostFunction, DidMerge, EGraph, Id, Language, RecExpr, Rewrite, Symbol,
    define_language, rewrite,
};
use serde::Serialize;

define_language! {
    /// Definitions of e-node types. Programs are the only node type that is not a net/signal.
    #[allow(missing_docs)]
    pub enum CellLang {
        Const(bool),
        Var(Symbol),
        "AND" = And([Id; 2]),
        "OR" = Or([Id; 2]),
        "INV" = Inv([Id; 1]),
        Cell(Symbol, Vec<Id>), // or Box<[Id]>?
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
                Some((_, strength)) => {
                    if let Ok(strength) = strength.parse::<usize>() {
                        Some(strength)
                    } else {
                        None
                    }
                }
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
            CellLang::Const(_) => 1,
            CellLang::Var(_) => 2,
            CellLang::Cell(_, l) => {
                if l.len() > self.cut_size {
                    usize::MAX
                } else {
                    3
                }
            }
            _ => usize::MAX,
        };

        enode.fold(op_cost, |sum, id| sum.saturating_add(costs(id)))
    }
}

impl Extractable for CellLang {
    fn depth_cost_fn() -> impl CostFunction<Self, Cost = i64> {
        DepthCostFn
    }

    fn cell_cost_with_reg_weight_fn(cut_size: usize, _w: u64) -> impl CostFunction<Self> {
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
            CircuitStats {
                ast_size: input.len(),
            },
            CircuitStats {
                ast_size: output.len(),
            },
        ))
    }

    fn with_name(self, name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..self
        }
    }
}

/// Rewrite rules on CellLang
pub fn asic_rewrites() -> Vec<egg::Rewrite<CellLang, CellAnalysis>> {
    let mut rules: Vec<Rewrite<CellLang, CellAnalysis>> = Vec::new();

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
    rules.push(rewrite!("negation"; "?a" => "(INV (INV ?a))"));

    // Standard Cells

    rules.push(rewrite!("nor2_x1"; "(INV (OR ?a ?b))" => "(NOR2_X1 ?a ?b)"));
    rules.push(rewrite!("nand2_x1"; "(INV (AND ?a ?b))" => "(NAND2_X1 ?a ?b)"));
    rules.push(
        rewrite!("nand4_x1"; "(INV (AND (AND ?a ?b) (AND ?c ?d)))" => "(NAND4_X1 ?a ?b ?c ?d)"),
    );
    rules
        .push(rewrite!("xor2_x1"; "(OR (AND ?b (INV ?a)) (AND ?a (INV ?b)))" => "(XOR2_X1 ?a ?b)"));
    rules.push(rewrite!("inv_x1"; "(INV ?a)" => "(INV_X1 ?a)"));
    rules.push(rewrite!("aoi_x1"; "(INV (OR (AND ?a ?b) ?c))" => "(AOI_X1 ?a ?b ?c)"));
    rules.push(rewrite!("oai_x1"; "(INV (AND (OR ?a ?b) ?c))" => "(OAI_X1 ?a ?b ?c)"));

    rules
}
