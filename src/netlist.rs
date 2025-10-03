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
    /// Get the logic function/variant associated with the output at position `ind`.
    /// The children IDs are invalid/nulled in the returned [CircuitLang].
    fn get_logic_func(&self, ind: usize) -> Option<L>;
}

/// Maps a circuit element to its expression, root, and leaf mappings
#[derive(Debug, Clone)]
pub struct LogicMapping<L: CircuitLang, I: Instantiable + LogicFunc<L>> {
    expr: RecExpr<L>,
    root: DrivenNet<I>,
    leaves: HashMap<Symbol, DrivenNet<I>>,
    leaves_by_id: HashMap<Id, DrivenNet<I>>,
}

impl<L: CircuitLang, I: Instantiable + LogicFunc<L>> LogicMapping<L, I> {
    /// Get the expression
    pub fn get_expr(&self) -> RecExpr<L> {
        self.expr.clone()
    }

    /// Returns the circuit node at the root of this expression
    pub fn root_net(&self) -> DrivenNet<I> {
        self.root.clone()
    }

    /// Returns the Id of the root of the expression
    pub fn root_id(&self) -> Id {
        (self.expr.as_ref().len() - 1).into()
    }

    /// Returns the driven net associated with the variable leaf called `sym`
    pub fn get_leaf(&self, sym: &Symbol) -> Option<DrivenNet<I>> {
        self.leaves.get(sym).cloned()
    }

    /// Returns the driven net associated with the variable leaf with id `id` in the expressions
    pub fn get_leaf_by_id(&self, id: &Id) -> Option<DrivenNet<I>> {
        self.leaves_by_id.get(id).cloned()
    }
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
        if net.is_an_input() {
            return Err("Inputs have trivial mappings".to_string());
        }

        if net
            .get_instance_type()
            .unwrap()
            .get_logic_func(net.get_output_index().unwrap())
            .is_none()
        {
            return Err(format!(
                "Root instance type {} does not have a logic function",
                net.get_instance_type().unwrap().get_name()
            ));
        }

        let mut expr = RecExpr::<L>::default();
        let mut mapping: HashMap<DrivenNet<I>, Id> = HashMap::new();
        let mut leaves: HashMap<Symbol, DrivenNet<I>> = HashMap::new();
        let mut leaves_by_id: HashMap<Id, DrivenNet<I>> = HashMap::new();
        let mut dfs = DFSIterator::new(self._netlist, net.clone().unwrap());
        let mut nodes: Vec<DrivenNet<I>> = Vec::new();
        while let Some(n) = dfs.next() {
            if dfs.check_cycles() {
                return Err("Cycle detected in netlist".to_string());
            }
            if n.is_multi_output() {
                // TODO(matth2k): safety-net should have dfs by [DrivenNet]
                return Err("Cannot map multi-output cells".to_string());
            }
            nodes.push(n.into());
        }
        nodes.reverse();

        for n in nodes {
            if let Some(inst_type) = n.get_instance_type()
                && let Some(mut logic) = inst_type.get_logic_func(n.get_output_index().unwrap())
            {
                for (i, c) in n.clone().unwrap().inputs().enumerate() {
                    let cid = c
                        .get_driver()
                        .ok_or(format!("Failed to get driver for input {} of net {}", i, n))?;
                    let cid = mapping[&cid];
                    logic.children_mut()[i] = cid;
                }

                let id = expr.add(logic);
                mapping.insert(n.clone(), id);
            } else {
                let sym = n.get_identifier();
                let id = expr.add(L::var(sym.to_string().into()));
                mapping.insert(n.clone(), id);
                leaves.insert(sym.to_string().into(), n.clone());
                leaves_by_id.insert(id, n.clone());
            }
        }

        self.mappings.insert(
            net.clone(),
            LogicMapping {
                expr: expr.clone(),
                root: net.clone(),
                leaves,
                leaves_by_id,
            },
        );

        Ok(expr)
    }

    /// Get the mapped expression
    pub fn get(&self, net: &DrivenNet<I>) -> Option<LogicMapping<L, I>> {
        self.mappings.get(net).cloned()
    }
}

/// Create an instantiable cell out of the [PrimitiveType]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveCell {
    name: Identifier,
    ptype: PrimitiveType,
    inputs: Vec<Net>,
    outputs: Vec<Net>,
    params: HashMap<Identifier, Parameter>,
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
            params: HashMap::new(),
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

    fn has_parameter(&self, id: &Identifier) -> bool {
        self.params.contains_key(id)
    }

    fn get_parameter(&self, id: &Identifier) -> Option<Parameter> {
        self.params.get(id).cloned()
    }

    fn set_parameter(&mut self, id: &Identifier, val: Parameter) -> Option<Parameter> {
        self.params.insert(id.clone(), val)
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
    fn get_logic_func(&self, _ind: usize) -> Option<CellLang> {
        match self.ptype {
            PrimitiveType::AND => Some(CellLang::And([0.into(); 2])),
            PrimitiveType::VCC => Some(CellLang::Const(true)),
            PrimitiveType::GND => Some(CellLang::Const(false)),
            PrimitiveType::OR => Some(CellLang::Or([0.into(); 2])),
            PrimitiveType::NOT => Some(CellLang::Inv([0.into()])),
            _ if self.ptype.is_lut() => None,
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

    fn and_gate() -> PrimitiveCell {
        PrimitiveCell::new(PrimitiveType::AND2)
    }

    fn and_netlist() -> Rc<Netlist<PrimitiveCell>> {
        let netlist = Netlist::new("example".to_string());

        // Add the the two inputs
        let a = netlist.insert_input("a".into());
        let b = netlist.insert_input("b".into());

        // Instantiate an AND gate
        let instance = netlist
            .insert_gate(and_gate(), "inst_0".into(), &[a, b])
            .unwrap();

        // Make this AND gate an output
        instance.expose_with_name("y".into());

        netlist
    }

    fn and_const_netlist() -> Rc<Netlist<PrimitiveCell>> {
        let netlist = Netlist::new("example".to_string());

        // Add the the two inputs
        let a = netlist.insert_constant(Logic::True, "a".into()).unwrap();
        let b = netlist.insert_constant(Logic::False, "a".into()).unwrap();

        // Instantiate an AND gate
        let instance = netlist
            .insert_gate(and_gate(), "inst_0".into(), &[a, b])
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

        // Check the RecExpr is correct
        let expr = mapper.insert(output.clone());
        assert!(expr.is_ok());
        let expr = expr.unwrap();
        assert_eq!(expr.to_string(), "(AND2 a b)");

        // Check the root properties are correct
        let mapping = mapper.get(&output);
        assert!(mapping.is_some());
        let mapping = mapping.unwrap();
        assert_eq!(mapping.root_net(), output);
        assert_eq!(netlist.objects().count(), mapping.get_expr().as_ref().len());

        // Check the leaves
        let l0 = mapping.get_leaf(&"a".into());
        let l1 = mapping.get_leaf_by_id(&1.into());
        assert!(l0.is_some());
        assert!(l1.is_some());
        let l0 = l0.unwrap();
        let l1 = l1.unwrap();
        assert_eq!(l0, netlist.first().unwrap().into());
        assert_eq!(l1.to_string(), "b");
    }

    #[test]
    fn test_consts() {
        let netlist = and_const_netlist();
        let output = netlist.last().unwrap().get_output(0);

        let mapper = netlist.get_analysis::<'_, LogicMapper<'_, CellLang, _>>();
        assert!(mapper.is_ok());
        let mut mapper = mapper.unwrap();

        // Check the RecExpr is correct
        let expr = mapper.insert(output.clone());
        assert!(expr.is_ok());
        let expr = expr.unwrap();
        assert_eq!(expr.to_string(), "(AND2 true false)");
    }
}
