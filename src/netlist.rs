/*!

  Support for nl-compiler

*/

use crate::asic::CellLang;
use crate::driver::CircuitLang;
use crate::lut::LutLang;
use crate::verilog::PrimitiveType;
use bitvec::field::BitField;
use egg::{Id, RecExpr, Symbol};
use nl_compiler::FromId;
use safety_net::{
    Analysis, DrivenNet, Error, Identifier, Instantiable, Logic, Net, Netlist, Parameter,
    format_id, iter::NetDFSIterator,
};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::str::FromStr;

/// Trait for circuit elements that can provide a logic function
pub trait LogicFunc<L: CircuitLang> {
    /// Get the logic function/variant associated with the output at position `ind`.
    /// The children IDs are set to `children`.
    fn get_logic_func(&self, ind: usize, children: &[Id]) -> Option<L>;
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
    /// Panics if the new expression does not have the same number of roots as the old one.
    /// Panics of the new expression contains leaf variables not in the original mapping.
    pub fn with_expr(self, expr: RecExpr<L>) -> Self {
        let l1 = self.expr.last().unwrap();
        let l2 = expr.last().unwrap();

        if l1.is_bus() != l2.is_bus() {
            panic!("New expression must have the same number of roots as the old one");
        }

        if l1.is_bus() && l1.children().len() != l2.children().len() {
            panic!("New expression must have the same number of roots as the old one");
        }

        let mut leaves_by_id = HashMap::new();
        let mut leaves = HashMap::new();
        for (i, n) in expr.iter().enumerate() {
            if let Some(sym) = n.get_var() {
                let id: Id = i.into();
                leaves_by_id.insert(id, self.leaves[&sym].clone());
                leaves.insert(sym, self.leaves[&sym].clone());
            }
        }

        Self {
            expr,
            leaves_by_id,
            leaves,
            ..self
        }
    }
}

/// A helper stack that can check contains in roughly O(1) time.
#[derive(Clone)]
struct Walk<T: std::hash::Hash + PartialEq + Eq + Clone> {
    stack: Vec<T>,
    counter: HashMap<T, usize>,
}

impl<T> Walk<T>
where
    T: std::hash::Hash + PartialEq + Eq + Clone,
{
    /// Create a new, empty Stack.
    fn new() -> Self {
        Self {
            stack: Vec::new(),
            counter: HashMap::new(),
        }
    }

    /// Inserts an element into the stack
    fn push(&mut self, item: T) {
        self.stack.push(item.clone());
        *self.counter.entry(item).or_insert(0) += 1;
    }

    // Pops an element from the stack
    fn pop(&mut self) -> Option<T> {
        if let Some(item) = self.stack.pop() {
            let count = self.counter.get_mut(&item).unwrap();
            *count -= 1;
            if *count == 0 {
                self.counter.remove(&item);
            }
            Some(item)
        } else {
            None
        }
    }

    /// Creates a stack with one element
    fn from_elem(elem: T) -> Self {
        let mut stack = Self::new();
        stack.push(elem);
        stack
    }

    /// Returns true if the stack shows a cycle
    fn contains_cycle(&self) -> bool {
        self.counter.values().any(|&count| count > 1)
    }

    /// Returns a reference to the last element in the stack
    fn last(&self) -> Option<&T> {
        self.stack.last()
    }
}

/// Extracts the logic equation from a portion of a netlist.
pub struct LogicMapper<'a, L: CircuitLang, I: Instantiable + LogicFunc<L>> {
    _netlist: &'a Netlist<I>,
    mappings: Vec<LogicMapping<L, I>>,
}

impl<'a, L, I> Analysis<'a, I> for LogicMapper<'a, L, I>
where
    L: CircuitLang + 'a,
    I: Instantiable + LogicFunc<L> + 'a,
{
    fn build(netlist: &'a Netlist<I>) -> Result<Self, Error> {
        netlist.verify()?;
        Ok(Self {
            _netlist: netlist,
            mappings: Vec::new(),
        })
    }
}

impl<'a, L: CircuitLang, I: Instantiable + LogicFunc<L>> LogicMapper<'a, L, I> {
    /// Map `nets` to [CircuitLang] nodes. `nets` that do not pass `filter` become leaves.
    fn insert_filtered<F>(
        &mut self,
        nets: Vec<DrivenNet<I>>,
        filter: F,
    ) -> Result<RecExpr<L>, String>
    where
        F: Fn(&I) -> bool,
    {
        let mut expr = RecExpr::<L>::default();
        let mut mapping: HashMap<DrivenNet<I>, Id> = HashMap::new();
        let mut leaves: HashMap<Symbol, DrivenNet<I>> = HashMap::new();
        let mut leaves_by_id: HashMap<Id, DrivenNet<I>> = HashMap::new();

        let roots = nets.clone();
        let mut topo = Vec::new();
        let mut sorted = HashSet::new();
        let mut walks: Vec<Walk<DrivenNet<I>>> = nets.into_iter().map(Walk::from_elem).collect();

        while let Some(mut walk) = walks.pop() {
            if walk.contains_cycle() {
                return Err(format!(
                    "Cycle detected at netlist stemming from {}",
                    walk.last().unwrap()
                ));
            }

            let net = walk.pop().unwrap();
            if sorted.contains(&net) {
                continue;
            }

            if net.is_an_input() {
                sorted.insert(net.clone());
                topo.push(net);
                continue;
            }

            // Something that is being filtered-out into a leaf is considered ready/sorted
            if let Some(inst_type) = net.clone().get_instance_type()
                && !filter(&inst_type)
            {
                sorted.insert(net.clone());
                topo.push(net);
                continue;
            }

            let mut dfs = NetDFSIterator::new_with_boundary(self._netlist, net.clone(), |n| {
                if n.is_an_input() {
                    return true;
                }

                if let Some(inst_type) = n.get_instance_type() {
                    !filter(&*inst_type)
                } else {
                    false
                }
            });
            let mut rdy = true;
            dfs.next(); // Skip the root node
            for n in dfs {
                if !sorted.contains(&n) {
                    rdy = false;
                    walk.push(net.clone());
                    walks.push(walk.clone());
                    walk.push(n);
                    walks.push(walk);
                    break;
                }
            }

            if rdy {
                sorted.insert(net.clone());
                topo.push(net);
            }
        }

        for n in topo {
            if mapping.contains_key(&n) {
                continue;
            } else if let Some(inst_type) = n.get_instance_type()
                && filter(&inst_type)
            {
                let mut children = vec![];
                for (i, c) in n.clone().unwrap().inputs().enumerate() {
                    let cid = c
                        .get_driver()
                        .ok_or(format!("Failed to get driver for input {} of net {}", i, n))?;
                    children.push(mapping[&cid]);
                }

                // TODO(matth2k): Generalize a way for CircuitLang to accept parameters
                if inst_type.get_name().to_string().starts_with("LUT") {
                    let tt = inst_type.get_parameter(&"INIT".into()).ok_or(format!(
                        "LUT cell {} missing INIT parameter",
                        inst_type.get_name()
                    ))?;
                    let tt = match tt {
                        Parameter::BitVec(tt) => tt.load::<u64>(),
                        _ => {
                            return Err(format!(
                                "LUT cell {} has non-integer INIT parameter",
                                inst_type.get_name()
                            ));
                        }
                    };
                    let p = expr.add(L::int(tt).ok_or(format!(
                        "Language does not support integer nodes required for LUT {}",
                        inst_type.get_name()
                    ))?);
                    children.insert(0, p);
                }

                if let Some(logic) =
                    inst_type.get_logic_func(n.get_output_index().unwrap(), &children)
                {
                    let id = expr.add(logic);
                    mapping.insert(n.clone(), id);
                    continue;
                }
            }

            let sym = n.get_identifier();
            let id = expr.add(L::var(sym.to_string().into()));
            mapping.insert(n.clone(), id);
            leaves.insert(sym.to_string().into(), n.clone());
            leaves_by_id.insert(id, n.clone());
        }

        if roots.len() > 1 {
            let bus = L::bus(roots.iter().map(|n| mapping[n]));
            expr.add(bus);
        }

        self.mappings.push(LogicMapping {
            expr: expr.clone(),
            roots,
            leaves,
            leaves_by_id,
        });

        Ok(expr)
    }

    /// Map `nets` to [CircuitLang] nodes.
    pub fn insert(&mut self, nets: Vec<DrivenNet<I>>) -> Result<RecExpr<L>, String> {
        self.insert_filtered(nets, |_| true)
    }

    /// Map a specific `net` to [CircuitLang] nodes.
    pub fn insert_single_net(&mut self, net: DrivenNet<I>) -> Result<RecExpr<L>, String> {
        if net.is_an_input() {
            return Err("Inputs have trivial mappings".to_string());
        }

        self.insert(vec![net])
    }

    /// Map all logic to [CircuitLang] along register-to-register paths. This prevents register retiming.
    pub fn insert_all_r2r(&mut self) -> Result<RecExpr<L>, String> {
        let mut nets: HashSet<DrivenNet<I>> = self
            ._netlist
            .outputs()
            .into_iter()
            .map(|(n, _)| n)
            .collect();

        for nr in self._netlist.matches(|i| i.is_seq()) {
            for input in nr.inputs() {
                if let Some(dr) = input.get_driver()
                    && let Some(di) = dr.clone().get_instance_type()
                    && !di.is_seq()
                {
                    nets.insert(dr);
                }
            }
        }

        let nets: Vec<DrivenNet<I>> = nets.into_iter().collect();

        self.insert_filtered(nets, |i| !i.is_seq())
    }

    /// Get the mapped expressions
    pub fn mappings(self) -> Vec<LogicMapping<L, I>> {
        self.mappings
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
    pub fn new(ptype: PrimitiveType, size: Option<usize>) -> Self {
        Self {
            name: if let Some(s) = size {
                format_id!("{}_X{}", ptype, s)
            } else {
                format_id!("{}", ptype)
            },
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

    /// Remap the ith input port to a new net name
    pub fn remap_input(mut self, ind: usize, name: Identifier) -> Self {
        let net = &mut self.inputs[ind];
        net.set_identifier(name);
        self
    }

    /// Remap the ith output port to a new net name
    pub fn remap_output(mut self, ind: usize, name: Identifier) -> Self {
        let net = &mut self.outputs[ind];
        net.set_identifier(name);
        self
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
        self.params.clone().into_iter()
    }

    fn from_constant(val: Logic) -> Option<Self> {
        match val {
            Logic::False => Some(PrimitiveCell::new(PrimitiveType::GND, None)),
            Logic::True => Some(PrimitiveCell::new(PrimitiveType::VCC, None)),
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

    fn is_seq(&self) -> bool {
        self.ptype.is_reg()
    }
}

impl LogicFunc<CellLang> for PrimitiveCell {
    fn get_logic_func(&self, ind: usize, children: &[Id]) -> Option<CellLang> {
        if ind != 0 {
            return None;
        }

        match self.ptype {
            PrimitiveType::AND => Some(CellLang::And(children.try_into().ok()?)),
            PrimitiveType::VCC => Some(CellLang::Const(true)),
            PrimitiveType::GND => Some(CellLang::Const(false)),
            PrimitiveType::OR => Some(CellLang::Or(children.try_into().ok()?)),
            PrimitiveType::NOT => Some(CellLang::Inv(children.try_into().ok()?)),
            _ if self.ptype.is_lut() => None,
            _ => Some(CellLang::Cell(
                self.ptype.to_string().into(),
                children.to_vec(),
            )),
        }
    }
}

impl LogicFunc<LutLang> for PrimitiveCell {
    fn get_logic_func(&self, ind: usize, children: &[Id]) -> Option<LutLang> {
        if ind != 0 {
            return None;
        }

        match self.ptype {
            PrimitiveType::AND => Some(LutLang::And(children.try_into().ok()?)),
            PrimitiveType::VCC => Some(LutLang::Const(true)),
            PrimitiveType::GND => Some(LutLang::Const(false)),
            PrimitiveType::NOR => Some(LutLang::Nor(children.try_into().ok()?)),
            PrimitiveType::XOR => Some(LutLang::Xor(children.try_into().ok()?)),
            PrimitiveType::MUX => Some(LutLang::Mux(children.try_into().ok()?)),
            PrimitiveType::NOT => Some(LutLang::Not(children.try_into().ok()?)),
            PrimitiveType::FDRE => Some(LutLang::Reg(children.try_into().ok()?)),
            _ if self.ptype.is_lut() => Some(LutLang::Lut(children.into())),
            _ => None,
        }
    }
}

/// Trait to create instantiable cell from the logic node
pub trait LogicCell<I: Instantiable>
where
    Self: Sized,
{
    /// Returns the instantiable cell type associated with this logic node
    fn get_cell(&self, params: &[(Identifier, Parameter)]) -> Option<I>;
}

impl<I: Instantiable + LogicFunc<L>, L: CircuitLang + LogicCell<I>> LogicMapping<L, I> {
    /// Rewrite the expression into the netlist
    pub fn rewrite(self, netlist: &Rc<Netlist<I>>) -> Result<Vec<DrivenNet<I>>, Error> {
        let mut mapping: HashMap<Id, DrivenNet<I>> = HashMap::new();

        for (i, n) in self.expr.iter().enumerate() {
            if let Some(var) = n.get_var() {
                mapping.insert(i.into(), self.leaves[&var].clone());
            } else if !n.is_bus() && n.get_int().is_none() {
                // TODO(matth2k): Generalize a param extractor for CircuitLang
                let params = if n.is_lut() {
                    let tt = &self.expr[n.children()[0]];
                    let tt = tt.get_int().ok_or(Error::ParseError(format!(
                        "LUT node missing integer parameter: {}",
                        tt
                    )))?;
                    let inputs = n.children().len() - 1;
                    vec![(
                        "INIT".into(),
                        Parameter::bitvec(2_usize.pow(inputs as u32), tt),
                    )]
                } else {
                    vec![]
                };

                let cell = n.get_cell(&params).ok_or(Error::ParseError(format!(
                    "Cannot reinsert node {} without associated cell",
                    n
                )))?;
                let operands = n
                    .children()
                    .iter()
                    // TODO(matth2k): Generalize a param extractor for CircuitLang
                    .skip(if n.is_lut() { 1 } else { 0 })
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

        let mut root_pairs: Vec<_> = self
            .root_nets()
            .zip(self.root_ids().map(|id| mapping[&id].clone()))
            .collect();

        root_pairs.sort();
        root_pairs.dedup();

        drop(self);
        drop(mapping);

        let mut new_roots = Vec::new();

        for (old, new) in root_pairs {
            if old == new {
                new_roots.push(new);
                continue;
            }

            if !old.is_an_input() && old.is_top_level_output() {
                let id = old.get_identifier() + "_overwritten".into();
                old.as_net_mut().set_identifier(id);
            }

            netlist.replace_net_uses(old, &new)?;
            new_roots.push(new);
        }

        netlist.clean()?;

        netlist.rename_nets(|_, i| format_id!("__{i}__"))?;

        Ok(new_roots)
    }
}

impl LogicCell<PrimitiveCell> for CellLang {
    fn get_cell(&self, params: &[(Identifier, Parameter)]) -> Option<PrimitiveCell> {
        let mut cell = match self {
            CellLang::And(_) => PrimitiveCell::new(PrimitiveType::AND2, Some(1)),
            CellLang::Or(_) => PrimitiveCell::new(PrimitiveType::OR2, Some(1)),
            CellLang::Inv(_) => PrimitiveCell::new(PrimitiveType::INV, Some(1)),
            CellLang::Const(b) => PrimitiveCell::from_constant(Logic::from(*b))?,
            CellLang::Cell(name, _) => match PrimitiveType::from_str(name.as_str()) {
                Ok(ptype) => PrimitiveCell::new(ptype, Some(1)),
                Err(_) => return None,
            },
            _ => return None,
        };

        for param in params {
            cell.set_parameter(&param.0, param.1.clone());
        }

        Some(cell)
    }
}

impl LogicCell<PrimitiveCell> for LutLang {
    fn get_cell(&self, params: &[(Identifier, Parameter)]) -> Option<PrimitiveCell> {
        let mut cell = match self {
            LutLang::And(_) => PrimitiveCell::new(PrimitiveType::AND, None),
            LutLang::Mux(_) => PrimitiveCell::new(PrimitiveType::MUX, None),
            LutLang::Nor(_) => PrimitiveCell::new(PrimitiveType::NOR, None),
            LutLang::Not(_) => PrimitiveCell::new(PrimitiveType::INV, None)
                .remap_input(0, "I".into())
                .remap_output(0, "O".into()),
            LutLang::Const(b) => PrimitiveCell::from_constant(Logic::from(*b))?,
            LutLang::DC => PrimitiveCell::from_constant(Logic::X)?,
            LutLang::Reg(_) => PrimitiveCell::new(PrimitiveType::FDRE, None),
            LutLang::Xor(_) => PrimitiveCell::new(PrimitiveType::XOR, None),
            LutLang::Lut(l) => match l.len() {
                2 => PrimitiveCell::new(PrimitiveType::LUT1, None),
                3 => PrimitiveCell::new(PrimitiveType::LUT2, None),
                4 => PrimitiveCell::new(PrimitiveType::LUT3, None),
                5 => PrimitiveCell::new(PrimitiveType::LUT4, None),
                6 => PrimitiveCell::new(PrimitiveType::LUT5, None),
                7 => PrimitiveCell::new(PrimitiveType::LUT6, None),
                _ => return None,
            },
            _ => return None,
        };

        for param in params {
            cell.set_parameter(&param.0, param.1.clone());
        }

        if cell.ptype.is_lut() && !cell.has_parameter(&"INIT".into()) {
            return None;
        }

        if cell.ptype.is_reg() && !cell.has_parameter(&"INIT".into()) {
            cell.set_parameter(&"INIT".into(), Parameter::Logic(Logic::X));
        }

        Some(cell)
    }
}

impl FromId for PrimitiveCell {
    fn from_id(s: &Identifier) -> Result<Self, Error> {
        match PrimitiveType::from_str(&s.to_string()) {
            Ok(ptype) => Ok(PrimitiveCell::new(
                ptype, None, /* Drop the size for logic synthesis */
            )),
            Err(e) => Err(Error::ParseError(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    fn and_gate() -> PrimitiveCell {
        PrimitiveCell::new(PrimitiveType::AND, None)
    }

    fn reg_cell() -> PrimitiveCell {
        PrimitiveCell::new(PrimitiveType::FDRE, None)
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
        // Setting both the net and output name to "y" tests more edge cases
        instance
            .get_output(0)
            .as_net_mut()
            .set_identifier("y".into());
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
        let b = netlist.insert_constant(Logic::False, "b".into()).unwrap();

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
        let expr = mapper.insert_single_net(output.clone());
        assert!(expr.is_ok());
        let expr = expr.unwrap();
        assert_eq!(expr.to_string(), "(AND a b)");

        // Check the root properties are correct
        let mut mapping = mapper.mappings();
        assert!(!mapping.is_empty());
        let mapping = mapping.pop().unwrap();
        assert_eq!(mapping.root_nets().next().unwrap(), output);
        assert_eq!(netlist.objects().count(), mapping.get_expr().as_ref().len());

        // Check the leaves
        let l0 = mapping.get_leaf(&"a".into());
        assert!(l0.is_some());
        let l0 = l0.unwrap();
        assert_eq!(l0, netlist.first().unwrap().into());
    }

    #[test]
    fn test_consts() {
        let netlist = and_const_netlist();
        let output = netlist.last().unwrap().get_output(0);

        let mapper = netlist.get_analysis::<'_, LogicMapper<'_, CellLang, _>>();
        assert!(mapper.is_ok());
        let mut mapper = mapper.unwrap();

        // Check the RecExpr is correct
        let expr = mapper.insert_single_net(output.clone());
        assert!(expr.is_ok());
        let expr = expr.unwrap();
        assert_eq!(expr.to_string(), "(AND true false)");
    }

    #[test]
    fn test_divider() {
        let netlist = divider_netlist();
        let output = netlist.last().unwrap().get_output(0);

        let mapper = netlist.get_analysis::<'_, LogicMapper<'_, CellLang, _>>();
        assert!(mapper.is_ok());
        let mut mapper = mapper.unwrap();

        let mapping = mapper.insert_single_net(output);
        assert!(mapping.is_err());

        let err = mapping.unwrap_err();
        // TODO(matth2k): Eventually simple cycles should be supported by breaking them up
        assert!(err.contains("Cycle"));
    }

    #[test]
    fn test_divider_r2r() {
        let netlist = divider_netlist();

        let mapper = netlist.get_analysis::<'_, LogicMapper<'_, CellLang, _>>();
        assert!(mapper.is_ok());
        let mut mapper = mapper.unwrap();

        let mapping = mapper.insert_all_r2r();
        assert!(mapping.is_ok());
    }

    #[test]
    fn test_and_flip() {
        let netlist = and_netlist();
        let output = netlist.last().unwrap().get_output(0);

        let mapper = netlist.get_analysis::<'_, LogicMapper<'_, CellLang, _>>();
        assert!(mapper.is_ok());
        let mut mapper = mapper.unwrap();

        // Check the RecExpr is correct
        let check = mapper.insert_single_net(output);
        assert!(check.is_ok());

        let mut mapping = mapper.mappings();
        assert!(!mapping.is_empty());
        let mapping = mapping.pop().unwrap();

        let rewrite: RecExpr<CellLang> = "(AND b a)".parse().unwrap();
        let mapping = mapping.with_expr(rewrite);

        let rewrite = mapping.rewrite(&netlist);
        assert!(rewrite.is_ok());
        assert!(netlist.objects().count() == 3);
    }
}
