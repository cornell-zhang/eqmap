use clap::Parser;
use eqmap::netlist::PrimitiveCell;
use eqmap::pass::{Error, Pass, PrintVerilog};
use eqmap::register_passes;
use eqmap::verilog::sv_parse_wrapper;
use nl_compiler::{from_vast, from_vast_overrides};
use safety_net::{Identifier, Instantiable, MultiDiGraph, Netlist};
use std::io::Read;
use std::path::PathBuf;
use std::rc::Rc;

/// Print the dot graph of the netlist
pub struct DotGraph;

impl Pass for DotGraph {
    type I = PrimitiveCell;

    fn run(&self, netlist: &Rc<Netlist<Self::I>>) -> Result<String, Error> {
        Ok(netlist.dot_string()?)
    }
}

/// Clean the netlist
pub struct Clean;

impl Pass for Clean {
    type I = PrimitiveCell;

    fn run(&self, netlist: &Rc<Netlist<Self::I>>) -> Result<String, Error> {
        let cleaned = netlist.clean()?;
        Ok(format!(
            "Cleaned {} objects. {} remain.",
            cleaned.len(),
            netlist.len()
        ))
    }
}

/// Disconnect all register inputs
pub struct DisconnectRegisters;

impl Pass for DisconnectRegisters {
    type I = PrimitiveCell;

    fn run(&self, netlist: &Rc<Netlist<Self::I>>) -> Result<String, Error> {
        let mut i = 0;

        for reg in netlist.matches(|i| i.is_seq()) {
            let mut disc = false;
            for input in reg.inputs() {
                disc |= input.disconnect().is_some();
            }
            if disc {
                i += 1;
            }
        }

        Ok(format!("Disconnected {i} registers"))
    }
}

/// Disconnect wires based on greedy arc set heuristic
pub struct DisconnectArcSet;

impl Pass for DisconnectArcSet {
    type I = PrimitiveCell;

    fn run(&self, netlist: &Rc<Netlist<Self::I>>) -> Result<String, Error> {
        let mut i = 0;
        let analysis = netlist.get_analysis::<MultiDiGraph<_>>()?;

        for arc in analysis.greedy_feedback_arcs() {
            arc.target().disconnect();
            i += 1;
        }

        Ok(format!("Disconnected {i} arcs"))
    }
}

register_passes!(PrimitiveCell; PrintVerilog, DotGraph, Clean, DisconnectRegisters, DisconnectArcSet);

/// Netlist optimization debugging tool
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Verilog file to read from (or use stdin)
    input: Option<PathBuf>,

    /// Do not parse with Xilinx-specific port names
    #[arg(short = 'x', long, default_value_t = false)]
    no_xilinx: bool,

    /// Verify after every pass (not just the last)
    #[arg(short = 'v', long, default_value_t = false)]
    verify: bool,

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

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    if cfg!(debug_assertions) {
        eprintln!("WARNING: Debug assertions are enabled");
    }

    eprintln!("INFO: Netlist optimization debugging tool");

    let mut buf = String::new();

    let path: Option<PathBuf> = match args.input {
        Some(p) => {
            std::fs::File::open(&p)?.read_to_string(&mut buf)?;
            Some(p)
        }
        None => {
            eprintln!("INFO: Reading from stdin...");
            std::io::stdin().read_to_string(&mut buf)?;
            None
        }
    };

    eprintln!("INFO: Parsing Verilog...");
    let ast = sv_parse_wrapper(&buf, path).map_err(std::io::Error::other)?;

    eprintln!("INFO: Compiling Verilog...");
    let f = if !args.no_xilinx {
        from_vast_overrides(&ast, xilinx_overrides).map_err(std::io::Error::other)?
    } else {
        from_vast(&ast).map_err(std::io::Error::other)?
    };

    let n = args.passes.len();

    for (i, pass) in args.passes.into_iter().enumerate() {
        eprintln!("INFO: Running pass {i} ({pass})...");
        let pass_instance = pass.get_pass();
        match pass_instance.run(&f) {
            Ok(output) => {
                if i == n - 1 {
                    f.verify().map_err(std::io::Error::other)?;
                    println!("{}", output)
                } else {
                    if args.verify {
                        f.verify().map_err(std::io::Error::other)?;
                    }
                    eprintln!("INFO: {pass}: {}", output)
                }
            }
            Err(Error::IoError(e)) => return Err(e),
            Err(e) => return Err(std::io::Error::other(e)),
        }
    }

    Ok(())
}
