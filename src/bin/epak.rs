use clap::Parser;
#[cfg(feature = "dyn_decomp")]
use eqmap::rewrite::dyn_decompositions;
use eqmap::{
    driver::{SynthReport, SynthRequest, process_expression},
    rewrite::{all_static_rules, register_retiming},
    verilog::{SVModule, sv_parse_wrapper},
};
use std::{
    io::{Read, Write, stdin},
    path::PathBuf,
};

/// Technology Mapping Optimization with E-Graphs
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Verilog file to read from (or use stdin)
    input: Option<PathBuf>,

    /// Verilog file to output to (or use stdout)
    output: Option<PathBuf>,

    /// If provided, output a JSON file with result data
    #[arg(long)]
    report: Option<PathBuf>,

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

    /// Do not canonicalize the input into LUTs
    #[arg(short = 'c', long, default_value_t = false)]
    no_canonicalize: bool,

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

    /// Do not use register retiming
    #[arg(short = 'r', long, default_value_t = false)]
    no_retime: bool,

    /// Print explanations (generates a proof and runs slower)
    #[arg(short = 'v', long, default_value_t = false)]
    verbose: bool,

    /// Extract for minimum circuit depth
    #[arg(long, default_value_t = false)]
    min_depth: bool,

    /// Max fan in size allowed for extracted LUTs
    #[arg(short = 'k', long, default_value_t = 6)]
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

    eprintln!("INFO: E-Pack Technology Mapping Optimization with E-Graphs");

    let mut buf = String::new();

    let path: Option<PathBuf> = match args.input {
        Some(p) => {
            std::fs::File::open(&p)?.read_to_string(&mut buf)?;
            Some(p)
        }
        None => {
            eprintln!("INFO: Reading from stdin...");
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

    let mut rules = all_static_rules(false);

    #[cfg(feature = "dyn_decomp")]
    if args.disassemble.is_some() {
        rules = all_static_rules(true);
    }

    #[cfg(feature = "dyn_decomp")]
    if args.decomp || args.disassemble.is_some() {
        rules.append(&mut dyn_decompositions(true));
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
        eprintln!(
            "INFO: Retiming rewrites {}",
            if args.no_retime { "OFF" } else { "ON" }
        );
    }

    let req = SynthRequest::default().with_rules(rules);

    let req = match (args.timeout, args.node_limit, args.iter_limit) {
        (None, None, None) => req.with_joint_limits(10, 48_000, 32),
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

    let req = if args.report.is_some() {
        req.with_report()
    } else {
        req
    };

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

    #[cfg(feature = "exactness")]
    if args.exact && args.output.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Stdout is reserved for cbc solver. Specify an output file",
        ));
    }

    eprintln!("INFO: Compiling Verilog...");
    let expr = f
        .to_single_lut_expr()
        .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;

    eprintln!("INFO: Building e-graph...");
    let result = process_expression::<_, _, SynthReport>(expr, req, args.no_verify, args.verbose)?
        .with_name(f.get_name());

    if let Some(p) = args.report {
        let mut writer = std::fs::File::create(p)?;
        result.write_report(&mut writer)?;
    }

    eprintln!("INFO: Writing output to Verilog...");
    let output_names: Vec<String> = f.get_outputs().iter().map(|x| x.to_string()).collect();
    let mut module = SVModule::from_luts(
        result.get_expr().to_owned(),
        f.get_name().to_string(),
        output_names,
    )
    .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;

    // Unused inputs from the original module are lost upon conversion to a LutLang expression so
    // they must be readded to the module here.
    module.append_inputs_from_module(&f);

    if let Some(p) = args.output {
        let mut file = std::fs::File::create(p)?;
        write!(
            file,
            "/* Generated by {} {} */\n\n{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            module
        )?;
        eprintln!("INFO: Goodbye");
    } else {
        print!("{}", module);
    }

    Ok(())
}
