/*!

  Common infrastructure to created command-line tools for logic synthesis using egg.

*/
use super::check::Check;
use super::cost::NegativeCostFn;
use super::lut::{CircuitStats, LutExprInfo, LutLang};
#[cfg(feature = "graph_dumps")]
use super::serialize::serialize_egraph;
use super::verilog::PrimitiveType;
use egg::{
    Analysis, BackoffScheduler, CostFunction, Explanation, Extractor, FromOpError, Language,
    LpCostFunction, RecExpr, RecExprParseError, Rewrite, Runner, StopReason, Symbol, TreeTerm,
};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;
use std::{
    io::{IsTerminal, Read, Write},
    path::PathBuf,
};

const MAX_CANON_SIZE: usize = 30000;

/// A trait to facilitate the generation of text-based reports on output ciruits.
/// For EqMap, this includes things like LUT count and circuit-depth.
pub trait Report<L: Language>
where
    Self: Serialize + Sized,
{
    /// Create a new [Report] comparing a pre-optimized `input` and a post-optimized `output`.
    fn new<A>(
        input: &RecExpr<L>,
        output: &RecExpr<L>,
        extract_time: f64,
        runner: &Runner<L, A>,
    ) -> Result<Self, String>
    where
        A: Analysis<L>;

    /// Rewrite the module name of the [Report].
    fn with_name(self, name: &str) -> Self;
}

/// A struct to compare two results before and after some optimization.
#[derive(Debug, Serialize)]
pub struct Comparison<T> {
    before: T,
    after: T,
}

impl<T> Comparison<T> {
    /// Create a new [Comparison] struct.
    pub fn new(before: T, after: T) -> Self {
        Self { before, after }
    }
}

/// The many stats associated with a synthesis run.
#[derive(Debug, Serialize)]
pub struct SynthReport {
    name: String,
    stop_reason: String,
    extract_time: f64,
    build_time: f64,
    input_size: u64,
    input_contains_gates: bool,
    num_inputs: u64,
    num_outputs: u64,
    num_classes: u64,
    num_nodes: u64,
    num_iterations: u64,
    saturated: bool,
    circuit_stats: Comparison<CircuitStats>,
}

impl SynthReport {
    /// Create a new [SynthReport] from a synthesized expression.
    pub fn new<A>(
        input: &RecExpr<LutLang>,
        extract_time: f64,
        runner: &Runner<LutLang, A>,
        output: &RecExpr<LutLang>,
    ) -> Self
    where
        A: Analysis<LutLang>,
    {
        let saturated = if let Some(reason) = &runner.stop_reason {
            matches!(reason, egg::StopReason::Saturated)
        } else {
            false
        };
        let rpt = runner.report();
        let num_classes = rpt.egraph_classes as u64;
        let num_nodes = rpt.egraph_nodes as u64;
        let build_time = rpt.total_time;
        let num_iterations = runner.report().iterations as u64;
        let input_info = LutExprInfo::new(input);
        let input_size = input_info.get_cse().as_ref().len() as u64;
        let input_contains_gates = input_info.contains_gates();
        let num_inputs = input_info.get_num_inputs();
        let num_outputs = input_info.get_num_outputs();
        let input_circuit_stats = input_info.get_circuit_stats();
        let output_info = LutExprInfo::new(output);
        let output_circuit_stats = output_info.get_circuit_stats();
        let circuit_stats = Comparison {
            before: input_circuit_stats,
            after: output_circuit_stats,
        };
        let stop_reason = match rpt.stop_reason {
            StopReason::Saturated => "Saturated".to_string(),
            StopReason::IterationLimit(n) => format!("{n} Iterations"),
            StopReason::NodeLimit(n) => format!("{n} Nodes"),
            StopReason::TimeLimit(n) => format!("{n} Seconds"),
            StopReason::Other(s) => s,
        };
        Self {
            name: "top".to_string(),
            stop_reason,
            extract_time,
            build_time,
            saturated,
            input_size,
            input_contains_gates,
            num_inputs,
            num_outputs,
            num_classes,
            num_nodes,
            num_iterations,
            circuit_stats,
        }
    }

    /// Mark whether the input originally contained gates or not.
    pub fn contains_gates(self, v: bool) -> Self {
        Self {
            input_contains_gates: self.input_contains_gates || v,
            ..self
        }
    }
}

impl Report<LutLang> for SynthReport {
    fn new<A>(
        input: &RecExpr<LutLang>,
        output: &RecExpr<LutLang>,
        extract_time: f64,
        runner: &Runner<LutLang, A>,
    ) -> Result<Self, String>
    where
        A: Analysis<LutLang>,
    {
        Ok(Self::new(input, extract_time, runner, output))
    }

    fn with_name(self, name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..self
        }
    }
}

/// The output of a [SynthRequest] run.
/// It optionally contains an explanation and a [Report].
pub struct SynthOutput<L, R>
where
    L: Language,
    R: Report<L>,
{
    expr: RecExpr<L>,
    expl: Option<Vec<Explanation<L>>>,
    rpt: Option<R>,
}

impl<L, R> SynthOutput<L, R>
where
    L: Language + egg::FromOp<Error = FromOpError> + std::fmt::Display,
    R: Report<L>,
{
    /// Create a new [SynthOutput] from a string.
    pub fn new(s: &str) -> Result<Self, RecExprParseError<FromOpError>> {
        let expr: RecExpr<L> = s.parse()?;
        Ok(Self {
            expr,
            expl: None,
            rpt: None,
        })
    }

    /// Get the expression of the output.
    pub fn get_expr(&self) -> &RecExpr<L> {
        &self.expr
    }

    /// Get the explanation for all output wires
    pub fn get_expl(&self) -> &Option<Vec<Explanation<L>>> {
        &self.expl
    }

    /// Get a compilation of proofs for all output wires
    pub fn get_proofs(&mut self) -> Option<String> {
        self.expl.as_mut().map(|e| {
            e.iter_mut().fold("".to_string(), |acc, x| {
                format!("{}\n{}", acc, x.get_flat_string())
            })
        })
    }

    fn get_rule_uses_rec(
        map: &mut BTreeMap<String, usize>,
        expl: &[std::rc::Rc<TreeTerm<L>>],
        visited: &mut std::collections::HashSet<*const TreeTerm<L>>,
    ) {
        for t in expl {
            let ptr = std::rc::Rc::as_ptr(t);
            if !visited.insert(ptr) {
                continue; // Already visited, skip to prevent infinite recursion
            }
            if let Some(r) = t.backward_rule {
                let name = r.to_string();
                let count = map.get(&name).unwrap_or(&0) + 1;
                map.insert(name, count);
            }

            if let Some(r) = t.forward_rule {
                let name = r.to_string();
                let count = map.get(&name).unwrap_or(&0) + 1;
                map.insert(name, count);
            }

            for c in &t.child_proofs {
                Self::get_rule_uses_rec(map, c, visited);
            }
        }
    }

    /// Get an accounting of rules used in the solution.
    pub fn get_rule_uses(&self) -> Option<String> {
        self.expl.as_ref()?;
        let mut map: BTreeMap<String, usize> = BTreeMap::new();
        let expl_list = self.expl.as_ref().unwrap();
        let mut visited = std::collections::HashSet::new();
        for expl in expl_list {
            Self::get_rule_uses_rec(&mut map, &expl.explanation_trees, &mut visited);
        }
        Some(
            map.iter()
                .fold("".to_string(), |acc, (k, v)| format!("{acc}\n\t{k}: {v}")),
        )
    }

    /// Check if the output has an explanation.
    pub fn has_explanation(&self) -> bool {
        self.expl.is_some()
    }

    /// Check if the output is empty.
    pub fn is_empty(&self) -> bool {
        self.expr.as_ref().is_empty()
    }

    /// Write the report of the output to a writer.
    pub fn write_report(&self, w: &mut impl Write) -> std::io::Result<()> {
        if let Some(rpt) = &self.rpt {
            serde_json::to_writer_pretty(w, rpt)?;
        }
        Ok(())
    }

    /// Write the report of the output to a string.
    pub fn write_report_to_string(&self) -> Result<String, std::io::Error> {
        match &self.rpt {
            Some(rpt) => Ok(serde_json::to_string_pretty(rpt)?),
            None => Ok(String::new()),
        }
    }

    /// Add a name to the synthesis report.
    pub fn with_name(self, name: &str) -> Self {
        if self.rpt.is_none() {
            return self;
        }
        Self {
            expr: self.expr,
            expl: self.expl,
            rpt: Some(self.rpt.unwrap().with_name(name)),
        }
    }
}

impl<L, R> std::fmt::Display for SynthOutput<L, R>
where
    L: Language + std::fmt::Display,
    R: Report<L>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.expr.as_ref().is_empty() {
            return Ok(());
        }
        write!(f, "{}", self.expr)
    }
}

/// Update a progress bar with the current state of the runner.
fn report_progress<L, A>(
    runner: &Runner<L, A>,
    interrupt: &Arc<AtomicBool>,
    iter_bar: Option<&mut ProgressBar>,
    node_bar: Option<&mut ProgressBar>,
) -> Result<(), String>
where
    L: Language,
    A: Analysis<L> + std::default::Default,
{
    if interrupt.load(Ordering::Acquire) {
        return Err("SIGINT Received".to_string());
    }

    if let Some(b) = iter_bar {
        b.inc(1);
    }
    if let Some(b) = node_bar {
        let nodes = runner.egraph.total_number_of_nodes();
        b.set_position(nodes as u64);
    }
    Ok(())
}

/// An enum for the e-graph build strategy
#[derive(Debug, Clone)]
enum BuildStrat {
    /// Build the e-graph with a time limit
    TimeLimited(u64),
    /// Build the e-graph with a node limit
    SizeLimited(usize),
    /// Build the e-graph with a rewrite iteration limit
    IterLimited(usize),
    /// Build the e-graph with a combination of limits (time, nodes, iterations)
    Custom(u64, usize, usize),
}

/// An enum for the optimization strategies used to synthesize LUT/cell networks.
#[derive(Debug, Clone)]
enum OptStrat {
    /// Extract the circuit by the syntax of the expression
    AstSize,
    /// Extract the cirucit using *real* cell areas.
    Area,
    /// Extract maximum circuit depth (RAM bomb).
    MaxDepth,
    /// Extract minimum circuit depth.
    MinDepth,
    /// Extract Cells/LUTs with at most `k` inputs.
    CellCount(usize),
    /// Extract Cells/LUTs with at most `k` inputs as well as registers with cost ratio `w`.
    CellCountRegWeighted(usize, u64),
    /// Disassemble into set of logic gates.
    Disassemble(HashSet<String>),
}

/// An enum for the extraction strategies used to synthesize LUT/cell networks.
#[derive(Debug, Clone)]
enum ExtractStrat {
    /// Use greedy extraction algorithm.
    Greedy,
    /// Use exact HiGHS ILP extraction with timeout in seconds.
    #[cfg(feature = "exact_highs")]
    Highs(u64),
    /// Use exact CBC ILP extraction with timeout in seconds.
    #[cfg(feature = "exact_cbc")]
    Cbc(u64),
}

/// An enum for the rewrite scheduling properties.
#[derive(Debug, Clone)]
enum RewriteStrat {
    /// Rewrite with flat ban every 960 matches.
    Boolean,
    /// Rewrite with exponential ban every 16 matches.
    Algebraic,
}

impl RewriteStrat {
    /// Apply the rewrite strategy to a runner.
    fn apply<L, A>(&self, runner: Runner<L, A>) -> Runner<L, A>
    where
        L: Language,
        A: Analysis<L> + std::default::Default,
    {
        let bos = BackoffScheduler::default();

        // Use back-off scheduling on runner to avoid some rules starving others
        let bos = match self {
            RewriteStrat::Boolean => bos.with_ban_length(1).with_initial_match_limit(960),
            RewriteStrat::Algebraic => bos.with_ban_length(1).with_initial_match_limit(960),
        };

        runner.with_scheduler(bos)
    }
}

/// The list of gates that must be reachable by the disassembling rewrite rule system.
pub const GATE_WHITELIST_STR: &str = "MUX,AND,OR,XOR,NOT,INV,FDRE,NAND,NOR";

impl OptStrat {
    /// Create an extraction strategy from a comma-separated list of gates.
    /// For example, `list` can be `"MUX,AND,NOT"`.
    pub fn from_gate_set(list: &str) -> Result<Self, String> {
        if list.is_empty() || list == "all" {
            return Self::from_gate_set(GATE_WHITELIST_STR);
        }

        // list is a comma-deliminted string
        let gates: HashSet<String> = list.split(',').map(|s| s.to_string()).collect();
        for gate in &gates {
            if PrimitiveType::from_str(gate).is_err() {
                return Err(format!("Gate {gate} is not a valid cell type"));
            }
        }
        Ok(OptStrat::Disassemble(gates))
    }
}

/// A trait to represent that a language has some form of equivalence-checking in the form of a [Check]
pub trait EquivCheck
where
    Self: Sized,
{
    /// Check if `expr` and `other` are equivalent.
    fn check_expr(expr: &RecExpr<Self>, other: &RecExpr<Self>) -> Check;
}

/// A trait to represent that a language can be canonicalized.
pub trait Canonical
where
    Self: Sized,
{
    /// Returns true if the expression is canonical.
    fn expr_is_canonical(expr: &RecExpr<Self>) -> bool;

    /// Returns a canonicalization of the expression.
    fn canonicalize_expr(expr: RecExpr<Self>) -> RecExpr<Self>;

    /// Verify that the expression does not have any extra syntax errors.
    fn verify_expr(expr: &RecExpr<Self>) -> Result<(), String>;
}

/// A trait to represent that a language is extractable under the [SynthRequest] optimization goals.
/// In short, three main cost functions are needed to cover all extraction strategies:
/// one for depth, one for cell count, and one for filtering/disassembly.
pub trait Extractable
where
    Self: Language,
{
    /// Returns the depth cost function for the language.
    fn depth_cost_fn() -> impl CostFunction<Self, Cost = i64>;

    /// Returns the area cost function for the language, only selecting cells with fewer than `cut_size` inputs.
    /// Additionally, registers have a parameterized weight `w`.
    fn cell_cost_with_reg_weight_fn(cut_size: usize, w: u64) -> impl CostFunction<Self>;

    /// Returns the area cost function for the language, only selecting cells with fewer than `cut_size` inputs.
    /// In this case, registers have weight 1.
    fn cell_cost_fn(cut_size: usize) -> impl CostFunction<Self> {
        Self::cell_cost_with_reg_weight_fn(cut_size, 1)
    }

    /// Returns the cost function using *real* cell areas.
    fn exact_area_cost_fn() -> impl CostFunction<Self>;

    /// Returns a cost function used for extracting only certain types nodes.
    fn filter_cost_fn(set: HashSet<String>) -> impl CostFunction<Self>;
}

/// A trait to represent a language that is extractable for ILP-based extraction.
pub trait LpExtractable<N>
where
    Self: Language,
    N: Analysis<Self>,
{
    /// Returns the depth cost function for the language.
    fn lp_depth_cost_fn() -> impl LpCostFunction<Self, N>;

    /// Returns the area cost function for the language, only selecting cells with fewer than `cut_size` inputs.
    /// Additionally, registers have a parameterized weight `w`.
    fn lp_cell_cost_with_reg_weight_fn(cut_size: usize, w: u64) -> impl LpCostFunction<Self, N>;

    /// Returns the area cost function for the language, only selecting cells with fewer than `cut_size` inputs.
    /// In this case, registers have weight 1.
    fn lp_cell_cost_fn(cut_size: usize) -> impl LpCostFunction<Self, N> {
        Self::lp_cell_cost_with_reg_weight_fn(cut_size, 1)
    }

    /// Returns the cost function using exact cell areas.
    fn lp_exact_area_cost_fn() -> impl LpCostFunction<Self, N>;

    /// Returns a cost function used for extracting only certain types nodes.
    fn lp_filter_cost_fn(set: HashSet<String>) -> impl LpCostFunction<Self, N>;
}

/// A trait to represent that an expression is not best explained by relating its roots.
/// As an example, explanations of [LutLang::Bus] are not useful.
pub trait Explanable
where
    Self: Language,
{
    /// Get a list of explanations for the relevant sub-terms in `expr` and `other` using the egraph in `runnner`.
    fn get_explanations<A>(
        expr: &RecExpr<Self>,
        other: &RecExpr<Self>,
        runner: &mut Runner<Self, A>,
    ) -> Result<Vec<Explanation<Self>>, String>
    where
        A: Analysis<Self>;
}

/// An alias to all the traits needed to be implemented in order to use the [SynthRequest] API.
pub trait CircuitLang:
    Language
    + Explanable
    + Extractable
    + Canonical
    + EquivCheck
    + std::fmt::Display
    + egg::FromOp<Error = FromOpError>
{
    /// Returns a fresh variable with name `sym`.
    fn var(sym: Symbol) -> Self;

    /// Capture multiple expressions in a single node
    fn bus(ids: impl Iterator<Item = egg::Id>) -> Self;

    /// Returns true is the node is a bus
    fn is_bus(&self) -> bool;

    /// Returns the symbol of the node, if is a variable
    fn get_var(&self) -> Option<Symbol>;
}

type PurgeFn<L> = Arc<dyn Fn(&L) -> bool + 'static>;

/// A request to explore and extract an expression.
/// The request can be configured with various options
/// before dedicating to a particular input and compilation strategy.
pub struct SynthRequest<L, A>
where
    L: Language,
    A: Analysis<L>,
{
    /// The expression to simplify.
    expr: RecExpr<L>,

    /// The rewrite rules used to simplify the expression.
    rules: Vec<Rewrite<L, A>>,

    /// The optimization strategy to use.
    opt_strat: OptStrat,

    /// The extraction strategy to use.
    extract_strat: ExtractStrat,

    /// The e-graph build strategy to use.
    build_strat: BuildStrat,

    /// The rewrite scheduling strategy to use.
    rewrite_strat: RewriteStrat,

    /// If true, do not canonicalize the input expression.
    no_canonicalize: bool,

    /// If true, an error is returned if saturation is not met.
    assert_sat: bool,

    /// If true, a proof of the simplification should be generation. If false, no proof will be
    /// generated.
    gen_proof: bool,

    /// If true, a progress bar will be displayed, else no progress bar will be displayed.
    prog_bar: bool,

    /// Produce a report which records extra stats.
    produce_rpt: bool,

    /// Produce a condensed JSON dump of the e-graph
    #[cfg(feature = "graph_dumps")]
    dump_egraph: Option<PathBuf>,

    /// The maximum number of nodes to canonicalize
    max_canon_size: usize,

    /// Whether the expression was non-trivially canonicalized before exploration.
    canonicalized: bool,

    /// A function to purge the e-graph with before extraction.
    purge_fn: Option<PurgeFn<L>>,

    /// The running result
    result: Option<Runner<L, A>>,
}

impl<L: Language, A: Analysis<L>> std::default::Default for SynthRequest<L, A> {
    fn default() -> Self {
        Self {
            expr: RecExpr::default(),
            rules: Vec::new(),
            opt_strat: OptStrat::CellCount(6),
            extract_strat: ExtractStrat::Greedy,
            build_strat: BuildStrat::Custom(10, 20_000, 16),
            rewrite_strat: RewriteStrat::Boolean,
            no_canonicalize: false,
            assert_sat: false,
            gen_proof: false,
            prog_bar: true,
            produce_rpt: false,
            max_canon_size: MAX_CANON_SIZE,
            canonicalized: false,
            purge_fn: None,
            result: None,
            #[cfg(feature = "graph_dumps")]
            dump_egraph: None,
        }
    }
}

impl<L: Language, A: Analysis<L> + std::clone::Clone> std::clone::Clone for SynthRequest<L, A> {
    fn clone(&self) -> Self {
        Self {
            expr: self.expr.clone(),
            rules: self.rules.clone(),
            opt_strat: self.opt_strat.clone(),
            extract_strat: self.extract_strat.clone(),
            build_strat: self.build_strat.clone(),
            rewrite_strat: self.rewrite_strat.clone(),
            no_canonicalize: self.no_canonicalize,
            assert_sat: self.assert_sat,
            gen_proof: self.gen_proof,
            prog_bar: self.prog_bar,
            produce_rpt: self.produce_rpt,
            max_canon_size: self.max_canon_size,
            canonicalized: self.canonicalized,
            purge_fn: self.purge_fn.clone(),
            result: None,
            #[cfg(feature = "graph_dumps")]
            dump_egraph: self.dump_egraph.clone(),
        }
    }
}

/// Purge the e-graph of nodes that satisfy the predicate `f`.
/// Also, delete self-loops.
fn purge_graph<L, A, F: Fn(&L) -> bool>(egraph: &mut egg::EGraph<L, A>, f: F) -> Result<(), String>
where
    L: Language,
    A: Analysis<L>,
{
    for class in egraph.classes_mut() {
        let new_nodes: Vec<L> = class
            .nodes
            .iter()
            .filter(|n| !f(n) && !n.children().contains(&class.id))
            .cloned()
            .collect();
        if !new_nodes.is_empty() {
            class.nodes = new_nodes;
        }
    }
    Ok(())
}

impl<L, A> SynthRequest<L, A>
where
    L: CircuitLang + LpExtractable<A>,
    A: Analysis<L> + Default,
{
    /// Request greedy extraction of cells/LUTs with at most `k` inputs.
    pub fn with_k(self, k: usize) -> Self {
        Self {
            opt_strat: OptStrat::CellCount(k),
            extract_strat: ExtractStrat::Greedy,
            ..self
        }
    }

    /// Request greedy extraction of cells/LUTs with at most `k` inputs and registers with weight `w`.
    pub fn with_klut_regw(self, k: usize, w: u64) -> Self {
        Self {
            opt_strat: OptStrat::CellCountRegWeighted(k, w),
            extract_strat: ExtractStrat::Greedy,
            ..self
        }
    }

    /// Request greedy extraction using *real* cell areas.
    pub fn with_area(self) -> Self {
        Self {
            opt_strat: OptStrat::Area,
            extract_strat: ExtractStrat::Greedy,
            ..self
        }
    }

    /// Request greedy extraction by syntax complexity.
    pub fn with_ast_size(self) -> Self {
        Self {
            opt_strat: OptStrat::AstSize,
            extract_strat: ExtractStrat::Greedy,
            ..self
        }
    }

    /// Request exact extraction using CBC to solve ILP with `timeout` in seconds.
    #[cfg(feature = "exact_cbc")]
    pub fn with_cbc(self, timeout: u64) -> Self {
        Self {
            extract_strat: ExtractStrat::Cbc(timeout),
            ..self
        }
    }

    /// Request exact extraction using HiGHS to solve ILP with `timeout` in seconds.
    #[cfg(feature = "exact_highs")]
    pub fn with_highs(self, timeout: u64) -> Self {
        Self {
            extract_strat: ExtractStrat::Highs(timeout),
            ..self
        }
    }

    /// Extract based on minimum circuit depth.
    pub fn with_min_depth(self) -> Self {
        Self {
            opt_strat: OptStrat::MinDepth,
            extract_strat: ExtractStrat::Greedy,
            ..self
        }
    }

    /// Extract based on maximum circuit depth. *Does not work with cycles in e-graph.*
    pub fn with_max_depth(self) -> Self {
        Self {
            opt_strat: OptStrat::MaxDepth,
            extract_strat: ExtractStrat::Greedy,
            ..self
        }
    }

    /// Extract by disassembling into basic logic gates. The exact list can be found at [GATE_WHITELIST_STR].
    pub fn with_disassembler(self) -> Self {
        Self {
            opt_strat: OptStrat::from_gate_set(GATE_WHITELIST_STR).unwrap(),
            extract_strat: ExtractStrat::Greedy,
            ..self
        }
    }

    /// Extract by disassembling into the logic gates in `list`. Elements in the list will be matched against [PrimitiveType::from_str].
    pub fn with_disassembly_into(self, list: &str) -> Result<Self, String> {
        Ok(Self {
            opt_strat: OptStrat::from_gate_set(list)?,
            extract_strat: ExtractStrat::Greedy,
            ..self
        })
    }

    /// Build request limited by `iter_limit` rewrite iterations.
    pub fn iter_limited(self, iter_limit: usize) -> Self {
        Self {
            build_strat: BuildStrat::IterLimited(iter_limit),
            result: None,
            ..self
        }
    }

    /// Build request limited by `node_limit` nodes.
    pub fn node_limited(self, node_limit: usize) -> Self {
        Self {
            build_strat: BuildStrat::SizeLimited(node_limit),
            result: None,
            ..self
        }
    }

    /// Build request limited by `timeout` seconds.
    pub fn time_limited(self, timeout: u64) -> Self {
        Self {
            build_strat: BuildStrat::TimeLimited(timeout),
            result: None,
            ..self
        }
    }

    /// Build request with joint limits. If any of the three limits are hit, building is stopped.
    pub fn with_joint_limits(self, timeout: u64, node_limit: usize, iter_limit: usize) -> Self {
        Self {
            build_strat: BuildStrat::Custom(timeout, node_limit, iter_limit),
            result: None,
            ..self
        }
    }

    /// Collect additional stats with e-graph build.
    pub fn with_report(self) -> Self {
        Self {
            produce_rpt: true,
            result: None,
            ..self
        }
    }

    /// Schedule rewrites with Boolean strategy.
    pub fn with_boolean_scheduler(self) -> Self {
        Self {
            rewrite_strat: RewriteStrat::Boolean,
            result: None,
            ..self
        }
    }

    /// Schedule rewrites with Algebraic strategy.
    pub fn with_algebraic_scheduler(self) -> Self {
        Self {
            rewrite_strat: RewriteStrat::Algebraic,
            result: None,
            ..self
        }
    }

    /// Collect additional stats with e-graph build.
    #[cfg(feature = "graph_dumps")]
    pub fn with_graph_dump(self, p: PathBuf) -> Self {
        Self {
            dump_egraph: Some(p),
            result: None,
            ..self
        }
    }

    /// Run exploration with explanations enabled.
    pub fn with_proof(self) -> Self {
        Self {
            gen_proof: true,
            result: None,
            ..self
        }
    }

    /// Run exploration with saturation asserts enabled.
    pub fn with_asserts(self) -> Self {
        Self {
            assert_sat: true,
            result: None,
            ..self
        }
    }

    /// Do not run canonicalization on the input before exploration.
    pub fn without_canonicalization(self) -> Self {
        if self.canonicalized && self.result.is_some() {
            panic!("Cannot uncanonicalize after exploration");
        }

        Self {
            no_canonicalize: true,
            result: None,
            ..self
        }
    }

    /// Do not display a progress bar during exploration.
    pub fn without_progress_bar(self) -> Self {
        Self {
            prog_bar: false,
            ..self
        }
    }

    /// Run exploration with expression `expr`.
    pub fn with_expr(self, expr: RecExpr<L>) -> Self {
        Self {
            expr,
            result: None,
            ..self
        }
    }

    /// Run exploration with `more_rules` added on.
    pub fn with_rules(mut self, mut more_rules: Vec<Rewrite<L, A>>) -> Self {
        self.rules.append(&mut more_rules);
        self.result = None;
        self
    }

    /// Clear the rules currently stored in the request.
    pub fn clear_rules(self) -> Self {
        Self {
            rules: Vec::new(),
            result: None,
            ..self
        }
    }

    /// Purge nodes that satisfy the predicate `f` from the e-graph before extraction.
    pub fn with_purge_fn<F>(self, f: F) -> Self
    where
        F: Fn(&L) -> bool + 'static,
    {
        Self {
            purge_fn: Some(Arc::new(f)),
            result: None,
            ..self
        }
    }

    /// Return a reference to the underlying expression
    pub fn get_expr(&self) -> &RecExpr<L> {
        &self.expr
    }

    fn explore(&mut self) -> Result<(), String> {
        let runner = if self.gen_proof {
            eprintln!("WARNING: Proof generation is on (slow)");
            Runner::default().with_explanations_enabled()
        } else {
            Runner::default().with_explanations_disabled()
        };

        // Print a progress bar to get a sense of growth
        let mp = MultiProgress::new();

        let runner = self.rewrite_strat.apply(runner);

        let runner = match self.build_strat {
            BuildStrat::TimeLimited(t) => runner
                .with_time_limit(Duration::from_secs(t))
                .with_node_limit(usize::MAX)
                .with_iter_limit(usize::MAX),
            BuildStrat::SizeLimited(n) => runner
                .with_node_limit(n)
                .with_time_limit(Duration::from_secs(31536000))
                .with_iter_limit(usize::MAX),
            BuildStrat::IterLimited(n) => runner
                .with_iter_limit(n)
                .with_time_limit(Duration::from_secs(31536000))
                .with_node_limit(usize::MAX),
            BuildStrat::Custom(t, n, i) => runner
                .with_time_limit(Duration::from_secs(t))
                .with_node_limit(n)
                .with_iter_limit(i),
        };

        let interrupt = Arc::new(AtomicBool::new(false));
        let interrupt_clone = Arc::clone(&interrupt);
        let _ = ctrlc::set_handler(move || {
            if interrupt_clone.load(Ordering::Acquire) {
                std::process::exit(130);
            }

            interrupt_clone.store(true, Ordering::Release);
        });
        let runner = if self.prog_bar {
            let mut iter_bar = match self.build_strat {
                BuildStrat::IterLimited(i) | BuildStrat::Custom(_, _, i) => {
                    let b = mp.add(ProgressBar::new(i as u64).with_message("iterations"));
                    b.set_style(
                        ProgressStyle::with_template("[{bar:60.cyan/blue}] {pos}/{len} iterations")
                            .unwrap(),
                    );
                    Some(b)
                }
                _ => None,
            };
            let mut node_bar = match self.build_strat {
                BuildStrat::SizeLimited(n) | BuildStrat::Custom(_, n, _) => {
                    let b = mp.add(ProgressBar::new(n as u64));
                    b.set_style(
                        ProgressStyle::with_template("[{bar:60.magenta}] {pos}/{len} nodes")
                            .unwrap(),
                    );
                    Some(b)
                }
                _ => None,
            };
            runner.with_hook(move |r| {
                report_progress(r, &interrupt, iter_bar.as_mut(), node_bar.as_mut())
            })
        } else {
            runner.with_hook(move |r| report_progress(r, &interrupt, None, None))
        };

        // Make a time bar
        let time_bar = match (self.prog_bar, self.build_strat.clone()) {
            (true, BuildStrat::TimeLimited(t)) | (true, BuildStrat::Custom(t, _, _)) => {
                // This is a Copilot special right here...
                let stop_signal = Arc::new(AtomicBool::new(false));
                let stop_signal_clone = Arc::clone(&stop_signal);
                let b = Arc::new(mp.add(ProgressBar::new(t * 2)));
                b.set_style(
                    ProgressStyle::with_template("[{bar:60.green}] {spinner:.green} {elapsed}")
                        .unwrap(),
                );
                b.tick();

                let pb_clone = Arc::clone(&b);
                std::thread::spawn(move || {
                    for _ in 0..(t * 2 - 1) {
                        std::thread::sleep(Duration::from_millis(500));
                        if stop_signal_clone.load(Ordering::Acquire) {
                            break;
                        }
                        pb_clone.inc(1);
                    }
                });
                Some((stop_signal, b))
            }
            _ => None,
        };

        let oexp = self.expr.clone();
        if self.expr.as_ref().len() > self.max_canon_size {
            eprintln!(
                "WARNING: Input is too large to canonicalize ({} nodes)",
                self.expr.as_ref().len()
            );
        } else if !self.no_canonicalize {
            self.canonicalized = !L::expr_is_canonical(&oexp);
            self.expr = L::canonicalize_expr(self.expr.clone());
        }

        if self.gen_proof && L::check_expr(&self.expr, &oexp).is_not_equiv() {
            return Err(format!(
                "Folding the initial expression had an error: {}",
                self.expr
            ));
        }

        self.result = Some(runner.with_expr(&self.expr).run(&self.rules));

        // Clear the progress bar
        if let Some(t) = time_bar {
            t.0.store(true, Ordering::Release);
            t.1.finish_and_clear();
        }
        mp.clear().unwrap();

        if self.gen_proof {
            let runner = self.result.as_ref().unwrap();
            let croot = runner.egraph.find(runner.roots[0]);
            let data = &runner.egraph[croot].data;
            eprintln!("INFO: Root analysis");
            eprintln!("INFO: =============");
            eprintln!("INFO:\t{data:?}");
        }

        Ok(())
    }

    /// Extract requested expression with `extractor`
    pub fn extract_with<R, F>(&mut self, extractor: F) -> Result<SynthOutput<L, R>, String>
    where
        R: Report<L>,
        F: FnOnce(&egg::EGraph<L, A>, egg::Id) -> RecExpr<L>,
    {
        if self.result.is_none() {
            self.explore()?;
        }

        if let Some(f) = self.purge_fn.take() {
            eprintln!("INFO: Purging e-graph...");
            purge_graph(&mut self.result.as_mut().unwrap().egraph, f.as_ref())?;
        }

        let runner = self.result.as_ref().unwrap();

        // We need to canonicalize the root ID
        let root = runner.egraph.find(runner.roots[0]);
        if self.gen_proof {
            let report = runner.report();
            eprintln!("INFO: {}", report.to_string().replace('\n', "\nINFO: "));
        }

        // use an Extractor to pick the best element of the root eclass
        eprintln!("INFO: Extracting...");
        let extraction_start = Instant::now();
        let best = extractor(&runner.egraph, root);
        let extraction_time = extraction_start.elapsed();
        if self.gen_proof {
            eprintln!(
                "INFO: Extraction time: {} seconds",
                extraction_time.as_secs_f64()
            );
        }

        let stop_reason = runner.stop_reason.as_ref().unwrap().clone();
        if self.assert_sat && !matches!(stop_reason, egg::StopReason::Saturated) {
            return Err(format!(
                "Expression {} failed to saturate. Grown to {} nodes with reason {:?}",
                self.expr,
                runner.egraph.total_number_of_nodes(),
                stop_reason
            ));
        } else {
            eprintln!(
                "INFO: Grown to {} nodes with reason {:?}",
                runner.egraph.total_number_of_nodes(),
                stop_reason
            );
        }

        let rpt = if self.produce_rpt {
            eprintln!("INFO: Generating report...");
            Some(R::new(
                &self.expr,
                &best,
                extraction_time.as_secs_f64(),
                runner,
            )?)
        } else {
            None
        };

        let expl = if self.gen_proof {
            self.get_explanations(&best)?.into()
        } else {
            None
        };

        Ok(SynthOutput {
            expr: best,
            expl,
            rpt,
        })
    }

    /// Extract greedily requested expression with cost model `c`
    pub fn greedy_extract_with<R, C>(&mut self, c: C) -> Result<SynthOutput<L, R>, String>
    where
        R: Report<L>,
        C: CostFunction<L>,
    {
        self.extract_with(|egraph, root| {
            let e = Extractor::new(egraph, c);
            e.find_best(root).1
        })
    }

    /// Serialize the e-graph with an associated cost provided by `c`.
    #[cfg(feature = "graph_dumps")]
    pub fn serialize_with_greedy_cost<C>(&mut self, c: C, w: &mut impl Write) -> std::io::Result<()>
    where
        C: CostFunction<L>,
        <C as CostFunction<L>>::Cost: Serialize + std::default::Default,
    {
        if self.result.is_none() {
            self.explore()
                .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;
        }
        let runner = self.result.as_ref().unwrap();
        serialize_egraph(&runner.egraph, &runner.roots, c, w)
    }

    /// Get individual proofs for each wire at the root of `best`.
    fn get_explanations(&mut self, best: &RecExpr<L>) -> Result<Vec<Explanation<L>>, String> {
        if !self.gen_proof {
            return Err("Proof generation was not enabled".to_string());
        }

        if self.result.is_none() {
            self.explore()?;
        }

        let runner = self.result.as_mut().unwrap();
        let root_expr = &self.expr;

        L::get_explanations(root_expr, best, runner)
    }

    /// Synthesize with the extraction strategy set in `self`.
    pub fn synth<R>(&mut self) -> Result<SynthOutput<L, R>, String>
    where
        R: Report<L>,
    {
        match (self.opt_strat.to_owned(), self.extract_strat.to_owned()) {
            (OptStrat::AstSize, ExtractStrat::Greedy) => self.greedy_extract_with(egg::AstSize),
            (OptStrat::Area, ExtractStrat::Greedy) => {
                self.greedy_extract_with(L::exact_area_cost_fn())
            }
            (OptStrat::MinDepth, ExtractStrat::Greedy) => {
                self.greedy_extract_with(L::depth_cost_fn())
            }
            (OptStrat::MaxDepth, ExtractStrat::Greedy) => {
                eprintln!("WARNING: Maximizing cost on e-graphs with cycles will crash.");
                self.greedy_extract_with(NegativeCostFn::new(L::depth_cost_fn()))
            }
            (OptStrat::CellCount(k), ExtractStrat::Greedy) => {
                self.greedy_extract_with(L::cell_cost_fn(k))
            }
            (OptStrat::CellCountRegWeighted(k, w), ExtractStrat::Greedy) => {
                self.greedy_extract_with(L::cell_cost_with_reg_weight_fn(k, w))
            }
            (OptStrat::Disassemble(set), ExtractStrat::Greedy) => {
                self.greedy_extract_with(L::filter_cost_fn(set))
            }
            #[cfg(feature = "exact_cbc")]
            (OptStrat::AstSize, ExtractStrat::Cbc(t)) => self.extract_with(|egraph, root| {
                eprintln!("INFO: CBC ILP ON");
                let mut e = egg::LpExtractor::new(egraph, egg::AstSize);
                L::canonicalize_expr(e.solve_with_timeout(root, good_lp::coin_cbc, t as f64))
            }),
            #[cfg(feature = "exact_cbc")]
            (OptStrat::Area, ExtractStrat::Cbc(t)) => self.extract_with(|egraph, root| {
                eprintln!("INFO: CBC ILP ON");
                let mut e = egg::LpExtractor::new(egraph, L::lp_exact_area_cost_fn());
                L::canonicalize_expr(e.solve_with_timeout(root, good_lp::coin_cbc, t as f64))
            }),
            #[cfg(feature = "exact_cbc")]
            (OptStrat::MinDepth, ExtractStrat::Cbc(t)) => self.extract_with(|egraph, root| {
                eprintln!("INFO: CBC ILP ON");
                let mut e = egg::LpExtractor::new(egraph, L::lp_depth_cost_fn());
                L::canonicalize_expr(e.solve_with_timeout(root, good_lp::coin_cbc, t as f64))
            }),
            #[cfg(feature = "exact_cbc")]
            (OptStrat::MaxDepth, ExtractStrat::Cbc(t)) => self.extract_with(|egraph, root| {
                eprintln!("WARNING: Maximizing cost on e-graphs with cycles will crash.");
                eprintln!("INFO: CBC ILP ON");
                let mut e =
                    egg::LpExtractor::new(egraph, NegativeCostFn::new(L::lp_depth_cost_fn()));
                L::canonicalize_expr(e.solve_with_timeout(root, good_lp::coin_cbc, t as f64))
            }),
            #[cfg(feature = "exact_cbc")]
            (OptStrat::CellCount(k), ExtractStrat::Cbc(t)) => self.extract_with(|egraph, root| {
                eprintln!("INFO: CBC ILP ON");
                let mut e = egg::LpExtractor::new(egraph, L::lp_cell_cost_fn(k));
                L::canonicalize_expr(e.solve_with_timeout(root, good_lp::coin_cbc, t as f64))
            }),
            #[cfg(feature = "exact_cbc")]
            (OptStrat::CellCountRegWeighted(k, w), ExtractStrat::Cbc(t)) => {
                self.extract_with(|egraph, root| {
                    eprintln!("INFO: CBC ILP ON");
                    let mut e =
                        egg::LpExtractor::new(egraph, L::lp_cell_cost_with_reg_weight_fn(k, w));
                    L::canonicalize_expr(e.solve_with_timeout(root, good_lp::coin_cbc, t as f64))
                })
            }
            #[cfg(feature = "exact_cbc")]
            (OptStrat::Disassemble(set), ExtractStrat::Cbc(t)) => {
                self.extract_with(|egraph, root| {
                    eprintln!("INFO: CBC ILP ON");
                    let mut e = egg::LpExtractor::new(egraph, L::lp_filter_cost_fn(set));
                    L::canonicalize_expr(e.solve_with_timeout(root, good_lp::coin_cbc, t as f64))
                })
            }
            #[cfg(feature = "exact_highs")]
            (OptStrat::AstSize, ExtractStrat::Highs(t)) => self.extract_with(|egraph, root| {
                eprintln!("INFO: HiGHS ILP ON");
                let mut e = egg::LpExtractor::new(egraph, egg::AstSize);
                L::canonicalize_expr(e.solve_with_timeout(root, good_lp::highs, t as f64))
            }),
            #[cfg(feature = "exact_highs")]
            (OptStrat::Area, ExtractStrat::Highs(t)) => self.extract_with(|egraph, root| {
                eprintln!("INFO: HiGHS ILP ON");
                let mut e = egg::LpExtractor::new(egraph, L::lp_exact_area_cost_fn());
                L::canonicalize_expr(e.solve_with_timeout(root, good_lp::highs, t as f64))
            }),
            #[cfg(feature = "exact_highs")]
            (OptStrat::MinDepth, ExtractStrat::Highs(t)) => self.extract_with(|egraph, root| {
                eprintln!("INFO: HiGHS ILP ON");
                let mut e = egg::LpExtractor::new(egraph, L::lp_depth_cost_fn());
                L::canonicalize_expr(e.solve_with_timeout(root, good_lp::highs, t as f64))
            }),
            #[cfg(feature = "exact_highs")]
            (OptStrat::MaxDepth, ExtractStrat::Highs(t)) => self.extract_with(|egraph, root| {
                eprintln!("WARNING: Maximizing cost on e-graphs with cycles will crash.");
                eprintln!("INFO: HiGHS ILP ON");
                let mut e =
                    egg::LpExtractor::new(egraph, NegativeCostFn::new(L::lp_depth_cost_fn()));
                L::canonicalize_expr(e.solve_with_timeout(root, good_lp::highs, t as f64))
            }),
            #[cfg(feature = "exact_highs")]
            (OptStrat::CellCount(k), ExtractStrat::Highs(t)) => {
                self.extract_with(|egraph, root| {
                    eprintln!("INFO: HiGHS ILP ON");
                    let mut e = egg::LpExtractor::new(egraph, L::lp_cell_cost_fn(k));
                    L::canonicalize_expr(e.solve_with_timeout(root, good_lp::highs, t as f64))
                })
            }
            #[cfg(feature = "exact_highs")]
            (OptStrat::CellCountRegWeighted(k, w), ExtractStrat::Highs(t)) => {
                self.extract_with(|egraph, root| {
                    eprintln!("INFO: HiGHS ILP ON");
                    let mut e =
                        egg::LpExtractor::new(egraph, L::lp_cell_cost_with_reg_weight_fn(k, w));
                    L::canonicalize_expr(e.solve_with_timeout(root, good_lp::highs, t as f64))
                })
            }
            #[cfg(feature = "exact_highs")]
            (OptStrat::Disassemble(set), ExtractStrat::Highs(t)) => {
                self.extract_with(|egraph, root| {
                    eprintln!("INFO: HiGHS ILP ON");
                    let mut e = egg::LpExtractor::new(egraph, L::lp_filter_cost_fn(set));
                    L::canonicalize_expr(e.solve_with_timeout(root, good_lp::highs, t as f64))
                })
            }
            #[cfg(any(feature = "exact_cbc", feature = "exact_highs"))]
            _ => Err(format!(
                "{:?} optimization strategy is incomptabile with {:?} extraction.",
                self.opt_strat, self.extract_strat
            )),
        }
    }
}

/// A simple function to read from file, stdin, or a string. The `cmd` takes the most precendence, then files, then stdin.
pub fn simple_reader(cmd: Option<String>, input_file: Option<PathBuf>) -> std::io::Result<String> {
    let mut buf = String::new();
    if let Some(cmd) = cmd {
        buf = cmd;
    } else if input_file.is_some() {
        std::fs::File::open(input_file.unwrap())?.read_to_string(&mut buf)?;
    } else {
        let mut stdin = std::io::stdin();
        if stdin.is_terminal() {
            print!("> ");
            std::io::stdout().flush()?;
            while stdin.read_line(&mut buf)? <= 2 {
                print!("> ");
                std::io::stdout().flush()?;
            }
        } else {
            stdin.read_to_string(&mut buf)?;
        }
    }

    Ok(buf)
}

/// Compile a [CircuitLang] expression using a baseline request `req`.
/// The output expression is returned as a [SynthOutput]. Everything else goes to stderr.
pub fn process_expression<L, A, R>(
    expr: RecExpr<L>,
    req: SynthRequest<L, A>,
    no_verify: bool,
    verbose: bool,
) -> std::io::Result<SynthOutput<L, R>>
where
    L: CircuitLang + LpExtractable<A>,
    A: Analysis<L> + Clone + Default,
    R: Report<L>,
{
    if !no_verify {
        L::verify_expr(&expr).map_err(std::io::Error::other)?;
    }

    if cfg!(debug_assertions) {
        eprintln!("WARNING: Running with debug assertions is slow");
    }

    let mut req = req.with_expr(expr.clone());

    let result = req.synth().map_err(std::io::Error::other)?;

    #[cfg(feature = "graph_dumps")]
    if let Some(p) = &req.dump_egraph {
        eprintln!("INFO: Dumping e-graph...");
        let mut file = std::fs::File::create(p)?;
        req.serialize_with_greedy_cost(L::depth_cost_fn(), &mut file)?;
    }

    if verbose && result.has_explanation() {
        eprintln!("INFO: Rule uses in proof");
        eprintln!("INFO: =============");
        let proof = result.get_rule_uses().unwrap();
        let mut linecount = 0;
        for line in proof.lines() {
            eprintln!("INFO:\t{line}");
            linecount += 1;
        }
        eprintln!("INFO: Approx. {linecount} lines in proof tree");
    } else if req.get_expr().as_ref().len() < 240 {
        let expr = req.get_expr().to_string();
        let len = expr.len().min(240);
        eprintln!("INFO: {} ... => ", &expr[0..len]);
    }
    let simplified = result.get_expr();
    // Verify functionality
    if no_verify {
        eprintln!("INFO: Skipping functionality tests...");
    } else {
        let check = L::check_expr(&expr, simplified);
        if check.is_inconclusive() && verbose {
            eprintln!("WARNING: Functionality verification inconclusive");
        }
        if check.is_not_equiv() {
            match result.get_expl() {
                Some(e) => {
                    eprintln!("ERROR: Exhaustive testing failed. Dumping explanations...");
                    for expl in e {
                        eprintln!("{expl}");
                    }
                }
                None => eprintln!(
                    "ERROR: Failed for unknown reason. Try running with --verbose for an attempted proof"
                ),
            }
            return Err(std::io::Error::other("Functionality verification failed"));
        }
    }
    Ok(result)
}

/// Compile a [CircuitLang] expression from a line of text using a baseline request `req`.
/// The output expression is returned as a [SynthOutput]. Everything else goes to stderr.
pub fn process_string_expression<L, A, R>(
    line: &str,
    req: SynthRequest<L, A>,
    no_verify: bool,
    verbose: bool,
) -> std::io::Result<SynthOutput<L, R>>
where
    L: CircuitLang + LpExtractable<A>,
    <L as egg::FromOp>::Error: serde::ser::StdError + Sync + Send + 'static,
    A: Analysis<L> + Clone + Default,
    R: Report<L>,
{
    let line = line.trim();
    if line.starts_with("//") || line.is_empty() {
        return Ok(SynthOutput {
            expr: RecExpr::default(),
            expl: None,
            rpt: None,
        });
    }
    let expr = line.split("//").next().unwrap();
    let expr: RecExpr<L> = expr.parse().map_err(std::io::Error::other)?;

    process_expression(expr, req, no_verify, verbose)
}
