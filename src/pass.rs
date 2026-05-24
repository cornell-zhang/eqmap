/*!

  Abstraction for running passes on netlists.

*/

use crate::netlist::PrimitiveCell;
use safety_net::graph::{CombDepthInfo, MultiDiGraph};
use safety_net::{Error, Identifier, Instantiable, Netlist};
use safety_pass::{
    Pass,
    passes::{Clean, DotGraph, PrintVerilog, RenameNets},
    register_passes,
};
use std::{fmt, rc::Rc};

/// Disconnect all register inputs
#[derive(Debug)]
pub struct DisconnectRegisters;

impl fmt::Display for DisconnectRegisters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DisconnectRegisters")
    }
}

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
#[derive(Debug)]
pub struct DisconnectArcSet;

impl fmt::Display for DisconnectArcSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DisconnectArcSet")
    }
}

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
#[derive(Debug)]
pub struct MarkArcSet;

impl fmt::Display for MarkArcSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MarkArcSet")
    }
}

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

/// Report the number of strongly connected components
#[derive(Debug)]
pub struct ReportSccs;

impl fmt::Display for ReportSccs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReportSccs")
    }
}

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
#[derive(Debug)]
pub struct ReportDepth;

impl fmt::Display for ReportDepth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReportDepth")
    }
}

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
#[derive(Debug)]
pub struct MarkCriticalPath;

impl fmt::Display for MarkCriticalPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MarkCriticalPath")
    }
}

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

register_passes!(Passes<PrimitiveCell>;
    /// Clean the netlist of cells which are not used
    Clean<PrimitiveCell>,
    /// Disconnect all register inputs
    DisconnectRegisters,
    /// Disconnect wires based on greedy arc set heuristic, creating a DAG
    DisconnectArcSet,
    /// Print the dot graph of the netlist
    DotGraph<PrimitiveCell>,
    /// Rename wires and instances that are part of the feedback arc set (prefixed with "arc_")
    MarkArcSet,
    /// Mark the node names of cells along the critical path (prefixed with "crit_")
    MarkCriticalPath,
    /// A dummy pass that emits the Verilog of the netlist.
    PrintVerilog<PrimitiveCell>,
    /// Rename wires and instances sequentially 0, 1, ...
    RenameNets<PrimitiveCell>,
    /// Report the longest path in the netlist
    ReportDepth,
    /// Report the number of strongly connected components
    ReportSccs);
