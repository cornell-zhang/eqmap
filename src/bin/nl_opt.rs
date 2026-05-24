use clap::Parser;
use eqmap::netlist::PrimitiveCell;
use eqmap::pass::Passes;
use log::{info, warn};
use nl_compiler::{from_vast, from_vast_overrides};
use safety_net::Identifier;
use safety_pass::Pipeline;
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Netlist optimization debugging tool
#[derive(Parser, Debug)]
#[command(version, long_about = None)]
struct Args {
    /// Verilog file to read from (or use stdin)
    input: Option<PathBuf>,

    /// Do not parse with Xilinx-specific port names
    #[arg(short = 'x', long, default_value_t = false)]
    no_xilinx: bool,

    /// Verify after every pass (not just the last)
    #[arg(long, default_value_t = false)]
    verify: bool,

    /// Verbose logging
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// A list of passes to run in order
    #[arg(value_delimiter = ',', short = 'p', long, value_enum)]
    passes: Vec<Passes>,
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

/// Initializes the logger
fn logger_init(verbose: bool) {
    let level = if verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    let config = ConfigBuilder::new()
        .add_filter_ignore_str("egg")
        .set_thread_level(log::LevelFilter::Off)
        .build();
    TermLogger::init(level, config, TerminalMode::Stderr, ColorChoice::Auto).unwrap();
}

/// A wrapper for parsing verilog at file `path` with content `s`
fn sv_parse_wrapper(
    s: &str,
    path: Option<PathBuf>,
) -> Result<sv_parser::SyntaxTree, sv_parser::Error> {
    let incl: Vec<std::path::PathBuf> = vec![];
    let path = path.unwrap_or(Path::new("top.v").to_path_buf());
    match sv_parser::parse_sv_str(s, path, &HashMap::new(), &incl, true, false) {
        Ok((ast, _defs)) => Ok(ast),
        Err(e) => Err(e),
    }
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    logger_init(args.verbose);

    if cfg!(debug_assertions) {
        warn!("Debug assertions are enabled");
    }

    info!("Netlist optimization debugging tool");

    let mut buf = String::new();

    let path: Option<PathBuf> = match args.input {
        Some(p) => {
            std::fs::File::open(&p)?.read_to_string(&mut buf)?;
            Some(p)
        }
        None => {
            info!("Reading from stdin...");
            std::io::stdin().read_to_string(&mut buf)?;
            None
        }
    };

    info!("Parsing Verilog...");
    let ast = sv_parse_wrapper(&buf, path).map_err(std::io::Error::other)?;

    info!("Compiling Verilog...");
    let f = if !args.no_xilinx {
        from_vast_overrides(&ast, xilinx_overrides).map_err(std::io::Error::other)?
    } else {
        from_vast(&ast).map_err(std::io::Error::other)?
    };

    let mut pipeline = Pipeline::default();

    for pass in args.passes {
        pipeline.insert_dyn(pass.get_pass());
    }

    let output = pipeline
        .execute(&f, args.verify)
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    println!("{output}");

    Ok(())
}
