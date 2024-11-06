use std::{
    io::{stdin, Read},
    path::PathBuf,
};

use clap::Parser;
use lut_synth::verilog::{sv_parse_wrapper, SVModule};
/// Parse structural verilog into a LutLang Expression
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to input file. If not provided, reads from stdin
    input: Option<PathBuf>,
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

    let f =
        SVModule::from_ast(&ast).map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;

    let expr = f
        .to_expr()
        .map_err(|s| std::io::Error::new(std::io::ErrorKind::Other, s))?;

    println!("{}", expr);

    Ok(())
}
