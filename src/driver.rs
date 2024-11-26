/*!

  The code module common infrastructure to created command-line tools for logic synthesis.

*/
use std::time::{Duration, Instant};

use super::cost::{DepthCostFn, KLUTCostFn};
use super::lut::{canonicalize_expr, verify_expr, LutExprInfo, LutLang};
use egg::{
    Analysis, BackoffScheduler, CostFunction, Explanation, Extractor, FromOpError, Language,
    RecExpr, RecExprParseError, Rewrite, Runner,
};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use serde::Serialize;
use std::{
    io::{IsTerminal, Read, Write},
    path::PathBuf,
};

const MAX_CANON_SIZE: usize = 30000;

/// The stats associated with a synthesis run.
#[derive(Debug, Serialize)]
pub struct SynthReport {
    extract_time: f64,
}

/// The output of a [SynthRequest] run.
pub struct SynthOutput {
    expr: RecExpr<LutLang>,
    expl: Option<Explanation<LutLang>>,
    rpt: Option<SynthReport>,
}

impl SynthOutput {
    /// Create a new [SynthOutput] from a string.
    pub fn new(s: &str) -> Result<Self, RecExprParseError<FromOpError>> {
        let expr: RecExpr<LutLang> = s.parse()?;
        Ok(Self {
            expr,
            expl: None,
            rpt: None,
        })
    }

    /// Get the expression of the output.
    pub fn get_expr(&self) -> &RecExpr<LutLang> {
        &self.expr
    }

    /// Get the explanation of the output.
    pub fn get_expl(&self) -> &Option<Explanation<LutLang>> {
        &self.expl
    }

    /// Get the proof of the output.
    pub fn get_proof(&mut self) -> String {
        if self.has_explanation() {
            self.expl.as_mut().unwrap().get_flat_string()
        } else {
            String::new()
        }
    }

    /// Get the analysis for an output expression.
    pub fn get_analysis(&self) -> LutExprInfo {
        LutExprInfo::new(&self.expr)
    }

    /// Check if the output has an explanation.
    pub fn has_explanation(&self) -> bool {
        self.expl.is_some()
    }

    /// Check if the output is empty.
    pub fn is_empty(&self) -> bool {
        self.expr.as_ref().is_empty()
    }
}

impl std::fmt::Display for SynthOutput {
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
    iter_bar: &mut ProgressBar,
    node_bar: &mut ProgressBar,
) -> Result<(), String>
where
    L: Language,
    A: Analysis<L> + std::default::Default,
{
    iter_bar.inc(1);
    let nodes = runner.egraph.total_number_of_nodes();
    node_bar.set_position(nodes as u64);
    Ok(())
}

#[derive(Debug, Clone)]
enum ExtractStrat {
    Depth,
    CountLUT(usize),
}

/// A request to simplify an expression.
pub struct SynthRequest<A>
where
    A: Analysis<LutLang>,
{
    /// The expression to simplify.
    expr: RecExpr<LutLang>,

    /// The rewrite rules used to simplify the expression.
    rules: Vec<Rewrite<LutLang, A>>,

    /// The extraction strategy to use.
    extract_strat: ExtractStrat,

    /// If true, do not canonicalize the input expression.
    no_canonicalize: bool,

    /// If true, an error is returned if saturation is not met.
    assert_sat: bool,

    /// If true, a proof of the simplification should be generation. If false, no proof will be
    /// generated.
    gen_proof: bool,

    /// If true, a progress bar will be displayed, else no progress bar will be displayed.
    prog_bar: bool,

    /// The timelimit, in seconds, given to the search.
    timeout: u64,

    /// Produce a report which records extra stats.
    produce_rpt: bool,

    /// The maximum number of enodes in a searched egraph.
    node_limit: usize,

    /// The maximum number of iterations of rewrites applied to the egraph.
    iter_limit: usize,

    /// The maximum number of nodes to canonicalize
    max_canon_size: usize,

    /// The running result
    result: Option<Runner<LutLang, A>>,
}

impl<A: Analysis<LutLang>> std::default::Default for SynthRequest<A> {
    fn default() -> Self {
        Self {
            expr: RecExpr::default(),
            rules: Vec::new(),
            extract_strat: ExtractStrat::CountLUT(6),
            no_canonicalize: false,
            assert_sat: false,
            gen_proof: false,
            prog_bar: true,
            timeout: 10,
            produce_rpt: false,
            node_limit: 20_000,
            iter_limit: 16,
            max_canon_size: MAX_CANON_SIZE,
            result: None,
        }
    }
}

impl<A: Analysis<LutLang> + std::clone::Clone> std::clone::Clone for SynthRequest<A> {
    fn clone(&self) -> Self {
        Self {
            expr: self.expr.clone(),
            rules: self.rules.clone(),
            extract_strat: self.extract_strat.clone(),
            no_canonicalize: self.no_canonicalize,
            assert_sat: self.assert_sat,
            gen_proof: self.gen_proof,
            prog_bar: self.prog_bar,
            timeout: self.timeout,
            produce_rpt: self.produce_rpt,
            node_limit: self.node_limit,
            iter_limit: self.iter_limit,
            max_canon_size: self.max_canon_size,
            result: None,
        }
    }
}

impl<A> SynthRequest<A>
where
    A: Analysis<LutLang>,
{
    /// Request with extracting LUTs up to size `k`.
    pub fn with_k(self, k: usize) -> Self {
        Self {
            extract_strat: ExtractStrat::CountLUT(k),
            ..self
        }
    }

    /// Extract based on circuit depth.
    pub fn with_depth(self) -> Self {
        Self {
            extract_strat: ExtractStrat::Depth,
            ..self
        }
    }

    /// Request with at most `iter_limit` rewrite iterations.
    pub fn with_iter_limit(self, iter_limit: usize) -> Self {
        Self {
            iter_limit,
            result: None,
            ..self
        }
    }

    /// Request with at most `node_limit` nodes.
    pub fn with_node_limit(self, node_limit: usize) -> Self {
        Self {
            node_limit,
            result: None,
            ..self
        }
    }

    /// Run exploration for at most `timeout` seconds.
    pub fn with_timeout(self, timeout: u64) -> Self {
        Self {
            timeout,
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
    pub fn with_expr(self, expr: RecExpr<LutLang>) -> Self {
        Self {
            expr,
            result: None,
            ..self
        }
    }

    /// Run exploration with `more_rules` added on.
    pub fn with_rules(mut self, mut more_rules: Vec<Rewrite<LutLang, A>>) -> Self {
        self.rules.append(&mut more_rules);
        self.result = None;
        self
    }

    fn explore(&mut self) -> Result<(), String>
    where
        A: Analysis<LutLang> + std::default::Default,
    {
        let runner = if self.gen_proof {
            eprintln!("WARNING: Proof generation is on (slow)");
            Runner::default().with_explanations_enabled()
        } else {
            Runner::default().with_explanations_disabled()
        };

        // Print a progress bar to get a sense of growth
        let mp = MultiProgress::new();

        // Use back-off scheduling on runner to avoid transpositions taking too much time
        let bos = BackoffScheduler::default()
            .with_ban_length(1)
            .with_initial_match_limit(960);

        let runner = runner
            .with_scheduler(bos)
            .with_time_limit(Duration::from_secs(self.timeout))
            .with_node_limit(self.node_limit)
            .with_iter_limit(self.iter_limit);

        let runner = if self.prog_bar {
            let (mut iter_bar, mut node_bar) = (
                mp.add(ProgressBar::new(self.iter_limit as u64).with_message("iterations")),
                mp.add(ProgressBar::new(self.node_limit as u64)),
            );
            iter_bar.set_style(
                ProgressStyle::with_template("[{bar:60.cyan/blue}] {pos}/{len} iterations")
                    .unwrap(),
            );
            node_bar.set_style(
                ProgressStyle::with_template("[{bar:60.magenta}] {pos}/{len} nodes").unwrap(),
            );
            runner.with_hook(move |r| report_progress(r, &mut iter_bar, &mut node_bar))
        } else {
            runner
        };

        if self.expr.as_ref().len() > self.max_canon_size {
            eprintln!(
                "WARNING: Input is too large to canonicalize ({} nodes)",
                self.expr.as_ref().len()
            );
        } else if !self.no_canonicalize {
            self.expr = canonicalize_expr(self.expr.clone())
        };

        if self.gen_proof {
            let info = LutExprInfo::new(&self.expr);
            if info.check(&self.expr).is_not_equiv() {
                return Err(format!(
                    "Folding the initial expression had an error: {}",
                    self.expr
                ));
            }
        }

        self.result = Some(runner.with_expr(&self.expr).run(&self.rules));

        // Clear the progress bar
        mp.clear().unwrap();

        Ok(())
    }

    /// simplify `expr` using egg with cost model `c`
    pub fn simplify_expr_with<C>(&mut self, c: C) -> Result<SynthOutput, String>
    where
        A: Analysis<LutLang> + std::default::Default,
        C: CostFunction<LutLang>,
    {
        if self.result.is_none() {
            self.explore()?;
        }
        let runner = self.result.as_mut().unwrap();

        // the Runner knows which e-class the expression given with `with_expr` is in
        let root = runner.roots[0];
        if self.gen_proof {
            let report = runner.report();
            eprintln!("INFO: {}", report.to_string().replace('\n', "\nINFO: "));
        }

        // use an Extractor to pick the best element of the root eclass
        let extraction_start = Instant::now();
        let extractor = Extractor::new(&runner.egraph, c);
        let (_best_cost, best) = extractor.find_best(root);
        let extraction_time = extraction_start.elapsed();
        if self.gen_proof {
            eprintln!(
                "INFO: Extraction time: {} seconds",
                extraction_time.as_secs_f64()
            );
        }
        let expl = if self.gen_proof {
            runner.explain_equivalence(&self.expr, &best).into()
        } else {
            None
        };

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

        // TODO(matth2k): Produce a wider report and print it to file or stderr
        let rpt = if self.produce_rpt {
            let rpt = SynthReport {
                extract_time: extraction_time.as_secs_f64(),
            };
            eprintln!("INFO: Report produced");
            eprintln!("INFO: {}", serde_json::to_string_pretty(&rpt).unwrap());
            Some(rpt)
        } else {
            None
        };

        Ok(SynthOutput {
            expr: best,
            expl,
            rpt,
        })
    }

    /// Simplify expression with the extraction strategy in request `self`.
    pub fn simplify_expr(&mut self) -> Result<SynthOutput, String>
    where
        A: Analysis<LutLang> + std::default::Default,
    {
        match self.extract_strat {
            ExtractStrat::Depth => self.simplify_expr_with(DepthCostFn),
            ExtractStrat::CountLUT(k) => self.simplify_expr_with(KLUTCostFn::new(k)),
        }
    }
}

/// A simple function to read from file, stdin, or a string. The `cmd` takes the most precendence, then files, then stdin.
pub fn simple_reader(cmd: Option<String>, input_file: Option<PathBuf>) -> std::io::Result<String> {
    let mut buf = String::new();
    if cmd.is_some() {
        buf = cmd.unwrap();
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

/// Compile a [LutLang] expression using a baseline request `req`.
/// The output expression is returned as a [SynthOutput]. Everything else goes to stderr.
pub fn process_expression<A>(
    expr: RecExpr<LutLang>,
    req: SynthRequest<A>,
    no_verify: bool,
    verbose: bool,
) -> std::io::Result<SynthOutput>
where
    A: Analysis<LutLang> + Clone + Default,
{
    if no_verify {
        verify_expr(&expr).map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;
    }

    if cfg!(debug_assertions) {
        eprintln!("WARNING: Running with debug assertions is slow");
    }

    let mut req = req.clone().with_expr(expr.clone());

    let mut result = req
        .simplify_expr()
        .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;

    if verbose && result.has_explanation() {
        let proof = result.get_proof();
        let mut linecount = 0;
        for line in proof.lines() {
            eprintln!("INFO: {}", line);
            linecount += 1;
        }
        eprintln!("INFO: Approx. {} lines in proof tree", linecount);
        eprintln!("INFO: ============================================================");
    } else if req.expr.as_ref().len() < 240 {
        let expr = expr.to_string();
        let len = expr.len().min(240);
        eprintln!("INFO: {} ... => ", &expr[0..len]);
    }

    let simplified = result.get_expr();

    // Verify functionality
    if no_verify {
        eprintln!("INFO: Skipping functionality tests...");
    } else {
        let check = LutExprInfo::new(&expr).check(simplified);
        if check.is_inconclusive() && verbose {
            eprintln!("WARNING: Functionality verification inconclusive");
        }
        if check.is_not_equiv() {
            match result.get_expl() {
                Some(e) => eprintln!("ERROR: Failed for explanation {}", e),
                None => eprintln!("ERROR: Failed for unknown reason. Try running with --verbose for an attempted proof"),
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Functionality verification failed",
            ));
        }
    }
    Ok(result)
}

/// Compile a [LutLang] expression from a line of text using a baseline request `req`.
/// The output expression is returned as a [SynthOutput]. Everything else goes to stderr.
pub fn process_string_expression<A>(
    line: &str,
    req: SynthRequest<A>,
    no_verify: bool,
    verbose: bool,
) -> std::io::Result<SynthOutput>
where
    A: Analysis<LutLang> + Clone + Default,
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
    let expr: RecExpr<LutLang> = expr
        .parse()
        .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;

    process_expression(expr, req, no_verify, verbose)
}
