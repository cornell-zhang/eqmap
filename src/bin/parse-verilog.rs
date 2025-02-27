use std::{
    io::{stdin, Read},
    path::PathBuf,
};

use clap::Parser;
use lut_synth::{
    lut::verify_expr,
    verilog::{sv_parse_wrapper, SVModule},
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
    #[arg(short = 'a', long, default_value_t = false)]
    dump_ast: bool,
    /// Get a separate expression for each output
    #[arg(short = 'm', long, default_value_t = false)]
    multiple_expr: bool,
    /// Print the parsed module as a data structure
    #[arg(short = 'v', long, default_value_t = false)]
    verbose: bool,
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
            let exprs = f
                .get_exprs()
                .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;
            for (y, expr) in exprs {
                verify_expr(&expr)
                    .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;
                eprintln!("{}: {}", y, expr);
                println!("{}", expr);
            }
        } else {
            let expr = f
                .to_single_expr()
                .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;
            verify_expr(&expr).map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;
            eprintln!("{:?}", f.get_outputs());
            println!("{}", expr);
        }
    } else {
        println!("{}", f);
    }

    Ok(())
}
