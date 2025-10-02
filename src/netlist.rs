/*!

  Support for nl-compiler

*/

use crate::asic::CellLang;
use crate::driver::CircuitLang;
use crate::verilog::PrimitiveType;
use egg::{Id, RecExpr, Symbol};
use safety_net::attribute::Parameter;
use safety_net::circuit::{Identifier, Instantiable, Net};
use safety_net::graph::Analysis;
use safety_net::logic::Logic;
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
    mapping: HashMap<DrivenNet<I>, Id>,
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
        let mut mapping: HashMap<DrivenNet<I>, Id> = HashMap::new();
        let mut leaves: HashMap<Symbol, DrivenNet<I>> = HashMap::new();
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
            if n.is_an_input()
                || n.clone()
                    .unwrap()
                    .get_instance_type()
                    .unwrap()
                    .get_logic_func()
                    .is_none()
            {
                let sym = n.get_identifier();
                let id = expr.add(L::var(sym.to_string().into()));
                mapping.insert(n.clone(), id);
                leaves.insert(sym.to_string().into(), n.clone());
            } else {
                let mut logic = n
                    .clone()
                    .unwrap()
                    .get_instance_type()
                    .unwrap()
                    .get_logic_func()
                    .unwrap();
                let ninputs = n
                    .clone()
                    .unwrap()
                    .get_instance_type()
                    .unwrap()
                    .get_input_ports()
                    .into_iter()
                    .count();
                for i in 0..ninputs {
                    let cid = n
                        .clone()
                        .unwrap()
                        .get_input(i)
                        .get_driver()
                        .ok_or(format!("Failed to get driver for input {} of net {}", i, n))?;
                    let cid = mapping[&cid];
                    logic.children_mut()[i] = cid;
                }

                let id = expr.add(logic);
                mapping.insert(n.clone(), id);
            }
        }

        self.mappings.insert(
            net.clone(),
            LogicMapping {
                expr: expr.clone(),
                mapping,
                root: net.clone(),
                leaves,
            },
        );

        Ok(expr)
    }

    /// Get the mapped expression
    pub fn get_expr(&self, net: &DrivenNet<I>) -> Option<RecExpr<L>> {
        self.mappings.get(net).map(|m| m.expr.clone())
    }
}

/// Create an instantiable cell out of the [PrimitiveType]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveCell {
    name: Identifier,
    ptype: PrimitiveType,
    inputs: Vec<Net>,
    outputs: Vec<Net>,
}

impl PrimitiveCell {
    /// Create a new primitive cell
    pub fn new(ptype: PrimitiveType) -> Self {
        Self {
            name: Identifier::new(ptype.to_string()),
            ptype,
            inputs: ptype
                .get_input_list()
                .into_iter()
                .map(|s| Net::new_logic(Identifier::new(s)))
                .collect(),
            outputs: vec![Net::new_logic(Identifier::new(ptype.get_output()))],
        }
    }
}

impl Instantiable for PrimitiveCell {
    fn get_name(&self) -> &Identifier {
        &self.name
    }

    fn get_input_ports(&self) -> impl IntoIterator<Item = &Net> {
        self.inputs.iter()
    }

    fn get_output_ports(&self) -> impl IntoIterator<Item = &Net> {
        self.outputs.iter()
    }

    fn has_parameter(&self, _id: &Identifier) -> bool {
        false
    }

    fn get_parameter(&self, _id: &Identifier) -> Option<safety_net::attribute::Parameter> {
        None
    }

    fn set_parameter(
        &mut self,
        _id: &Identifier,
        _val: safety_net::attribute::Parameter,
    ) -> Option<safety_net::attribute::Parameter> {
        None
    }

    fn parameters(&self) -> impl Iterator<Item = (Identifier, Parameter)> {
        std::iter::empty()
    }

    fn from_constant(val: Logic) -> Option<Self> {
        match val {
            Logic::False => Some(PrimitiveCell::new(PrimitiveType::GND)),
            Logic::True => Some(PrimitiveCell::new(PrimitiveType::VCC)),
            _ => None,
        }
    }

    fn get_constant(&self) -> Option<Logic> {
        match self.ptype {
            PrimitiveType::GND => Some(Logic::False),
            PrimitiveType::VCC => Some(Logic::True),
            _ => None,
        }
    }
}

impl LogicFunc<CellLang> for PrimitiveCell {
    fn get_logic_func(&self) -> Option<CellLang> {
        match self.ptype {
            PrimitiveType::AND => Some(CellLang::And([0.into(); 2])),
            PrimitiveType::VCC => Some(CellLang::Const(true)),
            PrimitiveType::GND => Some(CellLang::Const(false)),
            PrimitiveType::OR => Some(CellLang::Or([0.into(); 2])),
            PrimitiveType::NOT => Some(CellLang::Inv([0.into()])),
            _ => Some(CellLang::Cell(
                self.ptype.to_string().into(),
                vec![0.into(); self.ptype.get_num_inputs()],
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;

    fn and_netlist() -> Rc<Netlist<PrimitiveCell>> {
        let netlist = Netlist::new("example".to_string());

        // Add the the two inputs
        let a = netlist.insert_input("a".into());
        let b = netlist.insert_input("b".into());

        // Instantiate an AND gate
        let instance = netlist
            .insert_gate(
                PrimitiveCell::new(PrimitiveType::AND2),
                "inst_0".into(),
                &[a, b],
            )
            .unwrap();

        // Make this AND gate an output
        instance.expose_with_name("y".into());

        netlist
    }

    #[test]
    fn test_and_gate() {
        let netlist = and_netlist();
        let output = netlist.last().unwrap().get_output(0);

        let mapper = netlist.get_analysis::<'_, LogicMapper<'_, CellLang, _>>();
        assert!(mapper.is_ok());
        let mut mapper = mapper.unwrap();
        let expr = mapper.insert(output);
        assert!(expr.is_ok());
        let expr = expr.unwrap();
        assert_eq!(expr.to_string(), "(AND2 a b)");
    }
}
