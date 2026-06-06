/*!

  Abstraction for running passes on netlists.

*/

use crate::netlist::PrimitiveCell;
use safety_net::graph::{CombDepthInfo, MultiDiGraph};
use safety_net::{Error, Identifier, Instantiable, Netlist, format_id, rewriter::NetMapper};
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

/// Insert a double inverter at every net in the netlist.
#[derive(Debug)]
pub struct InsertInv;

impl fmt::Display for InsertInv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InsertInv")
    }
}

impl Pass for InsertInv {
    type I = PrimitiveCell;
    fn run(&self, netlist: &Rc<Netlist<Self::I>>) -> Result<String, Error> {
        use safety_pass::CellType;
        let inv_type = PrimitiveCell::new(CellType::INV, None);
        let mut everything = Vec::new();

        for node in netlist.objects() {
            for output in node.outputs() {
                everything.push(output);
            }
        }

        // n increases with every run of InsertInv, ensuring the net names are unique.
        let n = everything.len();

        let mut mapper = NetMapper::new(netlist)?;

        // We use i to differentiate between nets that have the same base identifer.
        for (i, net) in everything.into_iter().enumerate() {
            // Combine the net's base name (n) and i to to create unique instance names
            // across both repeated runs of this pass and nets with identical base names.
            let inst_name = net.as_net().get_identifier().clone() + format_id!("_{i}_{n}");

            let net_inv = netlist.insert_gate_disconnected(inv_type.clone(), inst_name.clone());

            // Repeat the pattern for the second inverter
            let inst_name = inst_name + "inv".into();
            let net_inv_inv =
                netlist.insert_gate(inv_type.clone(), inst_name, &[net_inv.clone().into()])?;

            // Replace the uses of the original net
            let replacement = net_inv_inv.get_output(0);
            let disconnected = mapper.replace(net, replacement);

            // Now take our disconnected net and drive the inverter pair
            net_inv.get_input(0).connect(disconnected);
        }

        mapper.apply()?;

        Ok(format!("Inserted {} pairs of inverters", n))
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
    /// Inserts a double inverter at every internal net in the graph
    InsertInv,
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
    ReportSccs,
);
