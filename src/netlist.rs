/*!

  Support for nl-compiler

*/

use crate::asic::CellLang;
use crate::driver::CircuitLang;
use crate::verilog::PrimitiveType;
use egg::{Id, RecExpr, Symbol};
use safety_net::attribute::Parameter;
use safety_net::circuit::{Identifier, Instantiable, Net};
use safety_net::error::Error;
use safety_net::format_id;
use safety_net::graph::Analysis;
use safety_net::logic::Logic;
use safety_net::netlist::iter::DFSIterator;
use safety_net::netlist::{DrivenNet, Netlist};
use std::collections::HashMap;
use std::rc::Rc;
use std::str::FromStr;

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
    roots: Vec<DrivenNet<I>>,
    leaves: HashMap<Symbol, DrivenNet<I>>,
    leaves_by_id: HashMap<Id, DrivenNet<I>>,
}

impl<L: CircuitLang, I: Instantiable + LogicFunc<L>> LogicMapping<L, I> {
    /// Get the expression
    pub fn get_expr(&self) -> RecExpr<L> {
        self.expr.clone()
    }

    /// Returns true if multiple nets are mapped
    pub fn is_multi_mapping(&self) -> bool {
        self.roots.len() > 1
    }

    /// Returns the circuit nodes at the root of this expression
    pub fn root_nets(&self) -> impl Iterator<Item = DrivenNet<I>> {
        self.roots.clone().into_iter()
    }

    /// Returns the Ids of the roots of the expression
    pub fn root_ids(&self) -> impl Iterator<Item = Id> {
        let last = self.expr.last().unwrap();
        if last.is_bus() {
            last.children().to_vec().into_iter()
        } else {
            let id: Id = (self.expr.len() - 1).into();
            let id = vec![id];
            id.into_iter()
        }
    }

    /// Returns the driven net associated with the variable leaf called `sym`
    pub fn get_leaf(&self, sym: &Symbol) -> Option<DrivenNet<I>> {
        self.leaves.get(sym).cloned()
    }

    /// Returns the driven net associated with the variable leaf with id `id` in the expressions
    pub fn get_leaf_by_id(&self, id: &Id) -> Option<DrivenNet<I>> {
        self.leaves_by_id.get(id).cloned()
    }

    /// Replaces the expression with a rewritten one
    ///
    /// # Panics
    /// Panics if the new expression does not have the same number of roots as the old one
    pub fn with_expr(self, expr: RecExpr<L>) -> Self {
        if self.expr.last().unwrap().is_bus() != expr.last().unwrap().is_bus() {
            panic!("New expression must have the same number of roots as the old one");
        }

        let mut leaves_by_id = HashMap::new();
        for (i, n) in expr.iter().enumerate() {
            if let Some(sym) = n.get_var() {
                let id: Id = i.into();
                leaves_by_id.insert(id, self.leaves[&sym].clone());
            }
        }

        Self {
            expr,
            leaves_by_id,
            ..self
        }
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
                roots: vec![net.clone()],
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

/// Trait to create instantiable cell from the logic node
pub trait LogicCell<I: Instantiable> {
    /// Returns the instantiable cell type associated with this logic node
    fn get_cell(&self) -> Option<I>;
}

impl<I: Instantiable + LogicFunc<L>, L: CircuitLang + LogicCell<I>> LogicMapping<L, I> {
    /// Reinsert the expression into the netlist
    pub fn reinsert(self, netlist: &Rc<Netlist<I>>) -> Result<Vec<DrivenNet<I>>, Error> {
        let mut mapping: HashMap<Id, DrivenNet<I>> = HashMap::new();

        for (i, n) in self.expr.iter().enumerate() {
            if let Some(var) = n.get_var() {
                mapping.insert(i.into(), self.leaves[&var].clone());
            } else if !n.is_bus() {
                let cell = n.get_cell().ok_or(Error::ParseError(format!(
                    "Cannot reinsert node {} without associated cell",
                    n
                )))?;
                let operands = n
                    .children()
                    .iter()
                    .map(|c| mapping[c].clone())
                    .collect::<Vec<_>>();
                let inst_name = format_id!("reinst_{}", i);
                let instance = netlist.insert_gate(cell, inst_name, &operands)?;
                // TODO(matth2k): Support multi-output cells
                assert!(!instance.is_multi_output());
                let out = instance.get_output(0);
                mapping.insert(i.into(), out);
            }
        }

        let new_roots: Vec<_> = self.root_ids().map(|id| mapping[&id].clone()).collect();
        let old_net_names = self
            .root_nets()
            .map(|n| n.as_net().clone())
            .collect::<Vec<_>>();

        let old_roots: Vec<_> = self.root_nets().collect();

        drop(self);

        for (old, new) in old_roots.into_iter().zip(new_roots.iter()) {
            // TODO: update replace API, rename net in replace()
            old.as_net_mut().set_identifier("deleted".into());
            netlist.replace_net_uses(old.unwrap(), &new.clone().unwrap())?;
        }

        netlist.clean()?;

        for (new, n) in new_roots.iter().zip(old_net_names.into_iter()) {
            *new.as_net_mut() = n;
        }

        Ok(new_roots)
    }
}

impl LogicCell<PrimitiveCell> for CellLang {
    fn get_cell(&self) -> Option<PrimitiveCell> {
        match self {
            CellLang::And(_) => Some(PrimitiveCell::new(PrimitiveType::AND)),
            CellLang::Or(_) => Some(PrimitiveCell::new(PrimitiveType::OR)),
            CellLang::Inv(_) => Some(PrimitiveCell::new(PrimitiveType::NOT)),
            CellLang::Const(b) => PrimitiveCell::from_constant(Logic::from(*b)),
            CellLang::Cell(name, _) => match PrimitiveType::from_str(name.as_str()) {
                Ok(ptype) => Some(PrimitiveCell::new(ptype)),
                Err(_) => None,
            },
            _ => None,
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

    fn reg_cell() -> PrimitiveCell {
        PrimitiveCell::new(PrimitiveType::FDRE)
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

    fn divider_netlist() -> Rc<Netlist<PrimitiveCell>> {
        let netlist = Netlist::new("example".to_string());

        // Add the the input
        let a = netlist.insert_input("a".into());

        // Instantiate a reg
        let reg = netlist.insert_gate_disconnected(reg_cell(), "inst_0".into());

        // And last val and input
        let and = netlist
            .insert_gate(and_gate(), "inst_1".into(), &[a, reg.get_output(0)])
            .unwrap();

        reg.find_input(&"D".into()).unwrap().connect(and.into());

        // Make this Reg an output
        reg.expose_with_name("y".into());

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
        assert_eq!(mapping.root_nets().next().unwrap(), output);
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

    #[test]
    fn test_divider() {
        let netlist = divider_netlist();
        let output = netlist.last().unwrap().get_output(0);

        let mapper = netlist.get_analysis::<'_, LogicMapper<'_, CellLang, _>>();
        assert!(mapper.is_ok());
        let mut mapper = mapper.unwrap();

        let mapping = mapper.insert(output);
        assert!(mapping.is_err());

        let err = mapping.unwrap_err();
        // TODO(matth2k): Eventually simple cycles should be supported by breaking them up
        assert!(err.contains("Cycle"));
    }
}
