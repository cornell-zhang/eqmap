use clap::Parser;
use eqmap::driver::logger_init;
use eqmap::netlist::PrimitiveCell;
use eqmap::pass::{Error, Pass, PrintVerilog};
use eqmap::register_passes;
use eqmap::verilog::sv_parse_wrapper;
use log::{info, warn};
use nl_compiler::{from_vast, from_vast_overrides};
use safety_net::graph::{CombDepthInfo, MultiDiGraph};
use safety_net::{Identifier, Instantiable, Netlist, format_id};
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

/// Rename wires and instances that are part of the feedback arc set
pub struct MarkArcSet;

impl Pass for MarkArcSet {
    type I = PrimitiveCell;

    fn run(&self, netlist: &Rc<Netlist<Self::I>>) -> Result<String, Error> {
        let mut i = 0;
        let analysis = netlist.get_analysis::<MultiDiGraph<_>>()?;

        for arc in analysis.greedy_feedback_arcs() {
            let src = arc.src().unwrap();
            let suffix = src.get_instance_name().unwrap();
            let prefix: Identifier = "arc_".into();
            src.set_instance_name(prefix + suffix);
            i += 1;
        }

        Ok(format!("Marked {i} arcs"))
    }
}

/// Rename wires and instances sequentially __0__, __1__, ...
pub struct RenameNets;

impl Pass for RenameNets {
    type I = PrimitiveCell;

    fn run(&self, netlist: &Rc<Netlist<Self::I>>) -> Result<String, Error> {
        netlist.rename_nets(|_, i| format_id!("__{i}__"))?;
        Ok(format!("Renamed {} cells", netlist.len()))
    }
}

/// Report the number of strongly connected components
pub struct ReportSccs;

impl Pass for ReportSccs {
    type I = PrimitiveCell;

    fn run(&self, netlist: &Rc<Netlist<Self::I>>) -> Result<String, Error> {
        let analysis = netlist.get_analysis::<MultiDiGraph<_>>()?;
        let sccs = analysis.sccs();
        let nt = sccs.iter().filter(|scc| scc.len() > 1).count();
        Ok(format!(
            "Netlist contains {} non-trivial strongly conncected components ({} total)",
            nt,
            sccs.len()
        ))
    }
}

/// Report the longest path in the netlist
pub struct ReportDepth;

impl Pass for ReportDepth {
    type I = PrimitiveCell;
    fn run(&self, netlist: &Rc<Netlist<Self::I>>) -> Result<String, Error> {
        let analysis = netlist.get_analysis::<CombDepthInfo<_>>()?;

        if analysis.get_max_depth().is_none() {
            return Ok("Circuit is ill-formed".to_string());
        }

        let depth = analysis.get_max_depth().unwrap();
        let mut res = format!("Maximum combinational depth is {depth}\n");

        for mut p in analysis.get_critical_points().into_iter().cloned() {
            let mut line = format!("{p}\n");
            let mut depth = "  ".to_string();
            while let Some(next) = analysis.get_crit_input(&p) {
                p = next.get_driver().unwrap().unwrap();
                line.push_str(&format!("{depth}<- {p}\n"));
                depth.push_str("  ");
            }
            line.push_str(&depth);
            line.push_str("<- INPUT\n");
            res.push_str(&line);
        }
        Ok(res)
    }
}

/// Mark the node names of cells along the critical path
pub struct MarkCriticalPath;

impl Pass for MarkCriticalPath {
    type I = PrimitiveCell;
    fn run(&self, netlist: &Rc<Netlist<Self::I>>) -> Result<String, Error> {
        let analysis = netlist.get_analysis::<CombDepthInfo<_>>()?;

        if analysis.get_max_depth().is_none() {
            return Ok("Circuit is ill-formed. No cells marked.".to_string());
        }

        let p = analysis.build_critical_path();

        if p.is_none() {
            return Ok("Circuit is ill-formed. No cells marked.".to_string());
        }

        let p = p.unwrap();
        let l = p.len();

        for c in p {
            let suffix = c.get_instance_name().unwrap();
            let prefix: Identifier = "crit_".into();
            c.set_instance_name(prefix + suffix);
        }

        Ok(format!("Marked {} cells as critical", l))
    }
}

register_passes!(PrimitiveCell; PrintVerilog, DotGraph, Clean, DisconnectRegisters,
                                DisconnectArcSet, MarkArcSet, RenameNets, ReportSccs,
                                ReportDepth, MarkCriticalPath);

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
    logger_init(false);

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

    let n = args.passes.len();

    for (i, pass) in args.passes.into_iter().enumerate() {
        info!("Running pass {i} ({pass})...");
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
                    for line in output.lines() {
                        info!("{pass}: {}", line)
                    }
                }
            }
            Err(Error::IoError(e)) => return Err(e),
            Err(e) => return Err(std::io::Error::other(e)),
        }
    }

    Ok(())
}
