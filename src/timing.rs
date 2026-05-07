/*!

  Timing analysis helpers for timing-aware optimization flows.

*/

use std::collections::HashSet;

use safety_net::graph::{CombDepthInfo, CombDepthResult};
use safety_net::{Instantiable, NetRef, Netlist};

use crate::netlist::PrimitiveCell;

#[derive(Debug, Clone, PartialEq, Eq)]
/// A representative critical path ending at a timing endpoint.
pub struct CriticalPath {
    /// The output or register-input driver where this path ends.
    pub endpoint: NetRef<PrimitiveCell>,
    /// The combinational depth at the endpoint.
    pub depth: usize,
    /// The path from endpoint backward through critical fan-in.
    pub path: Vec<NetRef<PrimitiveCell>>,
}

fn build_path_from_endpoint(
    analysis: &CombDepthInfo<'_, PrimitiveCell>,
    endpoint: NetRef<PrimitiveCell>,
) -> Result<Vec<NetRef<PrimitiveCell>>, std::io::Error> {
    let mut path = Vec::new();
    let mut current = endpoint;

    while let Some(crit) = analysis.get_crit_input(&current) {
        path.push(current.clone());
        current = crit
            .get_driver()
            .ok_or_else(|| std::io::Error::other("Circuit is ill-formed"))?
            .unwrap();
    }

    path.push(current);
    Ok(path)
}

/// Gets one critical path from the combinational-depth analysis.
pub fn get_critical_path(
    netlist: &Netlist<PrimitiveCell>,
) -> Result<Vec<NetRef<PrimitiveCell>>, std::io::Error> {
    let analysis = netlist
        .get_analysis::<CombDepthInfo<_>>()
        .map_err(std::io::Error::other)?;

    if analysis.get_max_depth().is_none() {
        return Err(std::io::Error::other("Circuit is ill-formed"));
    }

    let path = analysis
        .build_critical_path()
        .ok_or_else(|| std::io::Error::other("Circuit is ill-formed"))?;

    Ok(path)
}

/// Gets up to `n` critical paths ending at outputs or register inputs.
pub fn get_critical_paths(
    netlist: &Netlist<PrimitiveCell>,
    n: usize,
) -> Result<Vec<CriticalPath>, std::io::Error> {
    let analysis = netlist
        .get_analysis::<CombDepthInfo<_>>()
        .map_err(std::io::Error::other)?;

    if analysis.get_max_depth().is_none() {
        return Err(std::io::Error::other("Circuit is ill-formed"));
    }

    let mut endpoints = HashSet::new();

    for (driver, _) in netlist.outputs() {
        endpoints.insert(driver.unwrap());
    }

    for reg in netlist.matches(|inst| inst.is_seq()) {
        for driver in reg.drivers().flatten() {
            if !driver.get_instance_type().is_some_and(|inst| inst.is_seq()) {
                endpoints.insert(driver);
            }
        }
    }

    let mut endpoints: Vec<(NetRef<PrimitiveCell>, usize)> = endpoints
        .into_iter()
        .filter_map(|endpoint| match analysis.get_comb_depth(&endpoint) {
            Some(CombDepthResult::Depth(depth)) => Some((endpoint, depth)),
            _ => None,
        })
        .collect::<Vec<_>>();

    endpoints.sort_by(|(a_endpoint, a_depth), (b_endpoint, b_depth)| {
        b_depth
            .cmp(a_depth)
            .then_with(|| a_endpoint.cmp(b_endpoint))
    });

    endpoints
        .into_iter()
        .take(n)
        .map(|(endpoint, depth)| {
            let path = build_path_from_endpoint(&analysis, endpoint.clone())?;
            Ok(CriticalPath {
                endpoint,
                depth,
                path,
            })
        })
        .collect()
}

/// Expands a critical path backward through fan-in for `n` frontier steps.
pub fn expand_n_nodes(
    critical_path: &[NetRef<PrimitiveCell>],
    n: usize,
) -> Result<HashSet<NetRef<PrimitiveCell>>, std::io::Error> {
    let mut expanded_nodes: HashSet<NetRef<PrimitiveCell>> =
        critical_path.iter().cloned().collect();
    let mut frontier: Vec<NetRef<PrimitiveCell>> = critical_path.to_owned();

    for _ in 0..n {
        let mut next_frontier = Vec::new();

        for node in frontier {
            for driver in node.drivers().flatten() {
                if expanded_nodes.insert(driver.clone()) {
                    next_frontier.push(driver);
                }
            }
        }

        frontier = next_frontier;
    }

    Ok(expanded_nodes)
}
