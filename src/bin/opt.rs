use clap::Parser;
use egg::*;
#[cfg(feature = "dyn_decomp")]
use lut_synth::rewrite::dyn_decompositions;
use lut_synth::{
    analysis::LutAnalysis,
    driver::{process_string_expression, simple_reader, SynthRequest},
    lut,
    rewrite::{all_static_rules, register_retiming},
};
use std::path::PathBuf;

fn get_main_runner(s: &str) -> Result<SynthRequest<LutAnalysis>, RecExprParseError<FromOpError>> {
    let expr: RecExpr<lut::LutLang> = s.parse()?;
    let mut rules = all_static_rules(false);
    rules.append(&mut register_retiming());

    Ok(SynthRequest::default()
        .with_expr(expr)
        .with_rules(rules)
        .with_k(4)
        .with_asserts()
        .without_progress_bar()
        .with_timeout(20)
        .with_node_limit(20_000)
        .with_iter_limit(30))
}

#[allow(dead_code)]
/// parse an expression, simplify it with DSD and at most 4 fan-in, and pretty print it back out
fn simplify(s: &str) -> String {
    let mut req = get_main_runner(s).unwrap();
    req.simplify_expr().unwrap().get_expr().to_string()
}

#[allow(dead_code)]
/// parse an expression, simplify it with DSD and at most 4 fan-in, and pretty print it back out
fn simplify_w_proof(s: &str) -> String {
    let mut req = get_main_runner(s).unwrap().with_proof();
    req.simplify_expr().unwrap().get_expr().to_string()
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

    /// Disassemble the LUTs into their constituent gates
    #[cfg(feature = "dyn_decomp")]
    #[arg(long, default_value_t = false)]
    disassemble: bool,

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

    /// Max fan in size for LUTs
    #[arg(short = 'k', long, default_value_t = 4)]
    k: usize,

    /// Timeout in seconds for each expression
    #[arg(short = 't', long,
        default_value_t =
        if cfg!(debug_assertions) {
            30
        } else {
            10
        })
    ]
    timeout: u64,

    /// Maximum number of nodes in graph
    #[arg(short = 's', long, default_value_t = 24_000)]
    node_limit: usize,

    /// Maximum number of rewrite iterations
    #[arg(short = 'n', long, default_value_t = 32)]
    iter_limit: usize,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    if cfg!(debug_assertions) {
        eprintln!("WARNING: Debug assertions are enabled");
    }

    let buf = simple_reader(args.command, args.input)?;

    let mut rules = all_static_rules(false);

    #[cfg(feature = "dyn_decomp")]
    if args.disassemble {
        rules = all_static_rules(true);
    }

    #[cfg(feature = "dyn_decomp")]
    if args.decomp || args.disassemble {
        rules.append(&mut dyn_decompositions());
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

    let req = SynthRequest::default()
        .with_rules(rules)
        .with_k(args.k)
        .with_timeout(args.timeout)
        .with_node_limit(args.node_limit)
        .with_iter_limit(args.iter_limit);

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

    #[cfg(feature = "dyn_decomp")]
    let req = if args.disassemble {
        req.with_disassembler()
    } else {
        req
    };

    #[cfg(feature = "exactness")]
    let req = if args.exact {
        req.with_exactness()
    } else {
        req
    };

    for line in buf.lines() {
        let result = process_string_expression(line, req.clone(), args.no_verify, args.verbose)?;
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
    assert_eq!(simplify("(LUT 202 true a b)"), "a");
    assert_eq!(simplify("(LUT 0 a)"), "false");
    assert_eq!(simplify("(LUT 3 a)"), "true");
    assert_eq!(simplify("(LUT 3 a b c)"), "(LUT 1 a b)");
    assert_eq!(
        lut::canonicalize_expr(
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
