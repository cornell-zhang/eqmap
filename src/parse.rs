/*!

  Parse verilog

*/

use std::{collections::HashMap, path::PathBuf};

use sv_parser::{unwrap_node, Identifier, Locate, NetDeclaration, Node, NodeEvent, RefNode};

/// This prints out the source name for modules, nets, and ports
pub fn get_identifier(node: RefNode, ast: &sv_parser::SyntaxTree) -> Result<String, String> {
    // unwrap_node! can take multiple types
    let id: Option<Locate> =
        match unwrap_node!(node, SimpleIdentifier, EscapedIdentifier, NetIdentifier) {
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
/// Represent a signal declaration in the verilog
pub struct SVSignal {
    /// The bitwidth of the signal
    bw: usize,
    /// The decl name of the signal
    name: String,
}

impl SVSignal {
    pub fn new(bw: usize, name: String) -> Self {
        SVSignal { bw, name }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SVInstance {
    module: String,
    name: String,
    inputs: HashMap<String, String>,
    outputs: HashMap<String, String>,
}

impl SVInstance {
    pub fn new(module: String, name: String) -> Self {
        SVInstance {
            module,
            name,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
        }
    }

    pub fn add_input(&mut self, port: String, signal: String) {
        self.inputs.insert(port, signal);
    }

    pub fn add_output(&mut self, port: String, signal: String) {
        self.outputs.insert(port, signal);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SVModule {
    pub fname: Option<String>,
    pub name: String,
    pub signals: Vec<SVSignal>,
    pub instances: Vec<SVInstance>,
    pub inputs: Vec<SVSignal>,
    pub outputs: Vec<SVSignal>,
}

impl SVModule {
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

    pub fn append_insts(&mut self, insts: &mut Vec<SVInstance>) {
        self.instances.append(insts);
    }

    pub fn append_inputs(&mut self, inputs: &mut Vec<SVSignal>) {
        self.inputs.append(inputs);
    }

    pub fn append_outputs(&mut self, outputs: &mut Vec<SVSignal>) {
        self.outputs.append(outputs);
    }

    pub fn append_signals(&mut self, outputs: &mut Vec<SVSignal>) {
        self.signals.append(outputs);
    }

    pub fn from_ast(ast: &sv_parser::SyntaxTree) -> Result<Vec<Self>, String> {
        let mut modules = vec![];
        let mut cur_insts: Vec<SVInstance> = vec![];
        let mut cur_inputs: Vec<SVSignal> = vec![];
        let mut cur_outputs: Vec<SVSignal> = vec![];
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
                    cur_insts.push(SVInstance::new(mod_name, inst_name));
                }
                NodeEvent::Leave(RefNode::ModuleInstantiation(_inst)) => (),

                // Handle input decl

                // Handle output decl

                // Handle instance args
                NodeEvent::Enter(RefNode::NamedPortConnection(connection)) => {
                    let port = unwrap_node!(connection, PortIdentifier).unwrap();
                    let port_name = get_identifier(port, ast);
                    let arg = unwrap_node!(connection, Expression).unwrap();
                    let arg = unwrap_node!(arg, HierarchicalIdentifier).unwrap();
                    let arg_name = get_identifier(arg, ast);
                    // TODO: check if port is input or output
                    cur_insts
                        .last_mut()
                        .unwrap()
                        .add_input(port_name.unwrap(), arg_name.unwrap());
                }
                NodeEvent::Leave(RefNode::NamedPortConnection(connection)) => (),

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

        Ok(modules)
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
            yEnter
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
    let incl: Vec<PathBuf> = vec![];
    let (ast, _defs) =
        sv_parser::parse_sv_str(module, "verilog", &HashMap::new(), &incl, true, true).unwrap();
    let signals = SVModule::from_ast(&ast).unwrap();
    eprintln!("{:?}", signals);
    assert_eq!(signals.len(), 1);
}
