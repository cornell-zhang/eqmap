use std::{
    io::{Read, stdin},
    path::PathBuf,
};

use clap::Parser;
use eqmap::{asic::CellLang, driver::Canonical, lut::LutLang, verilog::VerilogParsing};
use eqmap::{
    driver::CircuitLang,
    netlist::{LogicMapper, PrimitiveCell},
    verilog::{SVModule, sv_parse_wrapper},
};
use nl_compiler::{from_vast, from_vast_overrides};
use safety_net::Identifier;
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

fn emit_exprs<L: CircuitLang + VerilogParsing>(f: &SVModule) -> std::io::Result<()> {
    let exprs = f.get_exprs().map_err(std::io::Error::other)?;
    for (y, expr) in exprs {
        L::verify_expr(&expr).map_err(std::io::Error::other)?;
        eprintln!("{y}: {expr}");
        println!("{expr}");
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

    let ast = sv_parse_wrapper(&buf, path).map_err(std::io::Error::other)?;

    if args.dump_ast {
        println!("{ast}");
        return Ok(());
    }

    let f = if args.asic {
        from_vast(&ast).map_err(std::io::Error::other)?
    } else {
        from_vast_overrides(&ast, xilinx_overrides).map_err(std::io::Error::other)?
    };

    if args.verbose {
        eprintln!("SVModule: ");
        eprintln!("{f:?}");
    }

    if !args.round_trip {
        if args.multiple_expr {
            eprintln!("{}", &f);
        } else {
            if args.asic {
                let mut mapper = f
                    .get_analysis::<LogicMapper<CellLang, PrimitiveCell>>()
                    .map_err(std::io::Error::other)?;
                mapper
                    .insert(f.outputs().into_iter().map(|x| x.0).collect())
                    .map_err(std::io::Error::other)?;
                let mut mapping = mapper.mappings();
                let mapping = mapping.pop().unwrap();
                let expr = mapping.get_expr();
                eprintln!("{:?}", f.outputs());
                eprintln!("{expr}");
            } else {
                let mut mapper = f
                    .get_analysis::<LogicMapper<LutLang, PrimitiveCell>>()
                    .map_err(std::io::Error::other)?;
                mapper
                    .insert(f.outputs().into_iter().map(|x| x.0).collect())
                    .map_err(std::io::Error::other)?;
                let mut mapping = mapper.mappings();
                let mapping = mapping.pop().unwrap();
                let expr = mapping.get_expr();
                eprintln!("{:?}", f.outputs());
                eprintln!("{expr}");
            }
        }
    } else {
        eprintln!("{f}");
    }

    Ok(())
}
