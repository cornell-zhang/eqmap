/*!

  Parse a rigid form of structural verilog and covert it into a [SVModule] struct.
  This struct can then be converted into a [LutLang] expression.

*/

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use egg::{Id, RecExpr};
use sv_parser::{unwrap_node, Identifier, Locate, NodeEvent, RefNode};

use super::lut::LutLang;

/// A wrapper for parsing verilog at file `path` with content `s`
pub fn sv_parse_wrapper(
    s: &str,
    path: Option<PathBuf>,
) -> Result<sv_parser::SyntaxTree, sv_parser::Error> {
    let incl: Vec<std::path::PathBuf> = vec![];
    let path = path.unwrap_or(Path::new("top.v").to_path_buf());
    match sv_parser::parse_sv_str(s, &path, &HashMap::new(), &incl, true, false) {
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
}

/// The [SVPrimitive] struct represents a primitive instance in the inputted structural verilog.
/// For now, it show always be a LUT.
/// For the `inputs` and `outputs` pairs of a primitive, the key is driven by the value.
/// E.g. (I0, a) in inputs and (y, O) in outputs. Input I0 is driven by signal a, signal y is driven by output O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SVPrimitive {
    /// The name of the primitive
    pub prim: String,
    /// The name of the instance
    pub name: String,
    /// Maps input ports to their signal driver
    inputs: HashMap<String, String>,
    /// Maps output signals to their port driver
    outputs: HashMap<String, String>,
    /// Stores arguments to module parameters as well as any other attribute
    pub attributes: HashMap<String, String>,
}

impl SVPrimitive {
    /// Create a new LUT primitive with size `k`, instance name `name`, and program `program`
    pub fn new_lut(k: usize, name: String, program: u64) -> Self {
        let mut attributes = HashMap::new();
        attributes.insert("program".to_string(), format!("{}", program));
        attributes.insert("size".to_string(), format!("{}", k));
        SVPrimitive {
            prim: format!("LUT{}", k),
            name,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
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
            "I0" | "I1" | "I2" | "I3" | "I4" | "I5" => self.add_input(port, signal),
            "O" | "Y" => self.add_output(port, signal),
            _ => return Err("Unknown port name".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents a Verilog Module. For now it can only have one output.
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
        }
    }

    /// Set the file name of the module
    pub fn with_fname(self, fname: String) -> Self {
        SVModule {
            fname: Some(fname),
            ..self
        }
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

    /// Get the driving primitive for a signal
    pub fn get_driving_primitive(&self, signal: &str) -> Result<&SVPrimitive, String> {
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
                    let prim: Vec<&str> = mod_name.split("LUT").collect();
                    if prim.len() != 2 || !prim[0].is_empty() {
                        return Err(format!(
                            "Expected a LUT primitive. Found primitive {}",
                            mod_name
                        ));
                    }
                    let size = match prim.last().unwrap().parse::<usize>() {
                        Ok(x) => x,
                        Err(_) => return Err(format!("Could not parse LUT size for {}", mod_name)),
                    };
                    let id = unwrap_node!(inst, NamedParameterAssignment).unwrap();
                    let program: u64 =
                        if let RefNode::HexValue(v) = unwrap_node!(id, HexValue).unwrap() {
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
                        } else {
                            return Err(format!(
                                "LUT {} should have INIT value written in hexadecimal",
                                mod_name
                            ));
                        };
                    cur_insts.push(SVPrimitive::new_lut(size, inst_name, program));
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
                    let port_name = get_identifier(port, ast);
                    let arg = unwrap_node!(connection, Expression).unwrap();
                    let arg = unwrap_node!(arg, HierarchicalIdentifier).unwrap();
                    let arg_name = get_identifier(arg, ast);
                    cur_insts
                        .last_mut()
                        .unwrap()
                        .add_signal(port_name.unwrap(), arg_name.unwrap())?;
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
                _ => (),
            }
        }

        if modules.len() != 1 {
            return Err("Expected exactly one module".to_string());
        }

        Ok(modules.pop().unwrap())
    }

    fn get_expr(&self, signal: &str, expr: &mut RecExpr<LutLang>) -> Result<Id, String> {
        match self.get_driving_primitive(signal) {
            Ok(primitive) => {
                let mut subexpr: Vec<Id> = vec![];
                let program: u64 = primitive.attributes["program"].parse().unwrap();
                subexpr.push(expr.add(LutLang::Program(program)));
                for input in (0..primitive.inputs.len()).rev().map(|x| format!("I{}", x)) {
                    let driver = primitive
                        .inputs
                        .get(&input)
                        .expect("Expect LUT to have input driven");
                    subexpr.push(self.get_expr(driver, expr)?);
                }
                Ok(expr.add(LutLang::Lut(subexpr.into())))
            }
            Err(e) => {
                if self.is_an_input(signal) {
                    Ok(expr.add(LutLang::Var(signal.into())))
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Convert the module to a [LutLang] expression
    pub fn to_expr(&self) -> Result<RecExpr<LutLang>, String> {
        if self.outputs.len() != 1 {
            return Err(format!(
                "{}: Expected exactly one output in module {}.",
                self.fname.clone().unwrap_or("".to_string()),
                self.name
            ));
        }

        let root = &self.outputs.first().unwrap().name;
        let mut expr = RecExpr::default();
        self.get_expr(root, &mut expr)?;
        Ok(expr)
    }
}
