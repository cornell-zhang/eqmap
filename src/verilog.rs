/*!

  Parses structural verilog and convert it into a [SVModule] struct.

*/

use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet, hash_map::Entry},
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

use egg::{Id, Language, RecExpr};
use sv_parser::{Identifier, Locate, NodeEvent, RefNode, unwrap_node};

use super::asic::CellLang;
use super::logic::{Logic, dont_care};
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

/// Parse a literal `node` in the `ast` into a four-state logic value
fn parse_literal_as_logic(node: RefNode, ast: &sv_parser::SyntaxTree) -> Result<Logic, String> {
    let value = unwrap_node!(node, BinaryValue, HexValue, UnsignedNumber);

    if value.is_none() {
        return Err(
            "Expected a BinaryValue, HexValue, or UnsignedNumber Node under the Literal"
                .to_string(),
        );
    }

    match value.unwrap() {
        RefNode::BinaryValue(b) => {
            let loc = b.nodes.0;
            let val = ast.get_str(&loc).unwrap();
            if val == "x" {
                return Ok(Logic::X);
            }
            let num = u64::from_str_radix(val, 2)
                .map_err(|_e| format!("Could not parse binary value {val} as bool"))?;
            match num {
                1 => Ok(true.into()),
                0 => Ok(false.into()),
                _ => Err(format!("Expected a 1 bit constant. Found {num}")),
            }
        }
        RefNode::HexValue(b) => {
            let loc = b.nodes.0;
            let val = ast.get_str(&loc).unwrap();
            if val == "x" {
                return Ok(Logic::X);
            }
            let num = u64::from_str_radix(val, 16)
                .map_err(|_e| format!("Could not parse hex value {val} as bool"))?;
            match num {
                1 => Ok(true.into()),
                0 => Ok(false.into()),
                _ => Err(format!("Expected a 1 bit constant. Found {num}")),
            }
        }
        RefNode::UnsignedNumber(b) => {
            let loc = b.nodes.0;
            let val = ast.get_str(&loc).unwrap();
            if val == "x" {
                return Ok(Logic::X);
            }
            let num = val
                .parse::<u64>()
                .map_err(|_e| format!("Could not parse decimal value {val} as bool"))?;
            match num {
                1 => Ok(true.into()),
                0 => Ok(false.into()),
                _ => Err(format!("Expected a 1 bit constant. Found {num}")),
            }
        }
        _ => unreachable!(),
    }
}

fn init_format(program: u64, k: usize) -> Result<String, ()> {
    let w = 1 << k;
    match k {
        1 => Ok(format!("{w}'h{program:01x}")),
        2 => Ok(format!("{w}'h{program:01x}")),
        3 => Ok(format!("{w}'h{program:02x}")),
        4 => Ok(format!("{w}'h{program:04x}")),
        5 => Ok(format!("{w}'h{program:08x}")),
        6 => Ok(format!("{w}'h{program:016x}")),
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

/// Escaped identifiers in Verilog must have a dangling space to end the escaped sequence.
fn emit_id(id: String) -> String {
    if id.starts_with("\\") { id + " " } else { id }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents a signal declaration in the verilog
struct SVSignal {
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

#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    AND,
    NAND,
    OR,
    NOR,
    XOR,
    XNOR,
    NOT,
    INV,
    AND2,
    NAND2,
    OR2,
    NOR2,
    XOR2,
    XNOR2,
    AND3,
    NAND3,
    OR3,
    NOR3,
    AND4,
    NAND4,
    OR4,
    NOR4,
    MUX,
    MUX2,
    MUXF7,
    MUXF8,
    MUXF9,
    AOI21,
    OAI21,
    AOI211,
    AOI22,
    OAI211,
    OAI22,
    OAI221,
    AOI221,
    OAI222,
    AOI222,
    LUT1,
    LUT2,
    LUT3,
    LUT4,
    LUT5,
    LUT6,
    VCC,
    GND,
    FDRE,
    MAJ,
}

impl PrimitiveType {
    /// Return the number of inputs of the primitive of this type
    pub fn get_num_inputs(&self) -> usize {
        match self {
            Self::AND2 | Self::AND => 2,
            Self::NAND2 | Self::NAND => 2,
            Self::OR2 | Self::OR => 2,
            Self::NOR2 | Self::NOR => 2,
            Self::XOR2 | Self::XOR => 2,
            Self::XNOR2 | Self::XNOR => 2,
            Self::NOT | Self::INV | Self::LUT1 => 1,
            Self::MUX | Self::MUX2 | Self::MUXF7 | Self::MUXF8 | Self::MUXF9 => 3,
            Self::AND3 | Self::NAND3 | Self::OR3 | Self::NOR3 => 3,
            Self::AND4 | Self::NAND4 | Self::OR4 | Self::NOR4 => 4,
            Self::AOI21 | Self::OAI21 => 3,
            Self::AOI211 | Self::AOI22 | Self::OAI211 | Self::OAI22 => 4,
            Self::AOI221 | Self::OAI221 => 5,
            Self::AOI222 | Self::OAI222 => 6,
            Self::LUT2 => 2,
            Self::LUT3 => 3,
            Self::LUT4 => 4,
            Self::LUT5 => 5,
            Self::LUT6 => 6,
            Self::VCC | Self::GND => 0,
            Self::FDRE => 1,
            Self::MAJ => 3,
        }
    }

    /// Get the list of inputs for the primitive
    pub fn get_input_list(&self) -> Vec<String> {
        match self {
            Self::AND
            | Self::NAND
            | Self::OR
            | Self::NOR
            | Self::XOR
            | Self::XNOR
            | Self::XOR2
            | Self::XNOR2 => {
                vec!["A".to_string(), "B".to_string()]
            }
            Self::INV | Self::NOT => vec!["A".to_string()],
            Self::AND2 | Self::NAND2 | Self::OR2 | Self::NOR2 => {
                vec!["A1".to_string(), "A2".to_string()]
            }
            Self::AND3 | Self::NAND3 | Self::OR3 | Self::NOR3 | Self::MAJ => {
                vec!["A1".to_string(), "A2".to_string(), "A3".to_string()]
            }
            Self::AND4 | Self::NAND4 | Self::OR4 | Self::NOR4 => {
                vec![
                    "A1".to_string(),
                    "A2".to_string(),
                    "A3".to_string(),
                    "A4".to_string(),
                ]
            }
            Self::MUX => {
                vec!["S".to_string(), "A".to_string(), "B".to_string()]
            }
            Self::MUX2 => {
                vec!["S".to_string(), "B".to_string(), "A".to_string()]
            }
            Self::MUXF7 | Self::MUXF8 | Self::MUXF9 => {
                vec!["S".to_string(), "I1".to_string(), "I0".to_string()]
            }
            Self::AOI21 | Self::OAI21 => vec!["A".to_string(), "B1".to_string(), "B2".to_string()],
            Self::AOI22 | Self::OAI22 => vec![
                "A1".to_string(),
                "A2".to_string(),
                "B1".to_string(),
                "B2".to_string(),
            ],
            Self::AOI211 | Self::OAI211 => vec![
                "A".to_string(),
                "B".to_string(),
                "C1".to_string(),
                "C2".to_string(),
            ],
            Self::AOI221 | Self::OAI221 => vec![
                "A".to_string(),
                "B1".to_string(),
                "B2".to_string(),
                "C1".to_string(),
                "C2".to_string(),
            ],
            Self::AOI222 | Self::OAI222 => vec![
                "A1".to_string(),
                "A2".to_string(),
                "B1".to_string(),
                "B2".to_string(),
                "C1".to_string(),
                "C2".to_string(),
            ],
            Self::LUT1 => vec!["I0".to_string()],
            Self::LUT2 => vec!["I1".to_string(), "I0".to_string()],
            Self::LUT3 => vec!["I2".to_string(), "I1".to_string(), "I0".to_string()],
            Self::LUT4 => vec![
                "I3".to_string(),
                "I2".to_string(),
                "I1".to_string(),
                "I0".to_string(),
            ],
            Self::LUT5 => vec![
                "I4".to_string(),
                "I3".to_string(),
                "I2".to_string(),
                "I1".to_string(),
                "I0".to_string(),
            ],
            Self::LUT6 => vec![
                "I5".to_string(),
                "I4".to_string(),
                "I3".to_string(),
                "I2".to_string(),
                "I1".to_string(),
                "I0".to_string(),
            ],
            Self::VCC | Self::GND => vec![],
            Self::FDRE => vec!["D".to_string()],
        }
    }

    /// Get the name of the output port for the primitive type
    pub fn get_output(&self) -> String {
        match self {
            Self::AND
            | Self::NAND
            | Self::OR
            | Self::NOR
            | Self::XOR
            | Self::XNOR
            | Self::NOT
            | Self::MUX => "Y".to_string(),
            Self::LUT1
            | Self::LUT2
            | Self::LUT3
            | Self::LUT4
            | Self::LUT5
            | Self::LUT6
            | Self::MUXF7
            | Self::MUXF8
            | Self::MUXF9 => "O".to_string(),
            Self::VCC => "P".to_string(),
            Self::GND => "G".to_string(),
            Self::FDRE => "Q".to_string(),
            Self::MUX2 | Self::XOR2 => "Z".to_string(),
            _ => "ZN".to_string(),
        }
    }

    /// Returns true if the primitive is a k-LUT
    pub fn is_lut(&self) -> bool {
        matches!(
            self,
            Self::LUT1 | Self::LUT2 | Self::LUT3 | Self::LUT4 | Self::LUT5 | Self::LUT6
        )
    }

    /// Returns true if the primitive is not a LUT
    pub fn is_gate(&self) -> bool {
        !self.is_lut() && !matches!(self, Self::VCC | Self::GND | Self::FDRE)
    }

    /// Returns true if the primitive is a register (FDRE)
    pub fn is_reg(&self) -> bool {
        matches!(self, Self::FDRE)
    }

    /// Get the area of a minimum sized primitive of [PrimitiveType]
    pub fn get_min_area(&self) -> Option<f32> {
        match self {
            Self::AND2 => Some(1.064),
            Self::AND3 => Some(1.33),
            Self::AND4 => Some(1.596),
            Self::AOI21 => Some(1.064),
            Self::AOI22 => Some(1.33),
            Self::AOI211 => Some(1.33),
            Self::AOI221 => Some(1.596),
            Self::AOI222 => Some(2.128),
            Self::INV => Some(0.532),
            Self::MUX2 => Some(1.862),
            Self::NAND2 => Some(0.798),
            Self::NAND3 => Some(1.064),
            Self::NAND4 => Some(1.33),
            Self::NOR2 => Some(0.798),
            Self::NOR3 => Some(1.064),
            Self::NOR4 => Some(1.33),
            Self::OAI21 => Some(1.064),
            Self::OAI22 => Some(1.33),
            Self::OAI211 => Some(1.33),
            Self::OAI221 => Some(1.596),
            Self::OAI222 => Some(2.128),
            Self::OR2 => Some(1.064),
            Self::OR3 => Some(1.33),
            Self::OR4 => Some(1.596),
            Self::XNOR2 => Some(1.596),
            Self::XOR2 => Some(1.596),
            Self::MAJ => Some(1.064),
            _ => None,
        }
    }
}

impl FromStr for PrimitiveType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pre = match s.split_once("_X") {
            Some((p, _)) => p,
            None => s,
        };

        match pre {
            "INV" => Ok(Self::INV),
            "MUX" => Ok(Self::MUX),
            "AND2" => Ok(Self::AND2),
            "NAND2" => Ok(Self::NAND2),
            "OR2" => Ok(Self::OR2),
            "NOR2" => Ok(Self::NOR2),
            "XOR2" => Ok(Self::XOR2),
            "XNOR2" => Ok(Self::XNOR2),
            "AND3" => Ok(Self::AND3),
            "NAND3" => Ok(Self::NAND3),
            "OR3" => Ok(Self::OR3),
            "NOR3" => Ok(Self::NOR3),
            "AND4" => Ok(Self::AND4),
            "NAND4" => Ok(Self::NAND4),
            "OR4" => Ok(Self::OR4),
            "NOR4" => Ok(Self::NOR4),
            "AOI21" => Ok(Self::AOI21),
            "OAI21" => Ok(Self::OAI21),
            "AOI211" => Ok(Self::AOI211),
            "AOI22" => Ok(Self::AOI22),
            "OAI211" => Ok(Self::OAI211),
            "OAI22" => Ok(Self::OAI22),
            "AOI221" => Ok(Self::AOI221),
            "OAI221" => Ok(Self::OAI221),
            "AOI222" => Ok(Self::AOI222),
            "OAI222" => Ok(Self::OAI222),
            "MUX2" => Ok(Self::MUX2),
            "AND" => Ok(Self::AND),
            "NAND" => Ok(Self::NAND),
            "OR" => Ok(Self::OR),
            "NOR" => Ok(Self::NOR),
            "XOR" => Ok(Self::XOR),
            "XNOR" => Ok(Self::XNOR),
            "NOT" => Ok(Self::NOT),
            "MUXF7" => Ok(Self::MUXF7),
            "MUXF8" => Ok(Self::MUXF8),
            "MUXF9" => Ok(Self::MUXF9),
            "LUT1" => Ok(Self::LUT1),
            "LUT2" => Ok(Self::LUT2),
            "LUT3" => Ok(Self::LUT3),
            "LUT4" => Ok(Self::LUT4),
            "LUT5" => Ok(Self::LUT5),
            "LUT6" => Ok(Self::LUT6),
            "VCC" => Ok(Self::VCC),
            "GND" => Ok(Self::GND),
            "FDRE" => Ok(Self::FDRE),
            "MAJ" => Ok(Self::MAJ),
            _ => Err(format!("Unknown primitive type {pre}")),
        }
    }
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// The [SVPrimitive] struct represents a primitive instance within a netlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SVPrimitive {
    /// The type of the primitive
    prim: String,
    /// The logic of the primitive
    logic: Option<PrimitiveType>,
    /// The name of the instance
    name: String,
    /// Number of inputs when this primitive is fully connected
    n_inputs: usize,
    /// Maps input ports to their signal driver
    /// E.g. (I0, a) means I0 is driven by signal a.
    inputs: BTreeMap<String, String>,
    /// Maps output signals to their port driver
    /// E.g. (y, O) means signal y is driven by output O.
    outputs: BTreeMap<String, String>,
    /// Stores arguments to module parameters as well as any other attribute
    attributes: BTreeMap<String, String>,
}

impl SVPrimitive {
    /// Read an attribute from the primitive
    pub fn get_attribute(&self, key: &str) -> Option<&String> {
        self.attributes.get(key)
    }

    /// Set an attribute on the primitive
    pub fn set_attribute(&mut self, key: String, val: String) -> Option<String> {
        self.attributes.insert(key, val)
    }

    /// Drive `signal` with named `port` of the primitive
    pub fn connect_output(&mut self, port: String, signal: String) -> Result<(), String> {
        if !self.outputs.is_empty() {
            let (y, o) = self.outputs.first_key_value().unwrap();
            return Err(format!("Signal {y} is already driven by {o}"));
        }
        self.outputs.insert(signal, port);
        Ok(())
    }

    /// The name of the signal the primitive is driving
    pub fn get_output_name(&self) -> Option<String> {
        self.outputs.keys().next().cloned()
    }

    /// Returns true if all the inputs are connected
    pub fn fully_driven(&self) -> bool {
        self.inputs.len() == self.n_inputs
    }

    /// Returns true if all the inputs *and* outputs are connected
    pub fn fully_connected(&self) -> bool {
        self.fully_driven() && !self.outputs.is_empty()
    }

    /// Returns the logic function of the primitive if it exists
    pub fn get_logic(&self) -> Option<PrimitiveType> {
        self.logic
    }

    /// Create a unconnected primitive `prim` with instance name `name` and `n_inputs` inputs
    pub fn new(prim: String, name: String, n_inputs: usize) -> Self {
        let mut prim = SVPrimitive {
            prim: prim.clone(),
            logic: PrimitiveType::from_str(prim.as_str()).ok(),
            name,
            n_inputs,
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            attributes: BTreeMap::new(),
        };

        if let Some(l) = prim.logic {
            match l {
                PrimitiveType::VCC => {
                    prim.set_attribute("VAL".to_string(), "1'b1".to_string());
                    return prim;
                }
                PrimitiveType::GND => {
                    prim.set_attribute("VAL".to_string(), "1'b0".to_string());
                    return prim;
                }
                PrimitiveType::FDRE => {
                    prim.set_attribute("INIT".to_string(), "1'hx".to_string());
                }
                _ => {}
            }
            prim.n_inputs = l.get_num_inputs();
        }

        prim
    }

    /// Sets the INIT attribute for a LUT primitive
    pub fn set_init(&mut self, val: u64) {
        let k = self.n_inputs;
        self.set_attribute("INIT".to_string(), init_format(val, k).unwrap());
    }

    /// Create a new unconnected LUT primitive with size `k`, instance name `name`, and program `program`
    pub fn new_lut(k: usize, name: String, program: u64) -> Self {
        let mut prim = Self::new(format!("{LUT_ROOT}{k}"), name, k);
        prim.set_init(program);
        prim
    }

    /// Create a new unconnected gate primitive with instance name `name`
    pub fn new_gate(logic: PrimitiveType, name: String) -> Self {
        let n_inputs: usize = logic.get_num_inputs();

        // Special cases
        let mut prim = SVPrimitive {
            prim: logic.to_string(),
            logic: Some(logic),
            name,
            n_inputs,
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            attributes: BTreeMap::new(),
        };
        match logic {
            PrimitiveType::VCC => {
                prim.set_attribute("VAL".to_string(), "1'b1".to_string());
                return prim;
            }
            PrimitiveType::GND => {
                prim.set_attribute("VAL".to_string(), "1'b0".to_string());
                return prim;
            }
            PrimitiveType::FDRE => {
                prim.set_attribute("INIT".to_string(), "1'hx".to_string());
            }
            _ => {}
        }
        prim
    }

    /// Create a new unconnected gate primitive with instance name `name` and `drive_strength`
    pub fn new_gate_with_strength(
        logic: PrimitiveType,
        name: String,
        drive_strength: usize,
    ) -> Self {
        let n_inputs: usize = logic.get_num_inputs();
        Self::new(format!("{logic}_X{drive_strength}"), name, n_inputs)
    }

    /// Create a new constant with name `name` and drive `signal` with it
    pub fn new_const(val: Logic, signal: String, name: String) -> Self {
        let mut prim = Self::new("CONST".to_string(), name, 0);
        prim.connect_output("Y".to_string(), signal).unwrap();
        prim.set_attribute("VAL".to_string(), val.as_str().to_string());
        prim
    }

    /// Create a new wire assignment with name `name` driven by `driver`
    pub fn new_wire(driver: String, signal: String, name: String) -> Self {
        let mut prim = Self::new("WIRE".to_string(), name, 0);
        prim.connect_output("Y".to_string(), signal).unwrap();
        prim.set_attribute("VAL".to_string(), driver);
        prim
    }

    /// Connect `signal` to `port` on the primitive
    pub fn connect_input(&mut self, port: String, signal: String) -> Result<(), String> {
        match self.inputs.insert(port.clone(), signal) {
            Some(d) => Err(format!(
                "Port {} is already driven on instance {} of {} by {}",
                port, self.name, self.prim, d
            )),
            None => Ok(()),
        }
    }

    /// Create an IO connection to the primitive based on port name. This is based on the Xilinx port naming conventions.
    pub fn connect_signal(&mut self, port: String, signal: String) -> Result<(), String> {
        match port.as_str() {
            "I" | "I0" | "I1" | "I2" | "I3" | "I4" | "I5" | "D" | "A" | "B" | "S" | "A1" | "A2"
            | "A3" | "A4" | "B1" | "B2" | "B3" | "C1" | "C2" | "C3" => {
                self.connect_input(port, signal)
            }
            "O" | "Y" | "Q" | "G" | "P" | "Z" | "ZN" => self.connect_output(port, signal),
            "C" | "CE" | "R" => Ok(()),
            _ => Err(format!("Unknown port name {port}")),
        }
    }

    fn is_gate(&self) -> bool {
        match &self.logic {
            Some(logic) => logic.is_gate(),
            None => PrimitiveType::from_str(&self.prim).is_ok_and(|p| p.is_gate()),
        }
    }

    fn is_lut(&self) -> bool {
        match &self.logic {
            Some(logic) => logic.is_lut(),
            None => PrimitiveType::from_str(&self.prim).is_ok_and(|p| p.is_lut()),
        }
    }

    fn is_reg(&self) -> bool {
        match &self.logic {
            Some(logic) => logic.is_reg(),
            None => PrimitiveType::from_str(&self.prim).is_ok_and(|p| p.is_reg()),
        }
    }

    fn is_const(&self) -> bool {
        matches!(self.prim.as_str(), "CONST" | "VCC" | "GND")
    }

    fn is_assign(&self) -> bool {
        self.is_const() || matches!(self.prim.as_str(), "WIRE")
    }
}

impl fmt::Display for SVPrimitive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let level = 2;
        let indent = " ".repeat(2);

        if self.is_assign() {
            return write!(
                f,
                "{}assign {} = {};",
                indent,
                self.outputs.keys().next().unwrap(),
                emit_id(self.attributes["VAL"].clone())
            );
        }

        writeln!(f, "{}{} #(", indent, self.prim)?;
        for (i, (key, value)) in self.attributes.iter().enumerate() {
            let indent = " ".repeat(level + 4);
            write!(f, "{indent}.{key}({value})")?;
            if i == self.attributes.len() - 1 {
                writeln!(f)?;
            } else {
                writeln!(f, ",")?;
            }
        }
        writeln!(f, "{}) {} (", indent, self.name)?;
        // TODO(matth2k): refactor as "is clocked"
        if self.prim.as_str() == REG_NAME {
            let indent = " ".repeat(level + 4);
            writeln!(f, "{indent}.C({CLK}),")?;
            writeln!(f, "{indent}.CE(1'h1),")?;
        }
        for (input, value) in self.inputs.iter() {
            let indent = " ".repeat(level + 4);
            writeln!(f, "{}.{}({}),", indent, input, emit_id(value.clone()))?;
        }
        if self.prim.as_str() == REG_NAME {
            let indent = " ".repeat(level + 4);
            writeln!(f, "{indent}.R(1'h0),")?;
        }
        for (i, (value, output)) in self.outputs.iter().enumerate() {
            let indent = " ".repeat(level + 4);
            write!(f, "{}.{}({})", indent, output, emit_id(value.clone()))?;
            if i == self.outputs.len() - 1 {
                writeln!(f)?;
            } else {
                writeln!(f, ",")?;
            }
        }
        write!(f, "{indent});")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents the connectivity of a Verilog module.
pub struct SVModule {
    /// The file name of the module
    fname: Option<String>,
    /// The name of the module
    name: String,
    /// All nets declared by the module (including inputs and outputs)
    signals: Vec<SVSignal>,
    /// All primitive instances in the module
    instances: Vec<SVPrimitive>,
    /// All input signals to the module
    inputs: Vec<SVSignal>,
    /// All output signals from the module
    outputs: Vec<SVSignal>,
    /// Returns the index of the primitive instance that drives a particular net
    driving_module: HashMap<String, usize>,
    /// Sequential and hence needs a clk
    clk: bool,
}

/// A trait for emitting Verilog code from a [Language] expression
pub trait VerilogEmission
where
    Self: Language + std::fmt::Display,
{
    /// Returns the primitive type (i.e. the logic) of the language node
    fn get_gate_type(&self) -> Option<PrimitiveType>;

    /// Returns all the ids which need to be mapped to Verilog outputs
    fn get_output_ids(expr: &RecExpr<Self>) -> Vec<Id>;

    /// Returns the variable name if the node is an input
    fn get_var(&self) -> Option<String>;

    /// Returns true if the node is an input/variable
    fn is_var(&self) -> bool {
        self.get_var().is_some()
    }

    /// Returns the fully connected [SVPrimitive] for this node.
    /// All the predecessors of this node must be already defined with `lookup`
    fn get_verilog_primitive(
        &self,
        lookup: impl Fn(&Id) -> Option<String>,
        fresh_prim_name: impl Fn() -> String,
        fresh_signal_name: impl Fn() -> String,
    ) -> Result<Option<SVPrimitive>, String>;
}

impl VerilogEmission for CellLang {
    fn get_gate_type(&self) -> Option<PrimitiveType> {
        match self {
            CellLang::And(_) => Some(PrimitiveType::AND),
            CellLang::Or(_) => Some(PrimitiveType::OR),
            CellLang::Inv(_) => Some(PrimitiveType::INV),
            CellLang::Cell(s, _) => PrimitiveType::from_str(s.as_str()).ok(),
            _ => None,
        }
    }

    fn get_output_ids(expr: &RecExpr<Self>) -> Vec<Id> {
        if expr.is_empty() {
            return vec![];
        }

        match expr.last().unwrap() {
            CellLang::Bus(l) => l.to_vec(),
            _ => vec![(expr.len() - 1).into()],
        }
    }

    fn get_var(&self) -> Option<String> {
        if let CellLang::Var(v) = self {
            Some(v.to_string())
        } else {
            None
        }
    }

    fn get_verilog_primitive(
        &self,
        lookup: impl Fn(&Id) -> Option<String>,
        fresh_prim_name: impl Fn() -> String,
        fresh_signal_name: impl Fn() -> String,
    ) -> Result<Option<SVPrimitive>, String> {
        match self {
            CellLang::And(_) | CellLang::Or(_) | CellLang::Inv(_) | CellLang::Cell(_, _) => {
                let inputs = self.children();
                let gate_type = self
                    .get_gate_type()
                    .ok_or("CellLang gates should have a primitive type".to_string())?;
                let port_list = gate_type.get_input_list();
                // TODO(matth2k): Carry through drive strength
                let mut prim = SVPrimitive::new_gate_with_strength(gate_type, fresh_prim_name(), 1);
                for (input, port) in inputs.iter().zip(port_list) {
                    let signal = lookup(input)
                        .ok_or(format!("Could not find signal {input} in the module"))?;
                    prim.connect_input(port, signal)?;
                }
                prim.connect_output(gate_type.get_output(), fresh_signal_name())?;
                Ok(Some(prim))
            }
            CellLang::Const(b) => Ok(Some(SVPrimitive::new_const(
                Logic::from(*b),
                fresh_signal_name(),
                fresh_prim_name(),
            ))),
            _ => Ok(None),
        }
    }
}

impl VerilogEmission for LutLang {
    fn get_gate_type(&self) -> Option<PrimitiveType> {
        match self {
            LutLang::And(_) => Some(PrimitiveType::AND),
            LutLang::Nor(_) => Some(PrimitiveType::NOR),
            LutLang::Not(_) => Some(PrimitiveType::NOT),
            LutLang::Lut(l) => match l.len() {
                2 => Some(PrimitiveType::LUT1),
                3 => Some(PrimitiveType::LUT2),
                4 => Some(PrimitiveType::LUT3),
                5 => Some(PrimitiveType::LUT4),
                6 => Some(PrimitiveType::LUT5),
                7 => Some(PrimitiveType::LUT6),
                _ => None,
            },
            LutLang::Mux(_) => Some(PrimitiveType::MUX),
            LutLang::Xor(_) => Some(PrimitiveType::XOR),
            LutLang::Reg(_) => Some(PrimitiveType::FDRE),
            _ => None,
        }
    }

    fn get_output_ids(expr: &RecExpr<Self>) -> Vec<Id> {
        if expr.is_empty() {
            return vec![];
        }

        match expr.last().unwrap() {
            LutLang::Bus(l) => l.to_vec(),
            _ => vec![(expr.len() - 1).into()],
        }
    }

    fn get_var(&self) -> Option<String> {
        if let LutLang::Var(v) = self {
            Some(v.to_string())
        } else {
            None
        }
    }

    fn get_verilog_primitive(
        &self,
        lookup: impl Fn(&Id) -> Option<String>,
        fresh_prim_name: impl Fn() -> String,
        fresh_signal_name: impl Fn() -> String,
    ) -> Result<Option<SVPrimitive>, String> {
        match self {
            LutLang::And(_)
            | LutLang::Mux(_)
            | LutLang::Nor(_)
            | LutLang::Not(_)
            | LutLang::Reg(_)
            | LutLang::Xor(_) => {
                let inputs = self.children();
                let gate_type = self
                    .get_gate_type()
                    .ok_or("LutLang gates should have a primitive type".to_string())?;
                let port_list = gate_type.get_input_list();
                let mut prim = SVPrimitive::new_gate(gate_type, fresh_prim_name());
                for (input, port) in inputs.iter().zip(port_list) {
                    let signal = lookup(input)
                        .ok_or(format!("Could not find signal {input} in the module"))?;
                    prim.connect_input(port, signal)?;
                }
                prim.connect_output(gate_type.get_output(), fresh_signal_name())?;
                Ok(Some(prim))
            }
            LutLang::Lut(l) => {
                let gate_type = self
                    .get_gate_type()
                    .expect("CellLang gates should have a primitive type");
                let port_list = gate_type.get_input_list();
                let mut prim = SVPrimitive::new_gate(gate_type, fresh_prim_name());
                for (input, port) in l.iter().skip(1).zip(port_list) {
                    let signal = lookup(input)
                        .ok_or(format!("Could not find signal {input} in the module"))?;
                    prim.connect_input(port, signal)?;
                }
                prim.connect_output(gate_type.get_output(), fresh_signal_name())?;
                Ok(Some(prim))
            }
            LutLang::Const(b) => Ok(Some(SVPrimitive::new_const(
                Logic::from(*b),
                fresh_signal_name(),
                fresh_prim_name(),
            ))),
            LutLang::DC => Ok(Some(SVPrimitive::new_const(
                dont_care(),
                fresh_signal_name(),
                fresh_prim_name(),
            ))),
            _ => Ok(None),
        }
    }
}

/// A trait for compiling Verilog code to a [Language] expression
pub trait VerilogParsing
where
    Self: Language + std::fmt::Display,
{
    /// Compile the Verilog signal `signal` in `module` into the `expr`.
    fn get_expr<'a>(
        signal: &'a str,
        module: &'a SVModule,
        expr: &mut RecExpr<Self>,
        map: &mut HashMap<&'a str, Id>,
    ) -> Result<Id, String>;

    /// Returns a mapping of the inputs to `prim` while also updating the root `map` as wel
    fn map_inputs<'a>(
        prim: &'a SVPrimitive,
        module: &'a SVModule,
        expr: &mut RecExpr<Self>,
        map: &mut HashMap<&'a str, Id>,
    ) -> Result<HashMap<&'a str, Id>, String> {
        let mut subexpr: HashMap<&'a str, Id> = HashMap::new();
        for (port, signal) in prim.inputs.iter() {
            subexpr.insert(port, Self::get_expr(signal, module, expr, map)?);
        }
        Ok(subexpr)
    }
}

impl VerilogParsing for CellLang {
    fn get_expr<'a>(
        signal: &'a str,
        module: &'a SVModule,
        expr: &mut RecExpr<CellLang>,
        map: &mut HashMap<&'a str, Id>,
    ) -> Result<Id, String> {
        if map.contains_key(signal) {
            return Ok(map[signal]);
        }

        let id = match module.get_driving_primitive(signal) {
            Ok(primitive) => {
                if primitive.is_gate() {
                    // Update the mapping
                    let subexpr = Self::map_inputs(primitive, module, expr, map)?;
                    let logic = primitive.get_logic().unwrap();
                    let ids: Vec<Result<Id, String>> = logic
                        .get_input_list()
                        .iter()
                        .map(|x| {
                            subexpr.get(x.as_str()).cloned().ok_or(format!(
                                "Missing input {} on {} {}",
                                x, logic, primitive.name
                            ))
                        })
                        .collect();
                    let ids = ids.into_iter().collect::<Result<Vec<Id>, String>>()?;
                    match logic {
                        PrimitiveType::AND => Ok(expr.add(CellLang::And([ids[0], ids[1]]))),
                        PrimitiveType::OR => Ok(expr.add(CellLang::Or([ids[0], ids[1]]))),
                        PrimitiveType::INV | PrimitiveType::NOT => {
                            Ok(expr.add(CellLang::Inv([ids[0]])))
                        }
                        _ => Ok(expr.add(CellLang::Cell(primitive.prim.clone().into(), ids))),
                    }
                } else if primitive.is_assign() {
                    let val = primitive.attributes.get("VAL").unwrap();
                    if primitive.is_const() {
                        let val = val.parse::<Logic>()?;
                        if val.is_dont_care() {
                            Err("Cannot use dont care in CellLang".to_string())
                        } else {
                            Ok(expr.add(CellLang::Const(val.unwrap())))
                        }
                    } else {
                        Self::get_expr(val.as_str(), module, expr, map)
                    }
                } else {
                    Err(format!("Unsupported cell primitive {}", primitive.prim))
                }
            }
            Err(e) => {
                if module.is_an_input(signal) {
                    Ok(expr.add(CellLang::Var(signal.into())))
                } else {
                    Err(e)
                }
            }
        }?;

        map.insert(signal, id);
        Ok(id)
    }
}

impl VerilogParsing for LutLang {
    fn get_expr<'a>(
        signal: &'a str,
        module: &'a SVModule,
        expr: &mut RecExpr<LutLang>,
        map: &mut HashMap<&'a str, Id>,
    ) -> Result<Id, String> {
        if map.contains_key(signal) {
            return Ok(map[signal]);
        }

        let id = match module.get_driving_primitive(signal) {
            Ok(primitive) => {
                if primitive.is_gate() || primitive.is_lut() || primitive.is_reg() {
                    // Update the mapping
                    let subexpr = Self::map_inputs(primitive, module, expr, map)?;
                    let logic = primitive.get_logic().unwrap();
                    let ids: Vec<Result<Id, String>> = if matches!(logic, PrimitiveType::INV) {
                        if let Some(a) = subexpr.get("A") {
                            vec![Ok(*a)]
                        } else if let Some(i) = subexpr.get("I") {
                            vec![Ok(*i)]
                        } else {
                            vec![Err("Expected A or I as input to NOT primitive".to_string())]
                        }
                    } else {
                        logic
                            .get_input_list()
                            .iter()
                            .map(|x| {
                                subexpr.get(x.as_str()).cloned().ok_or(format!(
                                    "Missing input {} on {} {}",
                                    x, logic, primitive.name
                                ))
                            })
                            .collect()
                    };
                    let mut ids = ids.into_iter().collect::<Result<Vec<Id>, String>>()?;
                    match logic {
                        PrimitiveType::AND | PrimitiveType::AND2 => {
                            Ok(expr.add(LutLang::And([ids[0], ids[1]])))
                        }
                        PrimitiveType::XOR | PrimitiveType::XOR2 => {
                            Ok(expr.add(LutLang::Xor([ids[0], ids[1]])))
                        }
                        PrimitiveType::NOR | PrimitiveType::NOR2 => {
                            Ok(expr.add(LutLang::Nor([ids[0], ids[1]])))
                        }
                        PrimitiveType::MUX
                        | PrimitiveType::MUXF7
                        | PrimitiveType::MUXF8
                        | PrimitiveType::MUXF9 => {
                            Ok(expr.add(LutLang::Mux([ids[0], ids[1], ids[2]])))
                        }
                        PrimitiveType::INV | PrimitiveType::NOT => {
                            Ok(expr.add(LutLang::Not([ids[0]])))
                        }
                        PrimitiveType::FDRE => Ok(expr.add(LutLang::Reg([ids[0]]))),
                        PrimitiveType::LUT1
                        | PrimitiveType::LUT2
                        | PrimitiveType::LUT3
                        | PrimitiveType::LUT4
                        | PrimitiveType::LUT5
                        | PrimitiveType::LUT6 => {
                            let program = primitive
                                .get_attribute("INIT")
                                .ok_or(format!("LUT {signal} has no INIT attribute"))?;
                            let program = init_parser(program)?;
                            let mut c = vec![expr.add(LutLang::Program(program))];
                            c.append(&mut ids);
                            Ok(expr.add(LutLang::Lut(c.into())))
                        }
                        _ => Err(format!(
                            "Unsupported gate primitive {} with logic {}",
                            primitive.prim, logic
                        )),
                    }
                } else if primitive.is_assign() {
                    let val = primitive.attributes.get("VAL").unwrap();
                    if primitive.is_const() {
                        let val = val.parse::<Logic>()?;
                        if val.is_dont_care() {
                            Ok(expr.add(LutLang::DC))
                        } else {
                            Ok(expr.add(LutLang::Const(val.unwrap())))
                        }
                    } else {
                        Self::get_expr(val.as_str(), module, expr, map)
                    }
                } else {
                    Err(format!("Unsupported gate primitive {}", primitive.prim))
                }
            }
            Err(e) => {
                if module.is_an_input(signal) {
                    Ok(expr.add(LutLang::Var(signal.into())))
                } else {
                    Err(e)
                }
            }
        }?;

        map.insert(signal, id);
        Ok(id)
    }
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
    fn append_insts(&mut self, insts: &mut Vec<SVPrimitive>) {
        let new_index = self.instances.len();
        for (i, inst) in insts.iter().enumerate() {
            for (signal, _port) in inst.outputs.iter() {
                self.driving_module.insert(signal.clone(), new_index + i);
            }
        }
        self.instances.append(insts);
    }

    /// Append a list of inputs to the module
    fn append_inputs(&mut self, inputs: &mut Vec<SVSignal>) {
        self.inputs.append(inputs);
    }

    /// Append a list of outputs to the module
    fn append_outputs(&mut self, outputs: &mut Vec<SVSignal>) {
        self.outputs.append(outputs);
    }

    /// Append a list of net declarations to the module
    fn append_signals(&mut self, outputs: &mut Vec<SVSignal>) {
        self.signals.append(outputs);
    }

    /// Names output `id` with `name` inside `self`
    fn name_output(&mut self, id: Id, name: String, mapping: &mut HashMap<Id, String>) {
        if let Entry::Vacant(e) = mapping.entry(id) {
            e.insert(name.clone());
        } else {
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
        }
        self.outputs.push(SVSignal::new(1, name));
    }

    /// Get the driving primitive for a signal
    fn get_driving_primitive<'a>(&'a self, signal: &'a str) -> Result<&'a SVPrimitive, String> {
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
        match PrimitiveType::from_str(name) {
            Ok(PrimitiveType::LUT1) => Some(1),
            Ok(PrimitiveType::LUT2) => Some(2),
            Ok(PrimitiveType::LUT3) => Some(3),
            Ok(PrimitiveType::LUT4) => Some(4),
            Ok(PrimitiveType::LUT5) => Some(5),
            Ok(PrimitiveType::LUT6) => Some(6),
            _ => None,
        }
    }

    fn add_clk(&mut self) {
        if !self.clk {
            self.clk = true;
            self.append_inputs(&mut vec![SVSignal::new(1, CLK.to_string())]);
        }
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
                                            "Could not parse hex value from INIT string {loc}"
                                        ));
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
                                            "Could not parse decimal value from INIT string {loc}"
                                        ));
                                    }
                                }
                            }
                            _ => {
                                return Err(format!(
                                    "{LUT_ROOT} {mod_name} should have INIT value written in hexadecimal"
                                ));
                            }
                        };
                        cur_insts.push(SVPrimitive::new_lut(k, inst_name, program));
                        continue;
                    }

                    if let Ok(p) = PrimitiveType::from_str(&mod_name) {
                        cur_insts.push(SVPrimitive::new(mod_name, inst_name, p.get_num_inputs()));
                        continue;
                    }

                    return Err(format!(
                        "Expected a {LUT_ROOT} or {REG_NAME} primitive. Found primitive {mod_name} {inst:?}"
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
                                .connect_signal(port_name, arg_name.unwrap())?;
                        }
                        None => {
                            // Ignore clock enable and reset signals,
                            // because they are not along the data path
                            // The verilog emitter just re-inserts them at the end
                            // This means we can only use D Flip-flops that are constantly *ON*.
                            if port_name == "CE" || port_name == "R" {
                                if unwrap_node!(arg, PrimaryLiteral).is_none() {
                                    return Err(format!(
                                        "Non-data port {port_name} should be driven constant"
                                    ));
                                }
                            } else {
                                // If we don't have a identifier, it must be a constant connection
                                let arg_name = cur_insts.last().unwrap().name.clone()
                                    + port_name.as_str()
                                    + "_const";
                                cur_signals.push(SVSignal::new(1, arg_name.clone()));
                                cur_insts
                                    .last_mut()
                                    .unwrap()
                                    .connect_signal(port_name.clone(), arg_name.clone())?;

                                // Create the constant
                                let literal = unwrap_node!(arg, PrimaryLiteral);
                                if literal.is_none() {
                                    return Err(format!(
                                        "Expected a literal for connection on port {port_name}"
                                    ));
                                }
                                let value = parse_literal_as_logic(literal.unwrap(), ast)?;
                                let const_inst = SVPrimitive::new_const(
                                    value,
                                    arg_name.clone(),
                                    arg_name.clone() + "_inst",
                                );

                                // Insert the constant before the current instance
                                cur_insts.insert(cur_insts.len() - 1, const_inst);
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
                    let rhs_id = unwrap_node!(rhs, Identifier, PrimaryLiteral).unwrap();
                    let assignment = unwrap_node!(net_assign, Symbol).unwrap();
                    match assignment {
                        RefNode::Symbol(sym) => {
                            let loc = sym.nodes.0;
                            let eq = ast.get_str(&loc).unwrap();
                            if eq != "=" {
                                return Err(format!("Expected an assignment operator, got {eq}"));
                            }
                        }
                        _ => {
                            return Err("Expected an assignment operator".to_string());
                        }
                    }
                    if matches!(rhs_id, RefNode::Identifier(_)) {
                        let rhs_name = get_identifier(rhs_id, ast).unwrap();
                        cur_insts.push(SVPrimitive::new_wire(
                            rhs_name.clone(),
                            lhs_name.clone(),
                            lhs_name + "_wire_" + &rhs_name,
                        ));
                    } else {
                        let val = parse_literal_as_logic(rhs_id, ast)?;
                        cur_insts.push(SVPrimitive::new_const(
                            val,
                            lhs_name.clone(),
                            lhs_name + "_const_binary",
                        ));
                    }
                }
                NodeEvent::Leave(RefNode::NetAssignment(_net_assign)) => (),

                // The stuff we definitely don't support
                NodeEvent::Enter(RefNode::BinaryOperator(_)) => {
                    return Err("Binary operators are not supported".to_string());
                }
                NodeEvent::Enter(RefNode::UnaryOperator(_)) => {
                    return Err("Unary operators are not supported".to_string());
                }
                NodeEvent::Enter(RefNode::Concatenation(_)) => {
                    return Err("Concatenation not supported".to_string());
                }
                NodeEvent::Enter(RefNode::AlwaysConstruct(_)) => {
                    return Err("Always block not supported".to_string());
                }
                NodeEvent::Enter(RefNode::ConditionalStatement(_)) => {
                    return Err("If/else block not supported".to_string());
                }
                _ => (),
            }
        }

        if modules.len() != 1 {
            return Err("Expected exactly one module".to_string());
        }

        Ok(modules.pop().unwrap())
    }

    fn insert_instance(&mut self, inst: SVPrimitive) -> Result<String, String> {
        if !inst.fully_connected() {
            return Err(format!(
                "Primitive {} is not fully connected before inserting into module",
                inst.name
            ));
        }
        let signal = inst.get_output_name().unwrap();
        self.signals.push(SVSignal::new(1, signal.clone()));
        self.driving_module
            .insert(signal.clone(), self.instances.len());
        self.instances.push(inst);
        Ok(signal)
    }

    fn insert_input(&mut self, var: String) -> Result<String, String> {
        let sname = var;
        if sname.contains("\n") || sname.contains(",") || sname.contains(";") {
            return Err("Input cannot span multiple lines or contain delimiters".to_string());
        }
        if sname.contains("tmp") {
            return Err("'tmp' is a reserved keyword".to_string());
        }
        if sname.contains(CLK) {
            return Err(format!("'{CLK}' is a reserved keyword"));
        }
        if sname.contains("input") {
            return Err("'input' is a reserved keyword".to_string());
        }
        let signal = SVSignal::new(1, sname.clone());
        self.signals.push(signal.clone());
        self.inputs.push(signal);

        Ok(sname)
    }

    /// Constructs a verilog module out of a [Language] expression.
    /// The module will be named `mod_name` and the outputs will be named from right to left with `outputs`.
    /// The default names for the outputs are `y0`, `y1`, etc. `outputs[0]` names the rightmost signal in a bus.
    pub fn from_cells<L>(
        expr: RecExpr<L>,
        mod_name: String,
        outputs: Vec<String>,
    ) -> Result<Self, String>
    where
        L: VerilogEmission,
    {
        let mut module = SVModule::new(mod_name);

        let mut mapping: HashMap<Id, String> = HashMap::new();

        // Add output mapping
        let out_ids = L::get_output_ids(&expr);
        if out_ids.len() > 1 {
            for (i, id) in out_ids.iter().enumerate() {
                let defname = format!("y{i}");
                module.name_output(
                    *id,
                    outputs.get(i).unwrap_or(&defname).to_string(),
                    &mut mapping,
                );
            }
        } else {
            module.name_output(
                *out_ids.first().unwrap(),
                outputs.first().unwrap_or(&"y".to_string()).to_string(),
                &mut mapping,
            );
        }

        let mut prim_count: usize = 0;
        for (i, l) in expr.as_ref().iter().enumerate() {
            if !mapping.contains_key(&i.into()) && !l.is_var() && i < expr.as_ref().len() - 1 {
                mapping.insert(i.into(), format!("__{prim_count}__"));
                prim_count += 1;
            }
        }

        let prim_count: RefCell<usize> = RefCell::new(prim_count);

        let fresh_prim = || {
            *prim_count.borrow_mut() += 1;
            format!("__{}__", *prim_count.borrow() - 1)
        };

        for (id, node) in expr.as_ref().iter().enumerate() {
            let fresh_wire = || {
                mapping.get(&id.into()).cloned().unwrap_or_else(|| {
                    *prim_count.borrow_mut() += 1;
                    format!("__{}__", *prim_count.borrow() - 1)
                })
            };
            if let Some(prim) =
                node.get_verilog_primitive(|x| mapping.get(x).cloned(), fresh_prim, fresh_wire)?
            {
                let sname = module.insert_instance(prim)?;
                mapping.insert(id.into(), sname);
            } else if let Some(v) = node.get_var() {
                let sname = module.insert_input(v)?;

                // Check if input directly drives an output
                if mapping.contains_key(&id.into()) {
                    let output = mapping[&id.into()].clone();
                    let wire = SVPrimitive::new_wire(sname.clone(), output.clone(), fresh_prim());
                    module
                        .driving_module
                        .insert(output.clone(), module.instances.len());
                    module.instances.push(wire);
                    module.signals.push(SVSignal::new(1, output));
                }
                mapping.insert(id.into(), sname);
            }
        }

        Ok(module)
    }

    /// Constructs a verilog module out of a [LutLang] expression.
    /// The module will be named `mod_name` and the outputs will be named from right to left with `outputs`.
    /// The default names for the outputs are `y0`, `y1`, etc. `outputs[0]` names the rightmost signal in a bus.
    pub fn from_luts(
        expr: RecExpr<LutLang>,
        mod_name: String,
        outputs: Vec<String>,
    ) -> Result<Self, String> {
        let mut module = SVModule::new(mod_name);
        let expr = LutExprInfo::new(&expr).get_cse();

        let mut mapping: HashMap<Id, String> = HashMap::new();

        // Add output mapping
        let out_ids = LutLang::get_output_ids(&expr);
        if out_ids.len() > 1 {
            for (i, id) in out_ids.iter().enumerate() {
                let defname = format!("y{i}");
                module.name_output(
                    *id,
                    outputs.get(i).unwrap_or(&defname).to_string(),
                    &mut mapping,
                );
            }
        } else {
            module.name_output(
                *out_ids.first().unwrap(),
                outputs.first().unwrap_or(&"y".to_string()).to_string(),
                &mut mapping,
            );
        }

        let mut prim_count: usize = 0;
        for (i, l) in expr.as_ref().iter().enumerate() {
            if !mapping.contains_key(&i.into())
                && !matches!(l, LutLang::Var(_) | LutLang::Program(_))
                && i < expr.as_ref().len() - 1
            {
                mapping.insert(i.into(), format!("__{prim_count}__"));
                prim_count += 1;
            }
        }

        let prim_count: RefCell<usize> = RefCell::new(prim_count);

        let fresh_prim = || {
            *prim_count.borrow_mut() += 1;
            format!("__{}__", *prim_count.borrow() - 1)
        };

        let mut programs: HashMap<Id, u64> = HashMap::new();

        for (id, node) in expr.as_ref().iter().enumerate() {
            let fresh_wire = || {
                mapping.get(&id.into()).cloned().unwrap_or_else(|| {
                    *prim_count.borrow_mut() += 1;
                    format!("__{}__", *prim_count.borrow() - 1)
                })
            };
            if let Some(mut prim) =
                node.get_verilog_primitive(|x| mapping.get(x).cloned(), fresh_prim, fresh_wire)?
            {
                if let LutLang::Lut(l) = node {
                    prim.set_init(programs[&l[0]]);
                }

                if matches!(node, LutLang::Reg(_)) {
                    module.add_clk();
                }

                let sname = module.insert_instance(prim)?;
                mapping.insert(id.into(), sname);
            } else if let Some(v) = node.get_var() {
                let sname = module.insert_input(v)?;

                // Check if input directly drives an output
                if mapping.contains_key(&id.into()) {
                    let output = mapping[&id.into()].clone();
                    let wire = SVPrimitive::new_wire(sname.clone(), output.clone(), fresh_prim());
                    module
                        .driving_module
                        .insert(output.clone(), module.instances.len());
                    module.instances.push(wire);
                    module.signals.push(SVSignal::new(1, output));
                }
                mapping.insert(id.into(), sname);
            } else if let LutLang::Program(p) = node {
                programs.insert(id.into(), *p);
            } else if !matches!(node, LutLang::Bus(_)) {
                return Err(format!("Unsupported node type: {node:?}"));
            }
        }

        Ok(module)
    }

    /// Get a separate [Language] expression for every output in the module
    pub fn get_exprs<L>(&self) -> Result<Vec<(String, RecExpr<L>)>, String>
    where
        L: VerilogParsing,
    {
        if let Err(s) = self.contains_cycles() {
            return Err(format!("Cannot convert module with feedback on signal {s}"));
        }

        let mut exprs = vec![];
        for output in self.outputs.iter() {
            let mut expr = RecExpr::default();
            L::get_expr(&output.name, self, &mut expr, &mut HashMap::new())?;
            exprs.push((output.name.clone(), expr));
        }
        Ok(exprs)
    }

    /// Get a single [LutLang] expression for the module as a bus
    pub fn to_single_lut_expr(&self) -> Result<RecExpr<LutLang>, String> {
        if let Err(s) = self.contains_cycles() {
            return Err(format!("Cannot convert module with feedback on signal {s}"));
        }

        let mut expr: RecExpr<LutLang> = RecExpr::default();
        let mut map = HashMap::new();
        let mut outputs: Vec<Id> = vec![];
        for output in self.outputs.iter() {
            outputs.push(LutLang::get_expr(&output.name, self, &mut expr, &mut map)?);
        }
        if outputs.len() > 1 {
            expr.add(LutLang::Bus(outputs.into()));
        }
        // TODO(matth2k): Add an option to run subexpression elimination here
        Ok(expr)
    }

    /// Get a single [CellLang] expression for the module as a bus
    pub fn to_single_cell_expr(&self) -> Result<RecExpr<CellLang>, String> {
        if let Err(s) = self.contains_cycles() {
            return Err(format!("Cannot convert module with feedback on signal {s}"));
        }

        let mut expr: RecExpr<CellLang> = RecExpr::default();
        let mut map = HashMap::new();
        let mut outputs: Vec<Id> = vec![];
        for output in self.outputs.iter() {
            outputs.push(CellLang::get_expr(&output.name, self, &mut expr, &mut map)?);
        }
        if outputs.len() > 1 {
            expr.add(CellLang::Bus(outputs.into()));
        }
        Ok(expr)
    }

    /// Convert the module to a [Language] expression
    pub fn to_expr<L>(&self) -> Result<RecExpr<L>, String>
    where
        L: VerilogParsing,
    {
        if let Err(s) = self.contains_cycles() {
            return Err(format!("Cannot convert module with feedback on signal {s}"));
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
        visited: &mut HashSet<&'a str>,
    ) -> Result<(), &'a str> {
        if walk.contains(signal) {
            return Err(signal);
        }
        if visited.contains(signal) {
            return Ok(());
        }
        walk.insert(signal);
        let driving = self.get_driving_primitive(signal);
        if driving.is_err() {
            walk.remove(signal);
            return Ok(());
        }
        let driving = driving.unwrap();
        for (_, driver) in driving.inputs.iter() {
            self.contains_cycles_rec(driver, walk, visited)?
        }
        walk.remove(signal);
        visited.insert(signal);
        Ok(())
    }

    /// We cannot lower verilog with cycles in it to LutLang expressions.
    /// This function returns [Ok] when there are no cycles in the module
    pub fn contains_cycles<'a>(&'a self) -> Result<(), &'a str> {
        let mut visited = HashSet::new();
        for output in self.outputs.iter() {
            let mut stack: HashSet<&'a str> = HashSet::new();
            self.contains_cycles_rec(output.get_name(), &mut stack, &mut visited)?
        }
        Ok(())
    }

    /// For each input in `other` that is not present in `self`, add it to the list of signals.
    /// It will remain undriven intentionally.
    pub fn append_inputs_from_module(&mut self, other: &Self) {
        for l in &other.inputs {
            if self.inputs.iter().any(|x| x == l) {
                continue;
            }
            self.append_inputs(&mut vec![l.clone()]);
        }
    }
}

impl fmt::Display for SVModule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let level = 0;
        let indent = " ".repeat(level);
        let mut already_decl: HashSet<String> = HashSet::new();
        writeln!(f, "{}module {} (", indent, self.name)?;
        for input in self.inputs.iter() {
            if already_decl.contains(&input.name) {
                continue;
            }
            let indent = " ".repeat(level + 4);
            writeln!(f, "{}{},", indent, emit_id(input.name.clone()))?;
            already_decl.insert(input.name.clone());
        }
        for (i, output) in self.outputs.iter().enumerate() {
            let indent = " ".repeat(level + 4);
            write!(f, "{}{}", indent, emit_id(output.name.clone()))?;
            if i == self.outputs.len() - 1 {
                writeln!(f)?;
            } else {
                writeln!(f, ",")?;
            }
        }
        writeln!(f, "{indent});")?;
        already_decl.clear();
        for input in self.inputs.iter() {
            if already_decl.contains(&input.name) {
                continue;
            }
            let indent = " ".repeat(level + 2);
            writeln!(f, "{}input {};", indent, emit_id(input.name.clone()))?;
            writeln!(f, "{}wire {};", indent, emit_id(input.name.clone()))?;
            already_decl.insert(input.name.clone());
        }
        for output in self.outputs.iter() {
            let indent = " ".repeat(level + 2);
            writeln!(f, "{}output {};", indent, emit_id(output.name.clone()))?;
            writeln!(f, "{}wire {};", indent, emit_id(output.name.clone()))?;
            already_decl.insert(output.name.clone());
        }
        for signal in self.signals.iter() {
            let indent = " ".repeat(level + 2);
            if !already_decl.contains(&signal.name) {
                writeln!(f, "{}wire {};", indent, emit_id(signal.name.clone()))?;
                already_decl.insert(signal.name.clone());
            }
        }
        for instance in self.instances.iter() {
            writeln!(f, "{instance}")?;
        }
        writeln!(f, "{indent}endmodule")
    }
}

#[test]
fn test_primitive_connections() {
    let mut prim = SVPrimitive::new_lut(4, "_0_".to_string(), 1);
    assert!(
        prim.connect_signal("I8".to_string(), "a".to_string())
            .is_err()
    );
    assert!(
        prim.connect_signal("I0".to_string(), "a".to_string())
            .is_ok()
    );
    assert!(
        prim.connect_signal("I0".to_string(), "a".to_string())
            .is_err()
    );
    assert!(
        prim.connect_signal("Y".to_string(), "b".to_string())
            .is_ok()
    );
    assert!(
        prim.connect_signal("Y".to_string(), "b".to_string())
            .is_err()
    );
    assert!(
        prim.connect_signal("bad".to_string(), "a".to_string())
            .is_err()
    );
}

#[test]
fn test_parse_verilog() {
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
        endmodule"
        .to_string();
    let ast = sv_parse_wrapper(&module, None).unwrap();
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
    assert_eq!(instance.attributes.len(), 1);
    assert_eq!(instance.attributes["INIT"], "64'hf0f0ccccff00aaaa");
}

#[test]
fn test_cellang_emission() {
    let expr: RecExpr<CellLang> = "(AND2_X1 a b)".parse().unwrap();
    let prim = expr.last().unwrap().get_verilog_primitive(
        |x| {
            if *x == Id::from(0) {
                Some("a".to_string())
            } else {
                Some("b".to_string())
            }
        },
        || "and_inst".to_string(),
        || "y".to_string(),
    );
    assert!(prim.is_ok());
    let prim = prim.unwrap().unwrap();
    assert_eq!(
        prim.to_string(),
        "  AND2_X1 #(\n  ) and_inst (\n      .A1(a),\n      .A2(b),\n      .ZN(y)\n  );"
    );
}
