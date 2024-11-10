use clap::Parser;
use egg::*;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use lut_synth::{
    cost::KLUTCostFn,
    lut::{self, LutExprInfo},
    rewrite::{all_rules_minus_dsd, known_decompositions, register_retiming},
};
use std::{
    io::{IsTerminal, Read, Write},
    path::PathBuf,
    time::{Duration, Instant},
};

fn report_progress<A>(
    runner: &Runner<lut::LutLang, A>,
    iter_bar: &mut ProgressBar,
    node_bar: &mut ProgressBar,
) -> Result<(), String>
where
    A: Analysis<lut::LutLang> + std::default::Default,
{
    iter_bar.inc(1);
    let nodes = runner.egraph.total_number_of_nodes();
    node_bar.set_position(nodes as u64);
    Ok(())
}

/// A request to simplify an expression.
struct SimplifyRequest<'a, A> {
    /// The expression to simplify.
    expr: &'a RecExpr<lut::LutLang>,

    /// The rewrite rules used to simplify the expression.
    rules: &'a Vec<Rewrite<lut::LutLang, A>>,

    /// The greatest fan-in on any LUT in an expression.
    k: usize,

    /// If true, an error is returned if saturation is not met.
    assert_sat: bool,

    /// If true, a proof of the simplification should be generation. If false, no proof will be
    /// generated.
    gen_proof: bool,

    /// If true, a progress bar will be displayed, else no progress bar will be displayed.
    prog_bar: bool,

    /// The timelimit, in seconds, given to the search.
    timeout: u64,

    /// The maximum number of enodes in a searched egraph.
    node_limit: usize,

    /// The maximum number of iterations of rewrites applied to the egraph.
    iter_limit: usize,
}

/// simplify `expr` using egg with at most `k` fan-in on LUTs
fn simplify_expr<A>(
    req: &SimplifyRequest<A>,
) -> Result<(RecExpr<lut::LutLang>, Option<Explanation<lut::LutLang>>), String>
where
    A: Analysis<lut::LutLang> + std::default::Default,
{
    // simplify the expression using a Runner, which creates an e-graph with
    // the given expression and runs the given rules over it

    let runner = if req.gen_proof {
        eprintln!("WARNING: Proof generation is on (slow)");
        Runner::default().with_explanations_enabled()
    } else {
        Runner::default().with_explanations_disabled()
    };

    // Print a progress bar to get a sense of growth
    let mp = MultiProgress::new();

    // Use back-off scheduling on runner to avoid transpositions taking too much time
    let bos = BackoffScheduler::default()
        .with_ban_length(2)
        .with_initial_match_limit(960);

    let runner = runner
        .with_scheduler(bos)
        .with_time_limit(Duration::from_secs(req.timeout))
        .with_node_limit(req.node_limit)
        .with_iter_limit(req.iter_limit);

    let runner = if req.prog_bar {
        let (mut iter_bar, mut node_bar) = (
            mp.add(ProgressBar::new(req.iter_limit as u64).with_message("iterations")),
            mp.add(ProgressBar::new(req.node_limit as u64)),
        );
        iter_bar.set_style(
            ProgressStyle::with_template("[{bar:60.cyan/blue}] {pos}/{len} iterations").unwrap(),
        );
        node_bar.set_style(
            ProgressStyle::with_template("[{bar:60.magenta}] {pos}/{len} nodes").unwrap(),
        );
        runner.with_hook(move |r| report_progress(r, &mut iter_bar, &mut node_bar))
    } else {
        runner
    };

    let mut runner = runner.with_expr(req.expr).run(req.rules);

    // Clear the progress bar
    mp.clear().unwrap();

    // the Runner knows which e-class the expression given with `with_expr` is in
    let root = runner.roots[0];
    if req.gen_proof {
        let report = runner.report();
        eprintln!("INFO: {}", report.to_string().replace('\n', "\nINFO: "));
    }

    // use an Extractor to pick the best element of the root eclass
    let extraction_start = Instant::now();
    let extractor = Extractor::new(&runner.egraph, KLUTCostFn::new(req.k));
    let (_best_cost, best) = extractor.find_best(root);
    let extraction_time = extraction_start.elapsed();
    if req.gen_proof {
        eprintln!(
            "INFO: Extraction time: {} seconds",
            extraction_time.as_secs_f64()
        );
    }
    let expl = if req.gen_proof {
        runner.explain_equivalence(req.expr, &best).into()
    } else {
        None
    };

    let stop_reason = runner.stop_reason.unwrap();
    if req.assert_sat && !matches!(stop_reason, egg::StopReason::Saturated) {
        return Err(format!(
            "Expression {} failed to saturate. Grown to {} nodes with reason {:?}",
            req.expr,
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
    Ok((best, expl))
}

#[allow(dead_code)]
/// parse an expression, simplify it with DSD and at most 4 fan-in, and pretty print it back out
fn simplify(s: &str) -> String {
    // parse the expression, the type annotation tells it which Language to use
    let expr: RecExpr<lut::LutLang> = s.parse().unwrap();
    let mut rules = all_rules_minus_dsd();
    rules.append(&mut known_decompositions());

    let req = SimplifyRequest {
        expr: &expr,
        rules: &rules,
        k: 4,
        assert_sat: true,
        gen_proof: false,
        prog_bar: false,
        timeout: 20,
        node_limit: 20_000,
        iter_limit: 30,
    };

    simplify_expr(&req).unwrap().0.to_string()
}

#[allow(dead_code)]
/// parse an expression, simplify it with DSD and at most 4 fan-in, and pretty print it back out
fn simplify_w_proof(s: &str) -> String {
    // parse the expression, the type annotation tells it which Language to use
    let expr: RecExpr<lut::LutLang> = s.parse().unwrap();
    let mut rules = all_rules_minus_dsd();
    rules.append(&mut known_decompositions());

    let req = SimplifyRequest {
        expr: &expr,
        rules: &rules,
        k: 4,
        assert_sat: true,
        gen_proof: true,
        prog_bar: false,
        timeout: 20,
        node_limit: 20_000,
        iter_limit: 30,
    };

    simplify_expr(&req).unwrap().0.to_string()
}

#[test]
fn simple_tests() {
    assert_eq!(simplify("(LUT 2 a b)"), "(LUT 2 a b)");
    assert_eq!(simplify("(LUT 3 a b c)"), "(LUT 1 a b)");

    assert_eq!(simplify("(LUT 0 a)"), "false");
    assert_eq!(simplify("(LUT 3 b)"), "true");
    assert_eq!(simplify("(LUT 1 true)"), "false");
    assert_eq!(simplify("(LUT 2 true)"), "true");
    assert_eq!(simplify("(LUT 1 false)"), "true");
    assert_eq!(simplify("(LUT 2 false)"), "false");
}

#[test]
fn redundant_inputs() {
    assert_eq!(simplify("(LUT 1 a a a a a)"), "(LUT 1 a)");
    assert_eq!(simplify("(LUT 1 a a a a a a)"), "(LUT 1 a)");
    assert_eq!(simplify("(LUT 1 a b a b a b)"), "(LUT 1 a b)");
}

#[test]
fn test_proof_generation() {
    assert_eq!(simplify_w_proof("(LUT 1 a b a b a b)"), "(LUT 1 a b)");
}

#[test]
fn test_eval() {
    let expr: RecExpr<lut::LutLang> = "(MUX s0 a b)".parse().unwrap();
    let other: RecExpr<lut::LutLang> = "(LUT 202 s0 a b)".parse().unwrap();
    assert!(lut::LutLang::func_equiv(&expr, &other).unwrap());
}

#[test]
fn test_dsd() {
    let expr: RecExpr<lut::LutLang> = "(MUX s1 (MUX s0 a b) (MUX s0 c d))".parse().unwrap();
    let other: RecExpr<lut::LutLang> = "(LUT 18374951396690406058 s1 s0 a b c d)".parse().unwrap();
    assert!(lut::LutLang::func_equiv(&expr, &other).unwrap());
    let dsd: RecExpr<lut::LutLang> = "(LUT 51952 s1 (LUT 61642 s1 s0 c d) a b)".parse().unwrap();
    assert!(lut::LutLang::func_equiv(&other, &dsd).unwrap());
}

#[test]
fn test_incorrect_dsd() {
    let expr: RecExpr<lut::LutLang> = "(MUX s1 (MUX s0 a b) (MUX s0 c d))".parse().unwrap();
    let p: u64 = 18374951396690406058;
    for i in 0..64 {
        let pos_to_flip: usize = i;
        let p = p ^ (1 << pos_to_flip);
        let other: RecExpr<lut::LutLang> = format!("(LUT {} s1 s0 a b c d)", p).parse().unwrap();
        assert!(!lut::LutLang::func_equiv(&expr, &other).unwrap());
    }
}

#[test]
fn test_const_input() {
    // TODO(matth2k): Don't yet have a general method to show that an LUT is invariant to an input.
    assert_eq!(simplify("(LUT 202 true a b)"), "a");
    assert_eq!(simplify("(LUT 0 a)"), "false");
    assert_eq!(simplify("(LUT 3 a)"), "true");
}

/// LUT Network Synthesis with E-Graphs
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to input file. If not provided, reads from stdin
    input: Option<PathBuf>,

    /// Return an error if the graph does not reach saturation
    #[arg(short = 'a', long, default_value_t = false)]
    assert_sat: bool,

    /// Don't verify functionality of the output
    #[arg(short = 'f', long, default_value_t = false)]
    no_verify: bool,

    /// Opt a specific LUT expr instead of from file
    #[arg(short = 'c', long)]
    command: Option<String>,

    /// Don't use disjoint set decompositions
    #[arg(short = 'd', long, default_value_t = false)]
    no_dsd: bool,

    /// Don't use register retiming
    #[arg(short = 'r', long, default_value_t = false)]
    no_retime: bool,

    /// Print explanations (this generates a proof and runs longer)
    #[arg(short = 'v', long, default_value_t = false)]
    verbose: bool,

    /// Max fan in size for LUTs
    #[arg(short = 'k', long, default_value_t = 4)]
    k: usize,

    /// Timeout in seconds for each expression
    #[arg(short = 't', long,
        default_value_t =
        if cfg!(debug_assertions) {
            27
        } else {
            9
        })
    ]
    timeout: u64,

    /// Maximum number of nodes in graph
    #[arg(short = 's', long, default_value_t = 18_000)]
    node_limit: usize,

    /// Maximum number of rewrite iterations
    #[arg(short = 'n', long, default_value_t = 24)]
    iter_limit: usize,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let mut buf = String::new();

    if cfg!(debug_assertions) {
        eprintln!("WARNING: Debug assertions are enabled");
    }

    if args.command.is_some() {
        buf = args.command.unwrap();
    } else if args.input.is_some() {
        std::fs::File::open(args.input.unwrap())?.read_to_string(&mut buf)?;
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

    let mut rules = all_rules_minus_dsd();
    if !args.no_dsd {
        rules.append(&mut known_decompositions());
    }

    if !args.no_retime {
        rules.append(&mut register_retiming());
    }

    if args.verbose {
        eprintln!("INFO: Running with {} rewrite rules", rules.len());
        eprintln!(
            "INFO: DSD rewrites {}",
            if args.no_dsd { "OFF" } else { "ON" }
        );
    }

    for line in buf.lines() {
        let line = line.trim();
        if line.starts_with("//") || line.is_empty() {
            continue;
        }
        let expr = line.split("//").next().unwrap();
        let expr: RecExpr<lut::LutLang> = expr.parse().unwrap();

        if !args.no_verify {
            lut::verify_expr(&expr)
                .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;
        }

        if cfg!(debug_assertions) {
            eprintln!("WARNING: Running with debug assertions is slow");
        }

        let req = SimplifyRequest {
            expr: &expr,
            rules: &rules,
            k: args.k,
            assert_sat: args.assert_sat,
            gen_proof: args.verbose,
            prog_bar: true,
            timeout: args.timeout,
            node_limit: args.node_limit,
            iter_limit: args.iter_limit,
        };

        let (simplified, expl) =
            simplify_expr(&req).map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;
        let expl: Option<String> = match expl {
            Some(mut e) => {
                let proof = e.get_flat_string();
                let linecount = proof.lines().count();
                format!("{}\n Approx. {} lines in proof tree", proof, linecount).into()
            }
            None => None,
        };

        if args.verbose {
            eprintln!("INFO: {}", expl.as_ref().unwrap().replace('\n', "\nINFO: "));
            eprintln!("INFO: ============================================================");
        } else {
            eprintln!("{} => ", expr);
        }

        println!("{}", simplified);

        // Verify functionality
        if args.no_verify {
            eprintln!("INFO: Skipping functionality tests...");
        } else {
            let result = LutExprInfo::new(&expr).check(&simplified);
            if result.is_inconclusive() && args.verbose {
                eprintln!("WARNING: Functionality verification inconclusive");
            }
            if result.is_not_equiv() {
                match expl.as_ref() {
                    Some(e) => eprintln!("ERROR: Failed for explanation {}", e),
                    None => eprintln!("ERROR: Failed for unknown reason. Try running with --verbose for an attempted proof"),
                }
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Functionality verification failed",
                ));
            }
        }
    }
    Ok(())
}
