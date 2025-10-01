/*!

  Support for nl-compiler

*/

use crate::driver::CircuitLang;
use egg::{Id, RecExpr, Symbol};
use safety_net::circuit::Instantiable;
use safety_net::graph::Analysis;
use safety_net::netlist::iter::DFSIterator;
use safety_net::netlist::{DrivenNet, Netlist};
use std::collections::HashMap;

/// Trait for circuit elements that can provide a logic function
pub trait LogicFunc<L: CircuitLang> {
    /// Get the logic function associated with this element.
    /// The children IDs are invalid.
    fn get_logic_func(&self) -> Option<L>;
}

/// Maps a circuit element to its expression, root, and leaf mappings
struct LogicMapping<L: CircuitLang, I: Instantiable + LogicFunc<L>> {
    expr: RecExpr<L>,
    mapping: HashMap<Id, DrivenNet<I>>,
    root: DrivenNet<I>,
    leaves: HashMap<Symbol, DrivenNet<I>>,
}

/// Extracts the logic equation from a portion of a netlist.
pub struct LogicMapper<'a, L: CircuitLang, I: Instantiable + LogicFunc<L>> {
    _netlist: &'a Netlist<I>,
    mappings: HashMap<DrivenNet<I>, LogicMapping<L, I>>,
}

impl<'a, L, I> Analysis<'a, I> for LogicMapper<'a, L, I>
where
    L: CircuitLang + 'a,
    I: Instantiable + LogicFunc<L> + 'a,
{
    fn build(netlist: &'a Netlist<I>) -> Result<Self, safety_net::error::Error> {
        Ok(Self {
            _netlist: netlist,
            mappings: HashMap::new(),
        })
    }
}

impl<'a, L: CircuitLang, I: Instantiable + LogicFunc<L>> LogicMapper<'a, L, I> {
    /// Add a mapping for a specific net
    pub fn insert(&mut self, net: DrivenNet<I>) -> Result<RecExpr<L>, String> {
        let mut expr = RecExpr::<L>::default();
        let mut mapping: HashMap<Id, DrivenNet<I>> = HashMap::new();
        let mut dfs = DFSIterator::new(self._netlist, net.clone().unwrap());
        let mut nodes: Vec<DrivenNet<I>> = Vec::new();
        while let Some(n) = dfs.next() {
            if dfs.check_cycles() {
                return Err("Cycle detected in netlist".to_string());
            }
            nodes.push(n.into());
        }
        nodes.reverse();

        for n in nodes {
            if n.is_an_input() {
            } else {
                let logic = n.unwrap().get_instance_type().unwrap().get_logic_func();
                todo!(" Need L::input(symb");
            }
        }

        Ok(expr)
    }

    /// Get the mapped expression
    pub fn get_expr(&self, net: &DrivenNet<I>) -> Option<RecExpr<L>> {
        self.mappings.get(net).map(|m| m.expr.clone())
    }
}
