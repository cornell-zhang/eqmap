/*!

  Serialize e-graph to concise JSON format.
  Based off of https://github.com/egraphs-good/egraph-serialize

*/

use serde::Serialize;
use std::{
    collections::{HashMap, hash_map::Entry},
    io::Write,
};
type Id = usize;

/// E-Node that can be serialized
#[derive(Debug, Serialize)]
struct Node<Cost> {
    pub op: String,
    pub children: Vec<String>,
    pub eclass: String,
    pub cost: Cost,
}

impl<Cost> Node<Cost> {
    /// Create a new serializable node from an E-Node
    pub fn new<L, F>(node: &L, eclass: egg::Id, cost: Cost, mut names: F) -> Self
    where
        L: egg::Language + std::fmt::Display,
        F: FnMut(egg::Id) -> String,
    {
        let op = format!("{}", node);
        let mut children: Vec<String> = Vec::new();
        for &child in node.children() {
            children.push(names(child));
        }
        Self {
            op,
            children,
            eclass: eclass.to_string(),
            cost,
        }
    }
}

/// E-Graph that can be serialized
#[derive(Debug, Serialize)]
struct SerialEGraph<Cost> {
    nodes: HashMap<String, Node<Cost>>,
    root_eclasses: Vec<String>,
    class_data: HashMap<Id, String>,
}

impl<Cost> SerialEGraph<Cost>
where
    Cost: Serialize + Default,
{
    /// Create a new serializable E-Graph from an E-Graph
    pub fn new<L, A, M>(egraph: &egg::EGraph<L, A>, roots: &[egg::Id], mut model: M) -> Self
    where
        L: egg::Language + std::fmt::Display,
        A: egg::Analysis<L>,
        M: egg::CostFunction<L, Cost = Cost>,
    {
        let mut nodes = HashMap::new();
        let mut node_names: HashMap<egg::Id, String> = HashMap::new();
        let mut class_data = HashMap::new();

        let mut node_count: usize = 0;
        let mut node_name = |id, unique| {
            let name = format!("node{}", node_count);
            if let Entry::Vacant(e) = node_names.entry(id) {
                e.insert(name);
                node_count += 1;
                node_names[&id].clone()
            } else if unique {
                node_count += 1;
                name
            } else {
                node_names[&id].clone()
            }
        };

        for class in egraph.classes() {
            for node in class.iter() {
                let name = node_name(class.id, true);
                let cost = model.cost(node, |_id| M::Cost::default());
                nodes.insert(
                    name,
                    Node::new(node, class.id, cost, |id| node_name(id, false)),
                );
            }
            class_data.insert(class.id.into(), format!("{:?}", class.data));
        }

        let mut root_eclasses: Vec<String> = Vec::new();
        for &r in roots {
            root_eclasses.push(egraph.find(r).to_string());
        }
        Self {
            nodes,
            root_eclasses,
            class_data,
        }
    }

    /// Write the report of the output to a writer.
    pub fn write(&self, w: &mut impl Write) -> std::io::Result<()> {
        serde_json::to_writer_pretty(w, self)?;
        Ok(())
    }
}

/// Serialize an E-Graph to a writer
pub fn serialize_egraph<L, A, Cost, M>(
    egraph: &egg::EGraph<L, A>,
    roots: &[egg::Id],
    costs: M,
    w: &mut impl Write,
) -> std::io::Result<()>
where
    L: egg::Language + std::fmt::Display,
    A: egg::Analysis<L>,
    Cost: Serialize + Default,
    M: egg::CostFunction<L, Cost = Cost>,
{
    let serial_egraph = SerialEGraph::new(egraph, roots, costs);
    serial_egraph.write(w)
}
