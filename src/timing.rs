/*!

  Timing analysis helpers for timing-aware optimization flows.

*/

use safety_net::DrivenNet;
use safety_net::Instantiable;
use safety_net::graph::CombDepthInfo;
use std::collections::HashSet;

#[derive(Debug)]
/// A representative critical path ending at a timing endpoint.
pub struct DelayPath<I: Instantiable> {
    /// The path from endpoint backward through critical fan-in.
    path: Vec<DrivenNet<I>>,
}

impl<I: Instantiable> DelayPath<I> {
    /// Returns the depth/length of the delay path.
    pub fn depth(&self) -> usize {
        self.path.len()
    }

    /// The signal being driven by this path
    pub fn endpoint(&self) -> DrivenNet<I> {
        self.path[0].clone()
    }

    /// The nodes along the delay path as a slice
    pub fn path(&self) -> &[DrivenNet<I>] {
        &self.path
    }
    /// Expands and collects the transitive fan-in along the critical path provided by a branch factor of n.
    pub fn expand_n_nodes(&self, n: usize) -> HashSet<DrivenNet<I>> {
        let mut frontier: Vec<DrivenNet<I>> = self.path().to_vec();
        let mut expanded_nodes: HashSet<DrivenNet<I>> = frontier.iter().cloned().collect();

        for _ in 0..n {
            let mut next_frontier = Vec::new();

            for net in frontier {
                let node = net.unwrap();

                for input in node.inputs() {
                    if let Some(driver) = input.get_driver()
                        && expanded_nodes.insert(driver.clone())
                    {
                        next_frontier.push(driver);
                    }
                }
            }

            frontier = next_frontier;
        }

        expanded_nodes
    }
}

impl<I: Instantiable> IntoIterator for DelayPath<I> {
    type Item = DrivenNet<I>;
    type IntoIter = std::vec::IntoIter<DrivenNet<I>>;

    fn into_iter(self) -> Self::IntoIter {
        self.path.into_iter()
    }
}

fn build_path_from_endpoint<I: Instantiable>(
    analysis: &CombDepthInfo<'_, I>,
    endpoint: DrivenNet<I>,
) -> Option<DelayPath<I>> {
    let mut path = Vec::new();
    let mut current = endpoint;

    while let Some(crit) = analysis.get_crit_input(&current.clone().unwrap()) {
        path.push(current.clone());
        if let Some(c) = crit.get_driver() {
            current = c;
        } else {
            return None;
        }
    }

    path.push(current);
    Some(DelayPath { path })
}

/// Build the critical paths along each critical endpoint
pub fn get_critical_paths<I: Instantiable>(
    analysis: &CombDepthInfo<'_, I>,
) -> impl Iterator<Item = DelayPath<I>> {
    analysis
        .get_critical_points()
        .into_iter()
        .flat_map(|p| build_path_from_endpoint(analysis, p))
}
