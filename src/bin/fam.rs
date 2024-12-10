use clap::Parser;
use lut_synth::{
    driver::{process_expression, SynthRequest},
    rewrite::{all_rules_minus_dyn_decomp, register_retiming},
    verilog::{sv_parse_wrapper, SVModule},
};
use std::{
    io::{stdin, Read},
    path::PathBuf,
};

/// Tech Re-Mapping with E-Graphs
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to input verilog file. If not provided, reads from stdin
    input: Option<PathBuf>,

    /// Path to output verilog file. If not provided, emits to stdout
    output: Option<PathBuf>,

    /// Path to output report JSON file. If not provided, does not emit a report
    #[arg(long)]
    report: Option<PathBuf>,

    /// Return an error if the graph does not reach saturation
    #[arg(short = 'a', long, default_value_t = false)]
    assert_sat: bool,

    /// Do not verify functionality of the output
    #[arg(short = 'f', long, default_value_t = false)]
    no_verify: bool,

    /// Do not canonicalize the input into LUTs
    #[arg(short = 'c', long, default_value_t = false)]
    no_canonicalize: bool,

    /// Opt a specific LUT expr instead of from file
    #[arg(long)]
    command: Option<String>,

    /// Find new decompositions at runtime
    #[arg(short = 'd', long, default_value_t = false)]
    decomp: bool,

    /// Perform an exact extraction using ILP (much slower)
    #[arg(short = 'e', long, default_value_t = false)]
    exact: bool,

    /// Do not use register retiming
    #[arg(short = 'r', long, default_value_t = false)]
    no_retime: bool,

    /// Print explanations (this generates a proof and runs much slower)
    #[arg(short = 'v', long, default_value_t = false)]
    verbose: bool,

    /// Extract based on min circuit depth instead of using 'k'. Can cause inf loop
    #[arg(long, default_value_t = false)]
    min_depth: bool,

    /// Max fan in size for extracted LUTs
    #[arg(short = 'k', long, default_value_t = 6)]
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
    #[arg(short = 's', long, default_value_t = 48_000)]
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

    let mut buf = String::new();

    let path: Option<PathBuf> = match args.input {
        Some(p) => {
            std::fs::File::open(&p)?.read_to_string(&mut buf)?;
            Some(p)
        }
        None => {
            stdin().read_to_string(&mut buf)?;
            None
        }
    };

    let ast = sv_parse_wrapper(&buf, path)
        .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;

    let f =
        SVModule::from_ast(&ast).map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;

    eprintln!(
        "INFO: Module {} has {} outputs",
        f.get_name(),
        f.get_outputs().len()
    );

    let mut rules = all_rules_minus_dyn_decomp();
    if args.decomp {
        todo!("Dynamic decomposition is not implemented yet");
    }

    if !args.no_retime {
        rules.append(&mut register_retiming());
    }

    if args.verbose {
        eprintln!("INFO: Running with {} rewrite rules", rules.len());
        eprintln!(
            "INFO: Dynamic Decomposition {}",
            if args.decomp { "ON" } else { "OFF" }
        );
        eprintln!(
            "INFO: Retiming rewrites {}",
            if args.no_retime { "OFF" } else { "ON" }
        );
    }

    let req = SynthRequest::default()
        .with_rules(rules)
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

    let req = if args.report.is_some() {
        req.with_report()
    } else {
        req
    };

    let req = if args.min_depth {
        req.with_min_depth()
    } else if args.exact {
        req.with_exactness()
    } else {
        req.with_k(args.k)
    };

    eprintln!("INFO: Compiling Verilog...");
    let expr = f
        .to_single_expr()
        .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;

    eprintln!("INFO: Building e-graph...");
    let result =
        process_expression(expr, req, args.no_verify, args.verbose)?.with_name(f.get_name());

    if let Some(p) = args.report {
        eprintln!("INFO: Emitting report...");
        let mut writer = std::fs::File::create(p)?;
        result.write_report(&mut writer)?;
    }

    eprintln!("INFO: Writing output to Verilog...");
    let output_names: Vec<String> = f.get_outputs().iter().map(|x| x.to_string()).collect();
    let mut module = SVModule::from_expr(
        result.get_expr().to_owned(),
        f.get_name().to_string(),
        output_names,
    )
    .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;

    // Unused inputs from the original module are lost upon conversion to a LutLang expression so
    // they must be readded to the module here.
    let mut new_inputs = f
        .inputs
        .clone()
        .into_iter()
        .filter(|i| !module.inputs.contains(i))
        .collect();
    module.append_inputs(&mut new_inputs);

    if let Some(p) = args.output {
        std::fs::write(p, module.to_string())?;
    } else {
        println!("{}", module);
    }

    Ok(())
}
