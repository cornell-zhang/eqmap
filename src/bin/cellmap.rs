use clap::Parser;
use egg::{FromOpError, RecExpr, RecExprParseError};
use lut_synth::{
    asic::{CellAnalysis, CellLang, CellRpt, asic_rewrites},
    driver::{SynthRequest, process_string_expression, simple_reader},
};
use std::path::PathBuf;

fn get_main_runner(
    s: &str,
) -> Result<SynthRequest<CellLang, CellAnalysis>, RecExprParseError<FromOpError>> {
    let expr: RecExpr<CellLang> = s.parse()?;
    let rules = asic_rewrites();

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
    req.simplify_expr::<CellRpt>()
        .unwrap()
        .get_expr()
        .to_string()
}

#[allow(dead_code)]
/// parse an expression, simplify it with DSD and at most 4 fan-in, and pretty print it back out
fn simplify_w_proof(s: &str) -> String {
    let mut req = get_main_runner(s).unwrap().with_proof();
    req.simplify_expr::<CellRpt>()
        .unwrap()
        .get_expr()
        .to_string()
}

/// ASIC Technology Mapping Optimization with E-Graphs
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
    verify: bool,

    /// Don't canonicalize the input expression
    #[arg(short = 'c', long, default_value_t = false)]
    canonicalize: bool,

    /// Opt a specific LUT expr instead of from file
    #[arg(long)]
    command: Option<String>,

    /// Perform an exact extraction using ILP (much slower)
    #[cfg(feature = "exactness")]
    #[arg(short = 'e', long, default_value_t = false)]
    exact: bool,

    /// Print explanations (this generates a proof and runs longer)
    #[arg(short = 'v', long, default_value_t = false)]
    verbose: bool,

    /// Extract for minimum circuit depth
    #[arg(long, default_value_t = false)]
    min_depth: bool,

    /// Max fan in size allowed for extracted Cells
    #[arg(short = 'k', long, default_value_t = 4)]
    k: usize,

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

    let rules = asic_rewrites();

    if args.verbose {
        eprintln!("INFO: Running with {} rewrite rules", rules.len());
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

    let req = if args.canonicalize {
        req
    } else {
        req.without_canonicalization()
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
        req.with_k(args.k)
    };

    #[cfg(feature = "exactness")]
    let req = if args.exact {
        req.with_exactness(args.timeout.unwrap_or(600))
    } else {
        req
    };

    for line in buf.lines() {
        let result = process_string_expression::<CellLang, _, CellRpt>(
            line,
            req.clone(),
            !args.verify,
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
    assert_eq!(simplify("(AND a b)"), "(INV_X1 (NAND2_X1 a b))");
    assert_eq!(simplify("(INV a)"), "(INV_X1 a)");
    assert_eq!(simplify("(AND a true)"), "a");
}
