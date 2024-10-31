/*!

  Parse verilog

*/

use std::collections::HashMap;

use sv_parser::{unwrap_node, Identifier, Locate, NodeEvent, RefNode};

/// A wrapper for parsing verilog at file `path` with content `s`
pub fn sv_parse_wrapper(
    s: &str,
    path: &std::path::Path,
) -> Result<sv_parser::SyntaxTree, sv_parser::Error> {
    let incl: Vec<std::path::PathBuf> = vec![];
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
    prim: String,
    /// The name of the instance
    name: String,
    /// Maps input ports to their signal driver
    inputs: HashMap<String, String>,
    /// Maps output signals to their port driver
    outputs: HashMap<String, String>,
    /// Stores arguments to module parameters as well as any other attribute
    attributes: HashMap<String, String>,
}

impl SVPrimitive {
    /// Create a new empty primitive with module name `prim` and instance name `name`
    pub fn new(prim: String, name: String) -> Self {
        SVPrimitive {
            prim,
            name,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            attributes: HashMap::new(),
        }
    }

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
        match self.inputs.insert(port, signal) {
            Some(_) => Err("Port is already driven".to_string()),
            None => Ok(()),
        }
    }

    /// Add an output connection
    fn add_output(&mut self, port: String, signal: String) -> Result<(), String> {
        match self.outputs.insert(signal, port) {
            Some(_) => Err("Signal is already driven".to_string()),
            None => Ok(()),
        }
    }

    /// Create an IO connection to the primitive based on port name. This is based on the Xilinx port naming conventions.
    pub fn add_signal(&mut self, port: String, signal: String) -> Result<(), String> {
        match port.as_str() {
            "I0" | "I1" | "I2" | "I3" | "I4" | "I5" => self.add_input(port, signal),
            "O" | "Y" => self.add_output(port, signal),
            _ => panic!("Unknown port name"),
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
                    if prim.len() != 2 || prim[0] != "" {
                        return Err("Expected LUT primitive".to_string());
                    }
                    let size = match usize::from_str_radix(prim.last().unwrap(), 10) {
                        Ok(x) => x,
                        Err(_) => return Err("Expected LUT primitive".to_string()),
                    };
                    let id = unwrap_node!(inst, NamedParameterAssignment).unwrap();
                    let program: u64 =
                        if let RefNode::HexValue(v) = unwrap_node!(id, HexValue).unwrap() {
                            let loc = v.nodes.0;
                            let loc = ast.get_str(&loc).unwrap();
                            match u64::from_str_radix(loc, 16) {
                                Ok(x) => x,
                                Err(_) => return Err("Expected hex value INIT string".to_string()),
                            }
                        } else {
                            return Err("Expected hex value INIT string".to_string());
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
}

#[test]
fn test_signal_visit() {
    let module = "module mux_4_1 (
            a,
            b,
            c,
            d,
            s0,
            s1,
            y
        );
          input a;
          wire a;
          input b;
          wire b;
          input c;
          wire c;
          input d;
          wire d;
          input s0;
          wire s0;
          input s1;
          wire s1;
          output y;
          wire y;
          LUT6 #(
              .INIT(64'hf0f0ccccff00aaaa)
          ) _0_ (
              .I0(d),
              .I1(c),
              .I2(a),
              .I3(b),
              .I4(s1),
              .I5(s0),
              .O (y)
          );
        endmodule";
    let ast = sv_parse_wrapper(module, std::path::Path::new("mux_4_1.v")).unwrap();
    let module = SVModule::from_ast(&ast);
    assert!(module.is_ok());
    let module = module.unwrap();
    assert_eq!(module.instances.len(), 1);
    assert_eq!(module.inputs.len(), 6);
    assert_eq!(module.outputs.len(), 1);
    assert_eq!(module.name, "mux_4_1");
    let instance = module.instances.first().unwrap();
    assert_eq!(instance.prim, "LUT6");
    assert_eq!(instance.name, "_0_");
    assert_eq!(instance.attributes.len(), 2);
    assert_eq!(instance.attributes["program"], "17361601744336890538");
}
