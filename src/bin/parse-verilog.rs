use std::{
    io::{Read, stdin},
    path::PathBuf,
};

use clap::Parser;
use lut_synth::{asic::CellLang, driver::Canonical, lut::LutLang, verilog::VerilogParsing};
use lut_synth::{
    driver::CircuitLang,
    verilog::{SVModule, sv_parse_wrapper},
};
/// Parse structural verilog into a LutLang Expression
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to input file. If not provided, reads from stdin
    input: Option<PathBuf>,
    /// Convert from and to verilog
    #[arg(short = 'r', long, default_value_t = false)]
    round_trip: bool,
    /// Dump ast
    #[arg(short = 'd', long, default_value_t = false)]
    dump_ast: bool,
    /// Parse Verilog as CellLang
    #[arg(short = 'a', long, default_value_t = false)]
    asic: bool,
    /// Get a separate expression for each output
    #[arg(short = 'm', long, default_value_t = false)]
    multiple_expr: bool,
    /// Print the parsed module as a data structure
    #[arg(short = 'v', long, default_value_t = false)]
    verbose: bool,
}

fn emit_exprs<L: CircuitLang + VerilogParsing>(f: &SVModule) -> std::io::Result<()> {
    let exprs = f
        .get_exprs()
        .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;
    for (y, expr) in exprs {
        L::verify_expr(&expr).map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;
        eprintln!("{}: {}", y, expr);
        println!("{}", expr);
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
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

    if args.dump_ast {
        println!("{}", ast);
        return Ok(());
    }

    let f =
        SVModule::from_ast(&ast).map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;

    if args.verbose {
        eprintln!("SVModule: ");
        eprintln!("{:?}", f);
    }

    if !args.round_trip {
        if args.multiple_expr {
            if args.asic {
                emit_exprs::<CellLang>(&f)?;
            } else {
                emit_exprs::<LutLang>(&f)?;
            }
        } else if args.asic {
            let expr = f
                .to_single_cell_expr()
                .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;
            eprintln!("{:?}", f.get_outputs());
            println!("{}", expr);
        } else {
            let expr = f
                .to_single_lut_expr()
                .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;
            LutLang::verify_expr(&expr)
                .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;
            eprintln!("{:?}", f.get_outputs());
            println!("{}", expr);
        }
    } else {
        print!("{}", f);
    }

    Ok(())
}
