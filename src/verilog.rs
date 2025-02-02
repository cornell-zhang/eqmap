/*!

  Parse a rigid form of structural verilog and covert it into a [SVModule] struct.
  This struct can then be converted into a [LutLang] expression.

*/

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
    path::{Path, PathBuf},
};

use egg::{Id, RecExpr};
use sv_parser::{unwrap_node, Identifier, Locate, NodeEvent, RefNode};

use super::lut::{LutExprInfo, LutLang};

/// A wrapper for parsing verilog at file `path` with content `s`
pub fn sv_parse_wrapper(
    s: &str,
    path: Option<PathBuf>,
) -> Result<sv_parser::SyntaxTree, sv_parser::Error> {
    let incl: Vec<std::path::PathBuf> = vec![];
    let path = path.unwrap_or(Path::new("top.v").to_path_buf());
    match sv_parser::parse_sv_str(s, path, &HashMap::new(), &incl, true, false) {
        Ok((ast, _defs)) => Ok(ast),
        Err(e) => Err(e),
    }
}

/// For a `node` in the ast, this returns the source name for modules, nets, and ports (if one exists)
pub fn get_identifier(node: RefNode, ast: &sv_parser::SyntaxTree) -> Result<String, String> {
    let id: Option<Locate> = match unwrap_node!(
        node,
        SimpleIdentifier,
        EscapedIdentifier,
        NetIdentifier,
        PortIdentifier,
        Identifier
    ) {
        Some(RefNode::SimpleIdentifier(x)) => Some(x.nodes.0),
        Some(RefNode::EscapedIdentifier(x)) => Some(x.nodes.0),
        Some(RefNode::NetIdentifier(x)) => match &x.nodes.0 {
            Identifier::SimpleIdentifier(x) => Some(x.nodes.0),
            Identifier::EscapedIdentifier(x) => Some(x.nodes.0),
        },
        Some(RefNode::PortIdentifier(x)) => match &x.nodes.0 {
            Identifier::SimpleIdentifier(x) => Some(x.nodes.0),
            Identifier::EscapedIdentifier(x) => Some(x.nodes.0),
        },
        Some(RefNode::Identifier(x)) => match x {
            Identifier::SimpleIdentifier(x) => Some(x.nodes.0),
            Identifier::EscapedIdentifier(x) => Some(x.nodes.0),
        },
        _ => None,
    };

    match id {
        None => Err("Expected a Simple, Escaped, or Net identifier".to_string()),
        Some(x) => match ast.get_str(&x) {
            None => Err("Expected an identifier".to_string()),
            Some(x) => Ok(x.to_string()),
        },
    }
}

fn init_format(program: u64, k: usize) -> Result<String, ()> {
    let w = 1 << k;
    match k {
        1 => Ok(format!("{}'h{:01x}", w, program)),
        2 => Ok(format!("{}'h{:01x}", w, program)),
        3 => Ok(format!("{}'h{:02x}", w, program)),
        4 => Ok(format!("{}'h{:04x}", w, program)),
        5 => Ok(format!("{}'h{:08x}", w, program)),
        6 => Ok(format!("{}'h{:016x}", w, program)),
        _ => Err(()),
    }
}

fn init_parser(v: &str) -> Result<u64, String> {
    let split = v.split("'").collect::<Vec<&str>>();
    if split.len() != 2 {
        return Err("Expected a literal with specific bitwidth/format".to_string());
    }
    let literal = split[1];
    if let Some(l) = split[1].strip_prefix('h') {
        u64::from_str_radix(l, 16).map_err(|e| e.to_string())
    } else if let Some(l) = literal.strip_prefix('d') {
        l.parse::<u64>().map_err(|e| e.to_string())
    } else {
        Err("Expected a literal with specific bitwidth/format".to_string())
    }
}

#[test]
fn test_verilog_literals() {
    assert_eq!(init_parser("8'hff").unwrap(), 0xff);
    assert_eq!(init_parser("8'h00").unwrap(), 0x00);
    assert_eq!(init_parser("8'h0f").unwrap(), 0x0f);
    assert_eq!(init_parser("8'd255").unwrap(), 255);
    assert_eq!(init_format(1, 1), Ok("2'h1".to_string()));
    assert_eq!(init_format(1, 5), Ok("32'h00000001".to_string()));
    assert!(init_parser("1'hx").is_err());
    assert!(init_parser("1'hz").is_err());
}

const CLK: &str = "clk";
const REG_NAME: &str = "FDRE";
const LUT_ROOT: &str = "LUT";

#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents a signal declaration in the verilog
pub struct SVSignal {
    /// The bitwidth of the signal
    bw: usize,
    /// The decl name of the signal
    name: String,
}

impl SVSignal {
    /// Create a new signal with a bitwidth `bw` and name
    pub fn new(bw: usize, name: String) -> Self {
        SVSignal { bw, name }
    }

    /// Get the name of the signal
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

/// The [SVPrimitive] struct represents a primitive instance within a netlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SVPrimitive {
    /// The name of the primitive
    pub prim: String,
    /// The name of the instance
    pub name: String,
    /// Maps input ports to their signal driver
    /// E.g. (I0, a) means I0 is driven by signal a.
    inputs: BTreeMap<String, String>,
    /// Maps output signals to their port driver
    /// E.g. (y, O) means signal y is driven by output O.
    outputs: BTreeMap<String, String>,
    /// Stores arguments to module parameters as well as any other attribute
    pub attributes: BTreeMap<String, String>,
}

impl SVPrimitive {
    /// Create a new unconnected LUT primitive with size `k`, instance name `name`, and program `program`
    pub fn new_lut(k: usize, name: String, program: u64) -> Self {
        let mut attributes = BTreeMap::new();
        attributes.insert("INIT".to_string(), init_format(program, k).unwrap());
        SVPrimitive {
            prim: format!("{}{}", LUT_ROOT, k),
            name,
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            attributes,
        }
    }

    /// Create a new unconnected FDRE primitive with instance name `name`
    pub fn new_reg(name: String) -> Self {
        let mut attributes = BTreeMap::new();
        attributes.insert("INIT".to_string(), "1'hx".to_string());
        SVPrimitive {
            prim: REG_NAME.to_string(),
            name,
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            attributes,
        }
    }

    /// Create a new unconnected gate primitive with instance name `name`
    pub fn new_gate(gate: String, name: String) -> Self {
        SVPrimitive {
            prim: gate,
            name,
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            attributes: BTreeMap::new(),
        }
    }

    /// Create a new constant with name `name`
    pub fn new_const(val: bool, signal: String, name: String) -> Self {
        let mut output: BTreeMap<String, String> = BTreeMap::new();
        output.insert(signal, "Y".to_string());
        let mut attributes: BTreeMap<String, String> = BTreeMap::new();
        if val {
            attributes.insert("VAL".to_string(), "1'b1".to_string());
        } else {
            attributes.insert("VAL".to_string(), "1'b0".to_string());
        }
        SVPrimitive {
            prim: "CONST".to_string(),
            name,
            inputs: BTreeMap::new(),
            outputs: output,
            attributes,
        }
    }

    /// Create a new wire assignment with name `name` driven by `driver`
    pub fn new_wire(driver: String, signal: String, name: String) -> Self {
        let mut output: BTreeMap<String, String> = BTreeMap::new();
        output.insert(signal, "Y".to_string());
        let mut attributes: BTreeMap<String, String> = BTreeMap::new();
        attributes.insert("VAL".to_string(), driver);
        SVPrimitive {
            prim: "WIRE".to_string(),
            name,
            inputs: BTreeMap::new(),
            outputs: output,
            attributes,
        }
    }

    /// Add an input connection
    fn add_input(&mut self, port: String, signal: String) -> Result<(), String> {
        match self.inputs.insert(port.clone(), signal) {
            Some(d) => Err(format!(
                "Port {} is already driven on instance {} of {} by {}",
                port, self.name, self.prim, d
            )),
            None => Ok(()),
        }
    }

    /// Add an output connection
    fn add_output(&mut self, port: String, signal: String) -> Result<(), String> {
        match self.outputs.insert(signal.clone(), port) {
            Some(d) => Err(format!(
                "Port {} is already driven on instance {} of {} by {}",
                signal, self.name, self.prim, d
            )),
            None => Ok(()),
        }
    }

    /// Create an IO connection to the primitive based on port name. This is based on the Xilinx port naming conventions.
    pub fn add_signal(&mut self, port: String, signal: String) -> Result<(), String> {
        match port.as_str() {
            "I" | "I0" | "I1" | "I2" | "I3" | "I4" | "I5" | "D" | "A" | "B" | "S" => {
                self.add_input(port, signal)
            }
            "O" | "Y" | "Q" => self.add_output(port, signal),
            "C" | "CE" | "R" => Ok(()),
            _ => Err(format!("Unknown port name {}", port)),
        }
    }
}

impl fmt::Display for SVPrimitive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let level = 2;
        let indent = " ".repeat(2);

        if SVModule::is_assign_prim(&self.prim) {
            return write!(
                f,
                "{}assign {} = {};",
                indent,
                self.outputs.keys().next().unwrap(),
                self.attributes["VAL"]
            );
        }

        writeln!(f, "{}{} #(", indent, self.prim)?;
        for (i, (key, value)) in self.attributes.iter().enumerate() {
            let indent = " ".repeat(level + 4);
            write!(f, "{}.{}({})", indent, key, value)?;
            if i == self.attributes.len() - 1 {
                writeln!(f)?;
            } else {
                writeln!(f, ",")?;
            }
        }
        writeln!(f, "{}) {} (", indent, self.name)?;
        if self.prim.as_str() == REG_NAME {
            let indent = " ".repeat(level + 4);
            writeln!(f, "{}.C({}),", indent, CLK)?;
            writeln!(f, "{}.CE(1'h1),", indent)?;
        }
        for (input, value) in self.inputs.iter() {
            let indent = " ".repeat(level + 4);
            writeln!(f, "{}.{}({}),", indent, input, value)?;
        }
        if self.prim.as_str() == REG_NAME {
            let indent = " ".repeat(level + 4);
            writeln!(f, "{}.R(1'h0),", indent)?;
        }
        for (i, (value, output)) in self.outputs.iter().enumerate() {
            let indent = " ".repeat(level + 4);
            write!(f, "{}.{}({})", indent, output, value)?;
            if i == self.outputs.len() - 1 {
                writeln!(f)?;
            } else {
                writeln!(f, ",")?;
            }
        }
        write!(f, "{});", indent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents the connectivity of a Verilog module.
pub struct SVModule {
    /// The file name of the module
    pub fname: Option<String>,
    /// The name of the module
    pub name: String,
    /// All nets declared by the module (including inputs and outputs)
    pub signals: Vec<SVSignal>,
    /// All primitive instances in the module
    pub instances: Vec<SVPrimitive>,
    /// All input signals to the module
    pub inputs: Vec<SVSignal>,
    /// All output signals from the module
    pub outputs: Vec<SVSignal>,
    /// Returns the index of the primitive instance that drives a particular net
    pub driving_module: HashMap<String, usize>,
    /// Sequential and hence needs a clk
    pub clk: bool,
}

impl SVModule {
    /// Create an empty module with name `name`
    pub fn new(name: String) -> Self {
        SVModule {
            fname: None,
            name,
            signals: vec![],
            instances: vec![],
            inputs: vec![],
            outputs: vec![],
            driving_module: HashMap::new(),
            clk: false,
        }
    }

    /// Set the file name of the module
    pub fn with_fname(self, fname: String) -> Self {
        SVModule {
            fname: Some(fname),
            ..self
        }
    }

    /// Get the name of the module
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Append a list of primitive instances to the module
    pub fn append_insts(&mut self, insts: &mut Vec<SVPrimitive>) {
        let new_index = self.instances.len();
        for (i, inst) in insts.iter().enumerate() {
            for (signal, _port) in inst.outputs.iter() {
                self.driving_module.insert(signal.clone(), new_index + i);
            }
        }
        self.instances.append(insts);
    }

    /// Append a list of inputs to the module
    pub fn append_inputs(&mut self, inputs: &mut Vec<SVSignal>) {
        self.inputs.append(inputs);
    }

    /// Append a list of outputs to the module
    pub fn append_outputs(&mut self, outputs: &mut Vec<SVSignal>) {
        self.outputs.append(outputs);
    }

    /// Append a list of net declarations to the module
    pub fn append_signals(&mut self, outputs: &mut Vec<SVSignal>) {
        self.signals.append(outputs);
    }

    /// Names output `id` with `name` inside `self`
    pub fn name_output(&mut self, id: Id, name: String, mapping: &mut HashMap<Id, String>) {
        if mapping.contains_key(&id) {
            // In this case, create a wire
            let driver = mapping[&id].clone();
            let signal = SVSignal::new(1, name.clone());
            let wire = SVPrimitive::new_wire(
                driver.clone(),
                name.clone(),
                name.clone() + "_wire_" + &driver,
            );
            self.driving_module
                .insert(name.clone(), self.instances.len());
            self.instances.push(wire);
            self.signals.push(signal);
        } else {
            mapping.insert(id, name.clone());
        }
        self.outputs.push(SVSignal::new(1, name));
    }

    /// Get the driving primitive for a signal
    pub fn get_driving_primitive<'a>(&'a self, signal: &'a str) -> Result<&'a SVPrimitive, String> {
        match self.driving_module.get(signal) {
            Some(idx) => Ok(&self.instances[*idx]),
            None => Err(format!(
                "{}: Signal {} is not driven by any primitive in {}",
                self.fname.clone().unwrap_or("".to_string()),
                signal,
                self.name
            )),
        }
    }

    /// An O(n) method to check if a net is an input to the module
    pub fn is_an_input(&self, signal: &str) -> bool {
        self.inputs.iter().any(|x| x.name == signal)
    }

    fn is_lut_prim(name: &str) -> Option<usize> {
        match name.strip_prefix(LUT_ROOT) {
            Some(x) => match x.parse::<usize>() {
                Ok(x) => {
                    if x > 6 {
                        panic!("Only support LUTs up to size 6");
                    } else {
                        Some(x)
                    }
                }
                Err(_) => panic!("Could not parse LUT size"),
            },
            None => None,
        }
    }

    fn add_clk(&mut self) {
        if !self.clk {
            self.clk = true;
            self.append_inputs(&mut vec![SVSignal::new(1, CLK.to_string())]);
        }
    }

    fn is_reg_prim(name: &str) -> bool {
        name == REG_NAME
    }

    fn is_gate_prim(name: &str) -> bool {
        matches!(name, "AND2" | "NOR2" | "XOR2" | "NOT" | "INV" | "MUX")
    }

    fn is_assign_prim(name: &str) -> bool {
        matches!(name, "CONST" | "WIRE")
    }

    /// From a parsed verilog ast, create a new module and fill it with its primitives and connections.
    /// This method only works on structural verilog.
    pub fn from_ast(ast: &sv_parser::SyntaxTree) -> Result<Self, String> {
        let mut modules = vec![];
        // Current primitive instances in current module
        let mut cur_insts: Vec<SVPrimitive> = vec![];
        // Inputs to current module
        let mut cur_inputs: Vec<SVSignal> = vec![];
        // Outputs to current module
        let mut cur_outputs: Vec<SVSignal> = vec![];
        // All declared nets in the module (including inputs and outputs)
        let mut cur_signals: Vec<SVSignal> = vec![];

        for node_event in ast.into_iter().event() {
            match node_event {
                // Hande module definitions
                NodeEvent::Enter(RefNode::ModuleDeclarationAnsi(decl)) => {
                    let id = unwrap_node!(decl, ModuleIdentifier).unwrap();
                    let name = get_identifier(id, ast).unwrap();
                    modules.push(SVModule::new(name));
                }
                NodeEvent::Enter(RefNode::ModuleDeclarationNonansi(decl)) => {
                    let id = unwrap_node!(decl, ModuleIdentifier).unwrap();
                    let name = get_identifier(id, ast).unwrap();
                    modules.push(SVModule::new(name));
                }
                NodeEvent::Leave(RefNode::ModuleDeclarationAnsi(_decl)) => {
                    modules.last_mut().unwrap().append_insts(&mut cur_insts);
                    cur_insts.clear();
                    modules.last_mut().unwrap().append_inputs(&mut cur_inputs);
                    cur_inputs.clear();
                    modules.last_mut().unwrap().append_outputs(&mut cur_outputs);
                    cur_outputs.clear();
                    modules.last_mut().unwrap().append_signals(&mut cur_signals);
                    cur_signals.clear();
                }
                NodeEvent::Leave(RefNode::ModuleDeclarationNonansi(_decl)) => {
                    modules.last_mut().unwrap().append_insts(&mut cur_insts);
                    cur_insts.clear();
                    modules.last_mut().unwrap().append_inputs(&mut cur_inputs);
                    cur_inputs.clear();
                    modules.last_mut().unwrap().append_outputs(&mut cur_outputs);
                    cur_outputs.clear();
                    modules.last_mut().unwrap().append_signals(&mut cur_signals);
                    cur_signals.clear();
                }

                // Handle module instantiation
                NodeEvent::Enter(RefNode::ModuleInstantiation(inst)) => {
                    let id = unwrap_node!(inst, ModuleIdentifier).unwrap();
                    let mod_name = get_identifier(id, ast).unwrap();
                    let id = unwrap_node!(inst, InstanceIdentifier).unwrap();
                    let inst_name = get_identifier(id, ast).unwrap();

                    if let Some(k) = Self::is_lut_prim(&mod_name) {
                        let id = unwrap_node!(inst, NamedParameterAssignment).unwrap();
                        let program: u64 = match unwrap_node!(id, HexValue, UnsignedNumber) {
                            Some(RefNode::HexValue(v)) => {
                                let loc = v.nodes.0;
                                let loc = ast.get_str(&loc).unwrap();
                                match u64::from_str_radix(loc, 16) {
                                    Ok(x) => x,
                                    Err(_) => {
                                        return Err(format!(
                                            "Could not parse hex value from INIT string {}",
                                            loc
                                        ))
                                    }
                                }
                            }
                            Some(RefNode::UnsignedNumber(v)) => {
                                let loc = v.nodes.0;
                                let loc = ast.get_str(&loc).unwrap();
                                match loc.parse::<u64>() {
                                    Ok(x) => x,
                                    Err(_) => {
                                        return Err(format!(
                                            "Could not parse decimal value from INIT string {}",
                                            loc
                                        ))
                                    }
                                }
                            }
                            _ => {
                                return Err(format!(
                                    "{} {} should have INIT value written in hexadecimal",
                                    LUT_ROOT, mod_name
                                ));
                            }
                        };
                        cur_insts.push(SVPrimitive::new_lut(k, inst_name, program));
                        continue;
                    }

                    if Self::is_reg_prim(&mod_name) {
                        cur_insts.push(SVPrimitive::new_reg(inst_name));
                        continue;
                    }

                    if Self::is_gate_prim(&mod_name) {
                        cur_insts.push(SVPrimitive::new_gate(mod_name, inst_name));
                        continue;
                    }

                    return Err(format!(
                        "Expected a {} or {} primitive. Found primitive {} {:?}",
                        LUT_ROOT, REG_NAME, mod_name, inst
                    ));
                }
                NodeEvent::Leave(RefNode::ModuleInstantiation(_inst)) => (),

                // Handle input decl
                // TODO(mrh259): Handle bitwidth. Different declaration styles will need to be handled
                NodeEvent::Enter(RefNode::InputDeclarationNet(output)) => {
                    let id = unwrap_node!(output, PortIdentifier).unwrap();
                    let name = get_identifier(id, ast).unwrap();
                    cur_inputs.push(SVSignal::new(1, name));
                }

                NodeEvent::Leave(RefNode::InputDeclarationNet(_output)) => (),

                // Handle output decl
                // TODO(mrh259): Handle bitwidth. Different declaration styles will need to be handled
                NodeEvent::Enter(RefNode::OutputDeclarationNet(output)) => {
                    let id = unwrap_node!(output, PortIdentifier).unwrap();
                    let name = get_identifier(id, ast).unwrap();
                    cur_outputs.push(SVSignal::new(1, name));
                }

                NodeEvent::Leave(RefNode::OutputDeclarationNet(_output)) => (),

                // Handle instance args
                NodeEvent::Enter(RefNode::NamedPortConnection(connection)) => {
                    let port = unwrap_node!(connection, PortIdentifier).unwrap();
                    let port_name = get_identifier(port, ast).unwrap();
                    let arg = unwrap_node!(connection, Expression).unwrap();
                    let arg_i = unwrap_node!(arg.clone(), HierarchicalIdentifier);

                    match arg_i {
                        Some(n) => {
                            let arg_name = get_identifier(n, ast);
                            cur_insts
                                .last_mut()
                                .unwrap()
                                .add_signal(port_name, arg_name.unwrap())?;
                        }
                        None => {
                            // Ignore clock enable and resets
                            if port_name == "CE" || port_name == "R" {
                                if unwrap_node!(arg, PrimaryLiteral).is_none() {
                                    return Err(format!(
                                        "Port {} should be driven constant",
                                        port_name
                                    ));
                                }
                                continue;
                            } else {
                                return Err(format!(
                                    "Expected a HierarchicalIdentifier for port {}",
                                    port_name
                                ));
                            }
                        }
                    }
                }
                NodeEvent::Leave(RefNode::NamedPortConnection(_connection)) => (),

                // Handle wire/net decl
                NodeEvent::Enter(RefNode::NetDeclAssignment(net_decl)) => {
                    let id = unwrap_node!(net_decl, NetIdentifier).unwrap();
                    if unwrap_node!(net_decl, UnpackedDimension).is_some() {
                        panic!("Only support 1 bit signals!");
                    }
                    let name = get_identifier(id, ast).unwrap();
                    cur_signals.push(SVSignal::new(1, name));
                }
                NodeEvent::Leave(RefNode::NetDeclAssignment(_net_decl)) => (),

                // Handle wire assignment
                // TODO(mrh259): Refactor this branch of logic and this function in general
                NodeEvent::Enter(RefNode::NetAssignment(net_assign)) => {
                    let lhs = unwrap_node!(net_assign, NetLvalue).unwrap();
                    let lhs_id = unwrap_node!(lhs, Identifier).unwrap();
                    let lhs_name = get_identifier(lhs_id, ast).unwrap();
                    let rhs = unwrap_node!(net_assign, Expression).unwrap();
                    let rhs_id = unwrap_node!(rhs, Identifier, BinaryNumber, HexNumber).unwrap();
                    let assignment = unwrap_node!(net_assign, Symbol).unwrap();
                    match assignment {
                        RefNode::Symbol(sym) => {
                            let loc = sym.nodes.0;
                            let eq = ast.get_str(&loc).unwrap();
                            if eq != "=" {
                                return Err(format!("Expected an assignment operator, got {}", eq));
                            }
                        }
                        _ => {
                            return Err("Expected an assignment operator".to_string());
                        }
                    }
                    match rhs_id {
                        RefNode::Identifier(_) => {
                            let rhs_name = get_identifier(rhs_id, ast).unwrap();
                            cur_insts.push(SVPrimitive::new_wire(
                                rhs_name.clone(),
                                lhs_name.clone(),
                                lhs_name + "_wire_" + &rhs_name,
                            ));
                        }
                        RefNode::BinaryNumber(b) => {
                            let loc = b.nodes.2.nodes.0;
                            let val = ast.get_str(&loc).unwrap();
                            let val = match val {
                                "0" => false,
                                "1" => true,
                                _ => {
                                    return Err(format!(
                                        "Expected a 1 bit constant. Found {}",
                                        val
                                    ));
                                }
                            };
                            cur_insts.push(SVPrimitive::new_const(
                                val,
                                lhs_name.clone(),
                                lhs_name + "_const_binary",
                            ));
                        }
                        RefNode::HexNumber(b) => {
                            let loc = b.nodes.2.nodes.0;
                            let val = ast.get_str(&loc).unwrap();
                            let val = !matches!(val, "0");
                            cur_insts.push(SVPrimitive::new_const(
                                val,
                                lhs_name.clone(),
                                lhs_name + "_const_hex",
                            ));
                        }
                        _ => {
                            return Err("Expected a Identifier or PrimaryLiteral".to_string());
                        }
                    }
                }
                NodeEvent::Leave(RefNode::NetAssignment(_net_assign)) => (),
                _ => (),
            }
        }

        if modules.len() != 1 {
            return Err("Expected exactly one module".to_string());
        }

        Ok(modules.pop().unwrap())
    }

    /// Constructs a verilog module out of a [LutLang] expression.
    /// The module will be named `mod_name` and the outputs will be named from right to left with `outputs`.
    /// The default names for the outputs are `y0`, `y1`, etc. `outputs[0]` names the rightmost signal in a bus.
    pub fn from_expr(
        expr: RecExpr<LutLang>,
        mod_name: String,
        outputs: Vec<String>,
    ) -> Result<Self, String> {
        let mut module = SVModule::new(mod_name);

        let expr = LutExprInfo::new(&expr).get_cse();

        let mut mapping: HashMap<Id, String> = HashMap::new();
        let mut programs: HashMap<Id, u64> = HashMap::new();
        let mut prim_count: usize = 0;

        let mut fresh_prim = || {
            prim_count += 1;
            format!("__{}__", prim_count - 1)
        };

        let size = expr.as_ref().len();

        // Add output mappings
        let output_n = expr.as_ref().last().unwrap();
        let last_id: Id = (size - 1).into();
        match output_n {
            LutLang::Bus(l) => {
                for (i, t) in l.iter().enumerate() {
                    let defname = format!("y{}", i);
                    module.name_output(
                        *t,
                        outputs.get(i).unwrap_or(&defname).to_string(),
                        &mut mapping,
                    );
                }
            }
            _ => {
                module.name_output(
                    last_id,
                    outputs.first().unwrap_or(&"y".to_string()).to_string(),
                    &mut mapping,
                );
            }
        }

        let fresh_wire = |id: Id, mapping: &mut HashMap<Id, String>| {
            if !mapping.contains_key(&id) {
                let s = mapping.len();
                mapping.insert(id, format!("tmp{}", s));
            }
            mapping[&id].clone()
        };

        for (id, node) in expr.as_ref().iter().enumerate() {
            match node {
                LutLang::Var(s) => {
                    let sname = s.to_string();
                    if sname.contains("\n") || sname.contains(",") || sname.contains(";") {
                        return Err(
                            "Input cannot span multiple lines or contain delimiters".to_string()
                        );
                    }
                    if sname.contains("tmp") {
                        return Err("'tmp' is a reserved keyword".to_string());
                    }
                    if sname.contains(CLK) {
                        return Err(format!("'{}' is a reserved keyword", CLK));
                    }
                    if sname.contains("input") {
                        return Err("'input' is a reserved keyword".to_string());
                    }
                    let signal = SVSignal::new(1, sname.clone());
                    module.signals.push(signal.clone());
                    module.inputs.push(signal);

                    // Check if input directly drives an output
                    if mapping.contains_key(&id.into()) {
                        let output = mapping[&id.into()].clone();
                        let wire =
                            SVPrimitive::new_wire(sname.clone(), output.clone(), fresh_prim());
                        module
                            .driving_module
                            .insert(output.clone(), module.instances.len());
                        module.instances.push(wire);
                        module.signals.push(SVSignal::new(1, output));
                    }
                    mapping.insert(id.into(), sname);
                }
                LutLang::Program(p) => {
                    programs.insert(id.into(), *p);
                }
                LutLang::Reg([d]) => {
                    let sname = fresh_wire(id.into(), &mut mapping);
                    let pname = fresh_prim();
                    let mut inst = SVPrimitive::new_reg(pname);
                    inst.add_input("D".to_string(), mapping[d].clone())?;
                    inst.add_output("Q".to_string(), sname.clone())?;
                    module.signals.push(SVSignal::new(1, sname.clone()));
                    module
                        .driving_module
                        .insert(sname.clone(), module.instances.len());
                    module.instances.push(inst);
                    module.add_clk();
                }
                LutLang::Lut(l) => {
                    let sname = fresh_wire(id.into(), &mut mapping);
                    let pname = fresh_prim();
                    let mut inst = SVPrimitive::new_lut(l.len() - 1, pname, programs[&l[0]]);
                    for (i, c) in l[1..].iter().rev().enumerate() {
                        inst.add_input(format!("I{}", i), mapping[c].clone())?;
                    }
                    inst.add_output("O".to_string(), sname.clone())?;
                    module.signals.push(SVSignal::new(1, sname.clone()));
                    module
                        .driving_module
                        .insert(sname.clone(), module.instances.len());
                    module.instances.push(inst);
                }
                LutLang::Bus(_) => {
                    let last = id == size - 1;
                    if !last {
                        return Err("Busses shold be the root of the expression".to_string());
                    }
                }
                LutLang::And([a, b]) | LutLang::Xor([a, b]) | LutLang::Nor([a, b]) => {
                    let sname = fresh_wire(id.into(), &mut mapping);
                    let pname = fresh_prim();
                    let mut inst = SVPrimitive::new_gate(node.get_prim_name().unwrap(), pname);
                    inst.add_input("A".to_string(), mapping[a].clone())?;
                    inst.add_input("B".to_string(), mapping[b].clone())?;
                    inst.add_output("Y".to_string(), sname.clone())?;
                    module.signals.push(SVSignal::new(1, sname.clone()));
                    module
                        .driving_module
                        .insert(sname.clone(), module.instances.len());
                    module.instances.push(inst);
                }
                LutLang::Not([a]) => {
                    let sname = fresh_wire(id.into(), &mut mapping);
                    let pname = fresh_prim();
                    let mut inst = SVPrimitive::new_gate(node.get_prim_name().unwrap(), pname);
                    inst.add_input("A".to_string(), mapping[a].clone())?;
                    inst.add_output("Y".to_string(), sname.clone())?;
                    module.signals.push(SVSignal::new(1, sname.clone()));
                    module
                        .driving_module
                        .insert(sname.clone(), module.instances.len());
                    module.instances.push(inst);
                }
                LutLang::Mux([s, a, b]) => {
                    let sname = fresh_wire(id.into(), &mut mapping);
                    let pname = fresh_prim();
                    let mut inst = SVPrimitive::new_gate(node.get_prim_name().unwrap(), pname);
                    inst.add_input("A".to_string(), mapping[a].clone())?;
                    inst.add_input("B".to_string(), mapping[b].clone())?;
                    inst.add_input("S".to_string(), mapping[s].clone())?;
                    inst.add_output("Y".to_string(), sname.clone())?;
                    module.signals.push(SVSignal::new(1, sname.clone()));
                    module
                        .driving_module
                        .insert(sname.clone(), module.instances.len());
                    module.instances.push(inst);
                }
                LutLang::Const(b) => {
                    let sname = fresh_wire(id.into(), &mut mapping);
                    let pname = fresh_prim();
                    let inst = SVPrimitive::new_const(*b, sname.clone(), pname);
                    module.signals.push(SVSignal::new(1, sname.clone()));
                    module
                        .driving_module
                        .insert(sname.clone(), module.instances.len());
                    module.instances.push(inst);
                }
                _ => return Err(format!("Unsupported node type: {:?}", node)),
            }
        }

        Ok(module)
    }

    fn get_expr<'a>(
        &'a self,
        signal: &'a str,
        expr: &mut RecExpr<LutLang>,
        map: &mut HashMap<&'a str, Id>,
    ) -> Result<Id, String> {
        if map.contains_key(signal) {
            return Ok(map[signal]);
        }

        let id = match self.get_driving_primitive(signal) {
            Ok(primitive) => {
                if Self::is_gate_prim(primitive.prim.as_str()) {
                    // Update the mapping
                    let mut subexpr: HashMap<&'a str, Id> = HashMap::new();
                    for (port, signal) in primitive.inputs.iter() {
                        subexpr.insert(port, self.get_expr(signal, expr, map)?);
                    }
                    match primitive.prim.as_str() {
                        "AND2" => Ok(expr.add(LutLang::And([subexpr["A"], subexpr["B"]]))),
                        "NOR2" => Ok(expr.add(LutLang::Nor([subexpr["A"], subexpr["B"]]))),
                        "XOR2" => Ok(expr.add(LutLang::Xor([subexpr["A"], subexpr["B"]]))),
                        "MUX" => {
                            Ok(expr.add(LutLang::Mux([subexpr["S"], subexpr["A"], subexpr["B"]])))
                        }
                        "NOT" => Ok(expr.add(LutLang::Not([subexpr["A"]]))),
                        "INV" => Ok(expr.add(LutLang::Not([subexpr["I"]]))),
                        _ => Err(format!("Unsupported gate primitive {}", primitive.prim)),
                    }
                } else if Self::is_reg_prim(primitive.prim.as_str()) {
                    let d = primitive.inputs.first_key_value().unwrap().1;
                    let d = self.get_expr(d, expr, map)?;
                    Ok(expr.add(LutLang::Reg([d])))
                } else if Self::is_assign_prim(primitive.prim.as_str()) {
                    let val = primitive.attributes.get("VAL").unwrap();
                    if primitive.prim.as_str() == "CONST" {
                        let val = val == "1'b1";
                        Ok(expr.add(LutLang::Const(val)))
                    } else {
                        self.get_expr(val.as_str(), expr, map)
                    }
                } else {
                    let mut subexpr: Vec<Id> = vec![];
                    let program = primitive.attributes.get("INIT").ok_or(format!(
                        "Only {} and {} primitives are supported. INIT not found.",
                        LUT_ROOT, REG_NAME
                    ))?;
                    let program: u64 = init_parser(program)?;
                    subexpr.push(expr.add(LutLang::Program(program)));
                    for input in (0..primitive.inputs.len()).rev().map(|x| format!("I{}", x)) {
                        let driver = primitive
                            .inputs
                            .get(&input)
                            .ok_or(format!("Expected {} on {} to be driven.", input, LUT_ROOT))?;
                        subexpr.push(self.get_expr(driver, expr, map)?);
                    }
                    Ok(expr.add(LutLang::Lut(subexpr.into())))
                }
            }
            Err(e) => {
                if self.is_an_input(signal) {
                    Ok(expr.add(LutLang::Var(signal.into())))
                } else {
                    Err(e)
                }
            }
        }?;

        map.insert(signal, id);
        Ok(id)
    }

    /// Get a separate [LutLang] expression for every output in the module
    pub fn get_exprs(&self) -> Result<Vec<(String, RecExpr<LutLang>)>, String> {
        if let Err(s) = self.contains_cycles() {
            return Err(format!(
                "Cannot convert module with feedback on signal {}",
                s
            ));
        }

        let mut exprs = vec![];
        for output in self.outputs.iter() {
            let mut expr = RecExpr::default();
            self.get_expr(&output.name, &mut expr, &mut HashMap::new())?;
            exprs.push((output.name.clone(), expr));
        }
        Ok(exprs)
    }

    /// Get a single [LutLang] expression for the module as a bus
    pub fn to_single_expr(&self) -> Result<RecExpr<LutLang>, String> {
        if let Err(s) = self.contains_cycles() {
            return Err(format!(
                "Cannot convert module with feedback on signal {}",
                s
            ));
        }

        let mut expr: RecExpr<LutLang> = RecExpr::default();
        let mut map = HashMap::new();
        let mut outputs: Vec<Id> = vec![];
        for output in self.outputs.iter() {
            outputs.push(self.get_expr(&output.name, &mut expr, &mut map)?);
        }
        if outputs.len() > 1 {
            expr.add(LutLang::Bus(outputs.into()));
        }
        // TODO(matth2k): Add an option to run subexpression elimination here
        Ok(expr)
    }

    /// Convert the module to a [LutLang] expression
    pub fn to_expr(&self) -> Result<RecExpr<LutLang>, String> {
        if let Err(s) = self.contains_cycles() {
            return Err(format!(
                "Cannot convert module with feedback on signal {}",
                s
            ));
        }

        if self.outputs.len() != 1 {
            return Err(format!(
                "{}: Expected exactly one output in module {}.",
                self.fname.clone().unwrap_or("".to_string()),
                self.name
            ));
        }

        Ok(self.get_exprs()?.pop().unwrap().1)
    }

    /// Get the name of the outputs of the module
    pub fn get_outputs(&self) -> Vec<&str> {
        self.outputs.iter().map(|x| x.get_name()).collect()
    }

    fn contains_cycles_rec<'a>(
        &'a self,
        signal: &'a str,
        walk: &mut HashSet<&'a str>,
    ) -> Result<(), &'a str> {
        if walk.contains(signal) {
            return Err(signal);
        }
        walk.insert(signal);
        let driving = self.get_driving_primitive(signal);
        if driving.is_err() {
            walk.remove(signal);
            return Ok(());
        }
        let driving = driving.unwrap();
        for (_, driver) in driving.inputs.iter() {
            self.contains_cycles_rec(driver, walk)?
        }
        walk.remove(signal);
        Ok(())
    }

    /// We cannot lower verilog with cycles in it to LutLang expressions.
    /// This function returns [Ok] when there are no cycles in the module
    pub fn contains_cycles<'a>(&'a self) -> Result<(), &'a str> {
        for output in self.outputs.iter() {
            let mut stack: HashSet<&'a str> = HashSet::new();
            self.contains_cycles_rec(output.get_name(), &mut stack)?
        }
        Ok(())
    }
}

impl fmt::Display for SVModule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let level = 0;
        let indent = " ".repeat(level);
        writeln!(f, "{}module {} (", indent, self.name)?;
        for input in self.inputs.iter() {
            let indent = " ".repeat(level + 4);
            writeln!(f, "{}{},", indent, input.name)?;
        }
        for (i, output) in self.outputs.iter().enumerate() {
            let indent = " ".repeat(level + 4);
            write!(f, "{}{}", indent, output.name)?;
            if i == self.outputs.len() - 1 {
                writeln!(f)?;
            } else {
                writeln!(f, ",")?;
            }
        }
        writeln!(f, "{});", indent)?;
        let mut already_decl: HashSet<String> = HashSet::new();
        for input in self.inputs.iter() {
            let indent = " ".repeat(level + 2);
            writeln!(f, "{}input {};", indent, input.name)?;
            writeln!(f, "{}wire {};", indent, input.name)?;
            already_decl.insert(input.name.clone());
        }
        for output in self.outputs.iter() {
            let indent = " ".repeat(level + 2);
            writeln!(f, "{}output {};", indent, output.name)?;
            writeln!(f, "{}wire {};", indent, output.name)?;
            already_decl.insert(output.name.clone());
        }
        for signal in self.signals.iter() {
            let indent = " ".repeat(level + 2);
            if !already_decl.contains(&signal.name) {
                writeln!(f, "{}wire {};", indent, signal.name)?;
            }
        }
        for instance in self.instances.iter() {
            writeln!(f, "{}", instance)?;
        }
        write!(f, "{}endmodule", indent)
    }
}
