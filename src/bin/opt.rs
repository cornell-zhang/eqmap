use clap::Parser;
use egg::*;
#[cfg(feature = "dyn_decomp")]
use lut_synth::rewrite::dyn_decompositions;
use lut_synth::{
    analysis::LutAnalysis,
    driver::{SynthReport, SynthRequest, process_string_expression, simple_reader},
    lut,
    lut::LutLang,
    rewrite::{all_static_rules, register_retiming},
};
use std::path::PathBuf;

fn get_main_runner(
    s: &str,
) -> Result<SynthRequest<LutLang, LutAnalysis>, RecExprParseError<FromOpError>> {
    let expr: RecExpr<lut::LutLang> = s.parse()?;
    let mut rules = all_static_rules(false);
    rules.append(&mut register_retiming());

    Ok(SynthRequest::default()
        .with_expr(expr)
        .with_rules(rules)
        .with_k(4)
        .with_asserts()
        .without_progress_bar()
        .with_joint_limits(20, 20_000, 30))
}

#[allow(dead_code)]
/// parse an expression, simplify it with DSD and at most 4 fan-in, and pretty print it back out
fn simplify(s: &str) -> String {
    let mut req = get_main_runner(s).unwrap();
    req.simplify_expr::<SynthReport>()
        .unwrap()
        .get_expr()
        .to_string()
}

#[allow(dead_code)]
/// parse an expression, simplify it with DSD and at most 4 fan-in, and pretty print it back out
fn simplify_w_proof(s: &str) -> String {
    let mut req = get_main_runner(s).unwrap().with_proof();
    req.simplify_expr::<SynthReport>()
        .unwrap()
        .get_expr()
        .to_string()
}

/// Technology Mapping Optimization with E-Graphs
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to input file. If not provided, reads from stdin
    input: Option<PathBuf>,

    /// If provided, output a condensed JSON file with the e-graph
    #[cfg(feature = "graph_dumps")]
    #[arg(long)]
    dump_graph: Option<PathBuf>,

    /// Return an error if the graph does not reach saturation
    #[arg(short = 'a', long, default_value_t = false)]
    assert_sat: bool,

    /// Do not verify the functionality of the output
    #[arg(short = 'f', long, default_value_t = false)]
    no_verify: bool,

    /// Don't canonicalize the input expression
    #[arg(short = 'c', long, default_value_t = false)]
    no_canonicalize: bool,

    /// Opt a specific LUT expr instead of from file
    #[arg(long)]
    command: Option<String>,

    /// Find new decompositions at runtime
    #[cfg(feature = "dyn_decomp")]
    #[arg(short = 'd', long, default_value_t = false)]
    decomp: bool,

    /// Comma separated list of cell types to decompose into
    #[cfg(feature = "dyn_decomp")]
    #[arg(long)]
    disassemble: Option<String>,

    /// Perform an exact extraction using ILP (much slower)
    #[cfg(feature = "exactness")]
    #[arg(short = 'e', long, default_value_t = false)]
    exact: bool,

    /// Don't use register retiming
    #[arg(short = 'r', long, default_value_t = false)]
    no_retime: bool,

    /// Print explanations (this generates a proof and runs longer)
    #[arg(short = 'v', long, default_value_t = false)]
    verbose: bool,

    /// Extract for minimum circuit depth
    #[arg(long, default_value_t = false)]
    min_depth: bool,

    /// Max fan in size allowed for extracted LUTs
    #[arg(short = 'k', long, default_value_t = 4)]
    k: usize,

    /// Ratio of register cost to LUT cost
    #[arg(short = 'w', long, default_value_t = 1)]
    reg_weight: u64,

    /// Build/extraction timeout in seconds
    #[arg(short = 't', long)]
    timeout: Option<u64>,

    /// Maximum number of nodes in graph
    #[arg(short = 's', long)]
    node_limit: Option<usize>,

    /// Maximum number of rewrite iterations
    #[arg(short = 'n', long)]
    iter_limit: Option<usize>,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    if cfg!(debug_assertions) {
        eprintln!("WARNING: Debug assertions are enabled");
    }

    let buf = simple_reader(args.command, args.input)?;

    let mut rules = all_static_rules(false);

    #[cfg(feature = "dyn_decomp")]
    if args.disassemble.is_some() {
        rules = all_static_rules(true);
    }

    #[cfg(feature = "dyn_decomp")]
    if args.decomp || args.disassemble.is_some() {
        rules.append(&mut dyn_decompositions(false));
    }

    if !args.no_retime {
        rules.append(&mut register_retiming());
    }

    if args.verbose {
        eprintln!("INFO: Running with {} rewrite rules", rules.len());
        #[cfg(feature = "dyn_decomp")]
        eprintln!(
            "INFO: Dynamic Decomposition {}",
            if args.decomp { "ON" } else { "OFF" }
        );
    }

    let req = SynthRequest::default().with_rules(rules).with_k(args.k);

    let req = match (args.timeout, args.node_limit, args.iter_limit) {
        (None, None, None) => req.with_joint_limits(10, 24_000, 32),
        (Some(t), None, None) => req.time_limited(t),
        (None, Some(n), None) => req.node_limited(n),
        (None, None, Some(i)) => req.iter_limited(i),
        (Some(t), Some(n), Some(i)) => req.with_joint_limits(t, n, i),
        _ => {
            return Err(std::io::Error::other(
                "Invalid build constraints (Use none, one, or three build constraints)",
            ));
        }
    };

    let req = if args.assert_sat {
        req.with_asserts()
    } else {
        req
    };

    let req = if args.no_canonicalize {
        req.without_canonicalization()
    } else {
        req
    };

    let req = if args.verbose { req.with_proof() } else { req };

    #[cfg(feature = "graph_dumps")]
    let req = match args.dump_graph {
        Some(p) => req.with_graph_dump(p),
        None => req,
    };

    let req = if args.min_depth {
        req.with_min_depth()
    } else {
        req.with_klut_regw(args.k, args.reg_weight)
    };

    #[cfg(feature = "dyn_decomp")]
    let req = match args.disassemble {
        Some(list) => req
            .without_canonicalization()
            .with_disassembly_into(&list)
            .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?,
        None => req,
    };

    #[cfg(feature = "exactness")]
    let req = if args.exact {
        req.with_exactness(args.timeout.unwrap_or(600))
    } else {
        req
    };

    for line in buf.lines() {
        let result = process_string_expression::<_, _, SynthReport>(
            line,
            req.clone(),
            args.no_verify,
            args.verbose,
        )?;
        if !result.is_empty() {
            println!("{}", result);
        }
    }
    Ok(())
}

#[test]
fn simple_tests() {
    assert_eq!(simplify("(LUT 2 a b)"), "(LUT 2 a b)");
    assert_eq!(simplify("(LUT 0 a)"), "false");
    assert_eq!(simplify("(LUT 3 b)"), "true");
    assert_eq!(simplify("(LUT 1 true)"), "false");
    assert_eq!(simplify("(LUT 2 true)"), "true");
    assert_eq!(simplify("(LUT 1 false)"), "true");
    assert_eq!(simplify("(LUT 2 false)"), "false");
    assert_eq!(
        simplify("(LUT 202 s1 (LUT 8 a b) (LUT 6 a b))"),
        "(LUT 134 s1 a b)"
    );
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
fn test_args_egraphs() {
    assert_eq!(simplify("(CYCLE (REG (ARG 0)))"), "(CYCLE (REG (ARG 0)))");
    assert_eq!(
        simplify("(CYCLE (REG (NOT (ARG 0))))"),
        "(CYCLE (LUT 1 (REG (ARG 0))))"
    );
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
fn test_greedy_folds() {
    use lut_synth::driver::Canonical;
    assert_eq!(simplify("(LUT 202 true a b)"), "a");
    assert_eq!(simplify("(LUT 0 a)"), "false");
    assert_eq!(simplify("(LUT 3 a)"), "true");
    assert_eq!(simplify("(LUT 3 a b c)"), "(LUT 1 a b)");
    assert_eq!(
        LutLang::canonicalize_expr(
            "(LUT 6 true (LUT 6 false (LUT 6 true false)))"
                .parse()
                .unwrap()
        )
        .to_string(),
        "false"
    );
}

#[test]
fn test_exploration() {
    // Since we have greedy folding now,
    // we need different kinds of inputs that don't optimize as well
    assert_eq!(simplify("(LUT 6 (LUT 6 c d) b)"), "(LUT 150 c d b)")
}
