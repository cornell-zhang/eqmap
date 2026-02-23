use clap::Parser;
#[cfg(any(feature = "exact_cbc", feature = "exact_highs"))]
use clap::ValueEnum;
#[cfg(feature = "dyn_decomp")]
use eqmap::rewrite::dyn_decompositions;
use eqmap::{
    driver::{SynthReport, SynthRequest, process_expression},
    lut::LutLang,
    netlist::{LogicMapper, PrimitiveCell},
    rewrite::{all_static_rules, register_retiming},
    verilog::sv_parse_wrapper,
};
use nl_compiler::from_vast_overrides;
use safety_net::Identifier;
use std::{
    io::{Read, Write, stderr, stdin},
    path::PathBuf,
};

#[cfg(any(feature = "exact_cbc", feature = "exact_highs"))]
#[derive(Debug, Clone, ValueEnum)]
enum Solver {
    #[cfg(feature = "exact_cbc")]
    Cbc,
    #[cfg(feature = "exact_highs")]
    Highs,
}

/// EqMap: FPGA Technology Mapping w/ E-Graphs
#[derive(Parser, Debug)]
#[command(version, long_about = None)]
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
    #[cfg(any(feature = "exact_cbc", feature = "exact_highs"))]
    #[arg(long, value_enum)]
    exact: Option<Solver>,

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

fn xilinx_overrides(id: &Identifier, cell: &PrimitiveCell) -> Option<PrimitiveCell> {
    if id.get_name() == "INV" {
        Some(
            cell.clone()
                .remap_input(0, "I".into())
                .remap_output(0, "O".into()),
        )
    } else {
        None
    }
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    if cfg!(debug_assertions) {
        eprintln!("WARNING: Debug assertions are enabled");
    }

    eprintln!("INFO: EqMap (FPGA Technology Mapping w/ E-Graphs)");

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

    eprintln!("INFO: Parsing Verilog...");
    let ast = sv_parse_wrapper(&buf, path).map_err(std::io::Error::other)?;

    eprintln!("INFO: Compiling Verilog...");
    let f = from_vast_overrides(&ast, xilinx_overrides).map_err(std::io::Error::other)?;

    eprintln!(
        "INFO: Module {} has {} outputs",
        f.get_name(),
        f.get_output_ports().len()
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

    // if !args.no_retime {
    //     rules.append(&mut register_retiming());
    // }

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
            .map_err(std::io::Error::other)?,
        None => req,
    };

    #[cfg(any(feature = "exact_cbc", feature = "exact_highs"))]
    let req = if let Some(solver) = &args.exact {
        let timeout = args.timeout.unwrap_or(600);
        match solver {
            #[cfg(feature = "exact_cbc")]
            Solver::Cbc => req.with_cbc(timeout),
            #[cfg(feature = "exact_highs")]
            Solver::Highs => req.with_highs(timeout),
        }
    } else {
        req
    };

    #[cfg(any(feature = "exact_cbc", feature = "exact_highs"))]
    if args.exact.is_some() && args.output.is_none() {
        return Err(std::io::Error::other(
            "Stdout is clutterd by ILP solver. Specify an output file",
        ));
    }

    eprintln!("INFO: Extracting logic...");
    let mut mapper = f
        .get_analysis::<LogicMapper<LutLang, PrimitiveCell>>()
        .map_err(std::io::Error::other)?;
    if args.no_retime {
        mapper.insert_all_r2r().map_err(std::io::Error::other)?;
    } else {
        mapper.insert_partitioned().map_err(std::io::Error::other)?;
    }
    let mut mapping = mapper.mappings();
    let mapping = mapping.pop().unwrap();
    let expr = mapping.get_expr();

    eprintln!("INFO: Building e-graph...");
    let result = process_expression::<_, _, SynthReport>(expr, req, args.no_verify, args.verbose)?
        .with_name(f.get_name().as_str());

    if let Some(p) = args.report {
        let mut writer = std::fs::File::create(p)?;
        result.write_report(&mut writer)?;
        result.print_report(&mut stderr().lock())?;
    }

    eprintln!("INFO: Writing output to Verilog...");
    let mapping = mapping.with_expr(result.get_expr().to_owned());
    mapping.rewrite(&f).map_err(std::io::Error::other)?;

    if let Some(p) = args.output {
        let mut file = std::fs::File::create(p)?;
        write!(
            file,
            "/* Generated by {} {} */\n\n{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            f
        )?;
        eprintln!("INFO: Goodbye");
    } else {
        print!("{f}");
    }

    Ok(())
}
