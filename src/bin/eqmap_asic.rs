use clap::Parser;
#[cfg(any(feature = "exact_cbc", feature = "exact_highs"))]
use clap::ValueEnum;
use eqmap::{
    asic::{CellAnalysis, CellLang, CellRpt, expansion_rewrites, expr_is_mapped},
    driver::{SynthRequest, process_expression},
    netlist::{LogicMapper, PrimitiveCell},
    rewrite::RewriteManager,
    verilog::sv_parse_wrapper,
};
use nl_compiler::from_vast;
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

/// EqMap: ASIC Technology Mapping w/ E-Graphs
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

    /// If provided, use rules compiled from file instead of built-in rules
    #[arg(long)]
    rules: Option<PathBuf>,

    /// If provided, output a condensed JSON file with the e-graph
    #[cfg(feature = "graph_dumps")]
    #[arg(long)]
    dump_graph: Option<PathBuf>,

    /// Comma separated list of cell types to extract
    #[arg(long)]
    filter: Option<String>,

    /// Use a cost model that weighs the cells by exact area
    #[arg(short = 'a', long, default_value_t = false)]
    area: bool,

    /// Do not check that all cells have been mapped
    #[arg(short = 'm', long, default_value_t = false)]
    no_assert: bool,

    /// Perform an exact extraction using ILP (much slower)
    #[cfg(any(feature = "exact_cbc", feature = "exact_highs"))]
    #[arg(long, value_enum)]
    exact: Option<Solver>,

    /// Print explanations (generates a proof and runs slower)
    #[arg(short = 'v', long, default_value_t = false)]
    verbose: bool,

    /// Extract for minimum circuit depth
    #[arg(long, default_value_t = false)]
    min_depth: bool,

    /// Max fan in size allowed for extracted Cells
    #[arg(short = 'k', long, default_value_t = 6)]
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

    eprintln!("INFO: ASIC Technology Mapping Optimization with E-Graphs");

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

    let ast = sv_parse_wrapper(&buf, path).map_err(std::io::Error::other)?;

    let f = from_vast(&ast).map_err(std::io::Error::other)?;

    eprintln!(
        "INFO: Module {} has {} outputs",
        f.get_name(),
        f.get_output_ports().len()
    );

    let path = if let Some(p) = args.rules {
        p
    } else {
        let root = match std::env::var("EQMAP_ROOT") {
            Ok(root) => PathBuf::from(root),
            Err(_) => std::env::current_exe()?
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf(),
        };
        root.join("rules/asic.celllang")
    };

    eprintln!("INFO: Loading rewrite rules from {path:?}");

    let mut rules = RewriteManager::<CellLang, CellAnalysis>::new();
    let file = std::fs::File::open(path)?;
    rules.parse_rules(file).map_err(std::io::Error::other)?;
    let categories = rules.categories().cloned().collect::<Vec<_>>();
    for cat in categories {
        rules.enable_category(&cat);
    }

    if args.filter.is_some() {
        rules
            .insert_category("expansion_rewrites".to_string(), expansion_rewrites())
            .map_err(|r| std::io::Error::other(format!("Repeat rule: {:?}", r)))?;
        rules.enable_category("expansion_rewrites");
    }

    if args.verbose {
        eprintln!(
            "INFO: Running with {} rewrite rules. Hash: {}",
            rules.num_active(),
            rules.rules_hash()
        );
    }

    let rules = rules.active_rules();

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

    let req = if let Some(l) = args.filter {
        req.with_algebraic_scheduler()
            .with_purge_fn(|n| matches!(n, CellLang::And(_) | CellLang::Or(_) | CellLang::Inv(_)))
            .with_disassembly_into(&l)
            .map_err(std::io::Error::other)?
    } else if args.min_depth {
        req.with_min_depth()
    } else if args.area {
        req.with_area()
    } else {
        req.with_k(args.k)
    };

    #[cfg(any(feature = "exact_cbc", feature = "exact_highs"))]
    let req = if let Some(solver) = &args.exact {
        let timeout = args.timeout.unwrap_or(600);
        let req = match solver {
            #[cfg(feature = "exact_cbc")]
            Solver::Cbc => req.with_cbc(timeout),
            #[cfg(feature = "exact_highs")]
            Solver::Highs => req.with_highs(timeout),
        };
        req.with_purge_fn(|n| matches!(n, CellLang::And(_) | CellLang::Or(_) | CellLang::Inv(_)))
    } else {
        req
    };

    #[cfg(any(feature = "exact_cbc", feature = "exact_highs"))]
    if args.exact.is_some() && args.output.is_none() {
        return Err(std::io::Error::other(
            "Stdout is clutterd by ILP solver. Specify an output file",
        ));
    }

    eprintln!("INFO: Compiling Verilog...");
    let mut mapper = f
        .get_analysis::<LogicMapper<CellLang, PrimitiveCell>>()
        .map_err(std::io::Error::other)?;
    mapper.insert_all_r2r().map_err(std::io::Error::other)?;
    let mut mapping = mapper.mappings();
    let mapping = mapping.pop().unwrap();
    let expr = mapping.get_expr();

    eprintln!("INFO: Building e-graph...");
    let result = process_expression::<CellLang, _, CellRpt>(expr, req, true, args.verbose)?
        .with_name(f.get_name().as_str());

    if !(args.no_assert || expr_is_mapped(result.get_expr())) {
        return Err(std::io::Error::other(
            "Not all logic is mapped to cells. Run the tool for more iterations/time.",
        ));
    }

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
