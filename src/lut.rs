/*!

  The lut module defines the grammar used to represent LUTs, gates, and principal inputs.

*/
use std::collections::HashMap;

use super::analysis::LutAnalysis;
use super::check::{equivalent, inconclusive, not_equivalent, Check};
use bitvec::prelude::*;
use egg::define_language;
use egg::CostFunction;
use egg::Id;
use egg::Language;
use egg::RecExpr;
use egg::Symbol;

define_language! {
    /// Definitions of e-node types. Programs are the only node type that is not a net/signal.
    #[allow(missing_docs)]
    pub enum LutLang {
        Const(bool),
        Program(u64), // The only node type that is not a net
        Var(Symbol),
        "x" = DC,
        "NOR" = Nor([Id; 2]),
        "MUX" = Mux([Id; 3]), // s, a, b
        "AND" = And([Id; 2]),
        "XOR" = Xor([Id; 2]),
        "NOT" = Not([Id; 1]),
        "LUT" = Lut(Box<[Id]>), // Program is first
        "BUS" = Bus(Box<[Id]>), // a bus of nodes
        "REG" = Reg([Id; 1]),
    }
}

impl LutLang {
    /// Maximum size allowed for a LUT.
    /// This cannot be made larger due to us using `u64` to represent LUTs. FPGAs generally do not
    /// support greater than 6-LUTs so this limit should hopefully be sufficient forever.
    pub const MAX_LUT_SIZE: usize = 6;

    /// Verify the grammar of a single [LutLang] node
    fn verify(&self) -> Result<(), String> {
        match self {
            LutLang::Lut(list) => {
                let l = list.len();
                if l == 0 {
                    Err("LUT must have at least one element".to_string())
                } else if l - 1 > Self::MAX_LUT_SIZE {
                    return Err(format!(
                        "Only {}-Luts or smaller supported",
                        Self::MAX_LUT_SIZE
                    ));
                } else {
                    Ok(())
                }
            }
            LutLang::Var(f) => match f.as_str() {
                "NOR" | "LUT" | "MUX" | "AND" | "XOR" | "NOT" | "BUS" | "DC" | "x" | "REG" => Err(
                    "Variable name is already reserved. Check for missing parentheses.".to_string(),
                ),
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    }

    /// Recursively verify the grammar of a [LutLang] expression `expr` rooted at `self`
    pub fn verify_rec(&self, expr: &RecExpr<Self>) -> Result<(), String> {
        self.verify()?;
        for c in self.children() {
            let t = &expr[*c];
            t.verify_rec(expr)?;
            if let LutLang::Lut(l) = self {
                if let LutLang::Program(p) = expr[l[0]] {
                    let k = l.len() - 1;
                    if k < Self::MAX_LUT_SIZE && p >= (1 << (1 << k)) {
                        return Err("Program too large for LUT".to_string());
                    }
                } else {
                    return Err("LUT must have a program".to_string());
                }
            } else if let LutLang::Bus(l) = self {
                for id in l.iter() {
                    if let LutLang::Program(_) = expr[*id] {
                        return Err("Bus cannot contain a program".to_string());
                    } else if let LutLang::Bus(_) = expr[*id] {
                        return Err("Bus construct cannot be nested".to_string());
                    }
                }
            }
        }
        Ok(())
    }

    /// Extract the program from a [LutLang::Lut] contained in expression `expr`
    pub fn get_program(&self, expr: &RecExpr<Self>) -> Result<u64, String> {
        match self {
            LutLang::Lut(l) => {
                self.verify()?;
                let p = l.first().unwrap();
                match expr[*p] {
                    LutLang::Program(p) => Ok(p),
                    _ => Err("First element of LUT must be a program".to_string()),
                }
            }
            _ => Err("Not a LUT".to_string()),
        }
    }

    /// Extract the program from a [LutLang::Lut] contained in `egraph`
    pub fn get_program_in_egraph(
        &self,
        egraph: &egg::EGraph<LutLang, LutAnalysis>,
    ) -> Result<u64, String> {
        match self {
            LutLang::Lut(l) => {
                self.verify()?;
                let p = l.first().unwrap();
                let class = &egraph[*p];
                let data = &class.data;
                data.get_program()
            }
            _ => Err("Not a LUT".to_string()),
        }
    }

    /// Extract the operand class ids from a [LutLang::Lut] contained in `egraph`
    pub fn get_operand_classes(
        &self,
        _egraph: &egg::EGraph<LutLang, LutAnalysis>,
    ) -> Result<Vec<Id>, String> {
        match self {
            LutLang::Lut(l) => {
                self.verify()?;
                Ok(Vec::from(&l[1..]))
            }
            LutLang::And(_)
            | LutLang::Xor(_)
            | LutLang::Nor(_)
            | LutLang::Mux(_)
            | LutLang::Not(_)
            | LutLang::Bus(_) => Ok(Vec::from(self.children())),
            _ => Err("Not a LUT or gate".to_string()),
        }
    }

    /// Returns the fan-in of a [LutLang::Lut]
    pub fn get_lut_size(&self) -> Result<usize, String> {
        match self {
            LutLang::Lut(l) => {
                self.verify()?;
                Ok(l.len() - 1)
            }
            _ => Err("Not a LUT".to_string()),
        }
    }

    /// Evaluates the combinational logic of a [LutLang] node contained in `expr` given the input state `inputs`
    fn eval_rec(
        &self,
        inputs: &HashMap<String, bool>,
        expr: &RecExpr<Self>,
    ) -> Result<BitVec, String> {
        match self {
            LutLang::Const(b) => Ok(bitvec!(usize, Lsb0; *b as usize; 1)),
            LutLang::Var(s) => match inputs.get(s.as_str()) {
                Some(b) => Ok(bitvec!(usize, Lsb0; *b as usize; 1)),
                None => Err(format!("Input {} is not driven", s.as_str())),
            },
            LutLang::Program(_) => panic!("Program node should not be evaluated"),
            LutLang::DC => Err("DC".to_string()),
            LutLang::Nor(a) => {
                let a0 = &a[0];
                let a1 = &a[1];
                // Implement short-circuiting
                let or_res = match (
                    expr[*a0].eval_rec(inputs, expr),
                    expr[*a1].eval_rec(inputs, expr),
                ) {
                    (Ok(a), Ok(b)) => Ok(a | b),
                    (Err(e), Ok(a)) | (Ok(a), Err(e)) => {
                        if a.eq(&bitvec!(usize, Lsb0; 1; a.len())) {
                            Ok(bitvec!(usize, Lsb0; 1; a.len()))
                        } else {
                            Err(e)
                        }
                    }
                    (Err(e), Err(_)) => Err(e),
                };
                match or_res {
                    Ok(a) => Ok(!a),
                    Err(e) => Err(e),
                }
            }
            LutLang::And(a) => {
                let a0 = &a[0];
                let a1 = &a[1];
                // Implement short-circuiting
                match (
                    expr[*a0].eval_rec(inputs, expr),
                    expr[*a1].eval_rec(inputs, expr),
                ) {
                    (Ok(a), Ok(b)) => Ok(a & b),
                    (Err(e), Ok(a)) | (Ok(a), Err(e)) => {
                        if a.eq(&bitvec!(usize, Lsb0; 0; a.len())) {
                            Ok(bitvec!(usize, Lsb0; 0; a.len()))
                        } else {
                            Err(e)
                        }
                    }
                    (Err(e), Err(_)) => Err(e),
                }
            }
            LutLang::Xor(a) => {
                let a0 = &a[0];
                let a1 = &a[1];
                Ok(expr[*a0].eval_rec(inputs, expr)? ^ expr[*a1].eval_rec(inputs, expr)?)
            }
            LutLang::Not(a) => {
                let a0 = &a[0];
                Ok(!expr[*a0].eval_rec(inputs, expr)?)
            }
            LutLang::Mux(a) => {
                let a0 = &a[0];
                let a1 = &a[1];
                let a2 = &a[2];
                let sel = expr[*a0].eval_rec(inputs, expr)?;
                let len = self.len();
                if sel.ge(&bitvec!(usize, Lsb0; 0; len)) {
                    expr[*a1].eval_rec(inputs, expr)
                } else {
                    expr[*a2].eval_rec(inputs, expr)
                }
            }
            LutLang::Lut(a) => {
                let p = match expr[*a.first().unwrap()] {
                    LutLang::Program(p) => p,
                    _ => panic!("First element of LUT must be a program"),
                };

                let mut x: Vec<bool> = Vec::new();
                for operand in &a[1..] {
                    x.push(expr[*operand].eval_rec(inputs, expr)?[0]);
                }

                let t = eval_lut(p, &x);
                Ok(bitvec!(usize, Lsb0; t as usize; 1))
            }
            LutLang::Bus(a) => {
                let mut bv: BitVec = BitVec::with_capacity(a.len());
                for id in a.iter().rev() {
                    bv.push(expr[*id].eval_rec(inputs, expr)?[0]);
                }
                Ok(bv)
            }
            LutLang::Reg(_) => Err("REG is not combinational logic".to_string()),
        }
    }

    /// This funcion evaluates the `expr` as combinational logic under given `inputs`
    pub fn eval(expr: &RecExpr<Self>, inputs: &HashMap<String, bool>) -> Result<BitVec, String> {
        expr[(expr.as_ref().len() - 1).into()].eval_rec(inputs, expr)
    }

    /// Since variables/leaves can be duplicated in expressions, we sometimes need to do deep checks for equality.
    /// This function returns true if the two nodes contained in `expr` are equal.
    pub fn deep_equals(&self, other: &Self, expr: &RecExpr<Self>) -> bool {
        if self == other {
            return true;
        }

        if self.children().len() != other.children().len() {
            return false;
        }

        match (self, other) {
            (LutLang::Lut(_), LutLang::Lut(_))
            | (LutLang::And(_), LutLang::And(_))
            | (LutLang::Xor(_), LutLang::Xor(_))
            | (LutLang::Nor(_), LutLang::Nor(_))
            | (LutLang::Not(_), LutLang::Not(_))
            | (LutLang::Mux(_), LutLang::Mux(_))
            | (LutLang::Bus(_), LutLang::Bus(_))
            | (LutLang::Reg(_), LutLang::Reg(_)) => {
                for (a, b) in self.children().iter().zip(other.children()) {
                    if !expr[*a].deep_equals(&expr[*b], expr) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn get_inputs(&self) -> Vec<String> {
        match self {
            LutLang::Var(s) => vec![s.as_str().to_string()],
            _ => Vec::new(),
        }
    }

    fn get_inputs_rec(&self, expr: &RecExpr<Self>) -> Vec<String> {
        let mut inputs = self.get_inputs();
        let mut children_inputs: Vec<String> = self
            .children()
            .iter()
            .flat_map(|c| expr[*c].get_inputs_rec(expr))
            .collect();
        inputs.append(&mut children_inputs);
        inputs
    }

    fn get_input_set(&self, expr: &RecExpr<Self>) -> Vec<String> {
        let mut inputs = self.get_inputs_rec(expr);
        inputs.sort();
        inputs.dedup();
        inputs
    }

    /// Given two expressions and a set of input values,
    /// this funcion returns true if they represent the same combinational logic
    pub fn func_equiv(expr: &RecExpr<Self>, other: &RecExpr<Self>) -> Check {
        // First double check for structural equality
        if deep_equals(expr, other) {
            return equivalent();
        }

        let root = &expr[(expr.as_ref().len() - 1).into()];
        let inputs = root.get_input_set(expr);
        for i in 0..1 << inputs.len() {
            let input_map = inputs
                .iter()
                .cloned()
                .zip((0..inputs.len()).map(|j| (i >> j) & 1 == 1))
                .collect();

            match (Self::eval(expr, &input_map), Self::eval(other, &input_map)) {
                (Ok(e), Ok(o)) => {
                    if e != o {
                        return not_equivalent();
                    }
                }
                _ => return inconclusive(),
            }
        }
        equivalent()
    }
}

/// Given two expressions, concatenate them and find if their roots are structurally equivalent
pub fn deep_equals(expr: &RecExpr<LutLang>, other: &RecExpr<LutLang>) -> bool {
    let concat = concat_expr(expr, other);
    let r0 = &concat[(expr.as_ref().len() - 1).into()];
    let r1 = &concat[(concat.as_ref().len() - 1).into()];
    r0.deep_equals(r1, &concat)
}

fn concat_expr<L>(expr: &RecExpr<L>, other: &RecExpr<L>) -> RecExpr<L>
where
    L: Language,
{
    let rlen = expr.as_ref().len();

    let mut remap: Vec<L> = other
        .as_ref()
        .iter()
        .cloned()
        .map(|n| n.map_children(|c| (usize::from(c) + rlen).into()))
        .collect();

    let mut newexpr: Vec<L> = expr.as_ref().to_vec();
    newexpr.append(&mut remap);
    RecExpr::from(newexpr)
}

/// Verify the grammar of a [LutLang] expression from its root
pub fn verify_expr(expr: &RecExpr<LutLang>) -> Result<(), String> {
    expr.as_ref().last().unwrap().verify_rec(expr)?;
    Ok(())
}

/// Evaluates the boolean value of a Lut program given a slice of [bool] inputs (msb first).
pub fn eval_lut(p: u64, inputs: &[bool]) -> bool {
    let mut index = 0;
    for (i, input) in inputs.iter().rev().enumerate() {
        if *input {
            index += 1 << i;
        }
    }
    (p >> index) & 1 == 1
}

/// Convert a [u64] LUT program to a lsb-first [BitVec] of length `capacity`
pub fn to_bitvec(p: u64, capacity: usize) -> Result<BitVec, String> {
    if capacity > 64 {
        return Err("Capacity must be less than or equal to 64".to_string());
    }
    let maxval = if capacity == 64 {
        u64::MAX
    } else {
        (1 << capacity) - 1
    };
    if p > maxval {
        return Err(format!(
            "Program value {} is too large for capacity {}",
            p, capacity
        ));
    }
    let mut bv: BitVec = bitvec!(usize, Lsb0; 0; capacity);
    bv[0..capacity].store::<u64>(p);
    Ok(bv)
}

/// Convert a lsb-first [BitVec] LUT program to a [u64]
pub fn from_bitvec(bv: &BitVec) -> u64 {
    assert!(bv.len() <= 64);
    bv[0..bv.len()].load::<u64>()
}

/// Evaluates the boolean value of a LUT program given a [BitVec] (lsb first).
pub fn eval_lut_bv(p: u64, inputs: &BitVec) -> bool {
    let mut index = 0;
    assert!(inputs.len() <= 64);
    for (i, input) in inputs.iter().enumerate() {
        if *input {
            index += 1 << i;
        }
    }
    (p >> index) & 1 == 1
}

/// Returns true if LUT is invariant to the last operand
pub fn lut_invariant_to_last_operand(p: &u64) -> bool {
    for j in (0..=64).step_by(2) {
        let bit = (p >> j) & 1;
        let val = (p >> j + 1) & 1;
        if bit != val {
            return false;
        }
    }
    return true;
}

/// Removes last operand from LUT. Assumes LUT is invariant to last operand.
pub fn remove_last_var_from_lut(p: &u64) -> u64 {
    let mut mod_program: u64 = 0;
    for i in (0..=64).rev() {
        if i % 2 == 0 {
            let bit = (p >> i) & 1;
            mod_program = (mod_program << 1) | bit;
        }
    }
    return mod_program;
}

/// Return a partially-evaluated LUT program with the `msb` input tied to the constant `v`
pub fn eval_lut_const_input(p: &u64, msb: usize, v: bool) -> u64 {
    assert!(msb <= 5);
    assert_eq!(msb >> (1 << (msb + 1)), 0);
    if v {
        p >> (1 << msb)
    } else {
        p & ((1 << (1 << msb)) - 1)
    }
}

/// Swap the truth table for input `pos` and input `pos + 1`, where `pos` is offset from the lsb.
/// Together these generate the permutation group.
pub fn swap_pos(bv: &u64, k: usize, pos: usize) -> u64 {
    assert!(pos < k - 1);
    let mut table: Vec<BitVec> = Vec::new();
    for i in 0..(1 << k) {
        table.push(to_bitvec(i, k).unwrap());
    }

    // Swap the bit at `pos` in the truth table entries. Then use those entries to index the new
    // LUT program.
    for entry in table.iter_mut().take(1 << k) {
        let tmp = entry[pos];
        let tmp2 = entry[pos + 1];
        entry.set(pos, tmp2);
        entry.set(pos + 1, tmp);
    }
    let mut nbv: BitVec = bitvec!(usize, Lsb0; 0; 1 << k);
    for (i, entry) in table.iter().enumerate().take(1 << k) {
        let index = from_bitvec(entry) as usize;
        nbv.set(index, eval_lut_bv(*bv, &to_bitvec(i as u64, k).unwrap()));
    }
    from_bitvec(&nbv)
}

/// The size of a given LUT.
enum LutSize {
    /// A LUT of given size.
    Size(usize),
    /// A LUT of any size. A wildcard in a sense.
    Any,
}

/// A cost function counting LUTs of a given size.
struct NumKLUTsCostFn {
    size: LutSize,
}

impl NumKLUTsCostFn {
    /// Returns a new cost function counting LUTs of `size`.
    pub fn new(size: LutSize) -> Self {
        const TOO_LARGE: usize = LutLang::MAX_LUT_SIZE + 1;
        match size {
            LutSize::Size(0) | LutSize::Size(TOO_LARGE..) => {
                panic!("k must be between 1 and {}", LutLang::MAX_LUT_SIZE)
            }
            size => Self { size },
        }
    }
}

impl CostFunction<LutLang> for NumKLUTsCostFn {
    type Cost = u64;
    fn cost<C>(&mut self, enode: &LutLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let op_cost = match enode {
            LutLang::Lut(l) => match self.size {
                LutSize::Size(k) if l.len() == k + 1 => 1,
                LutSize::Any => 1,
                LutSize::Size(_) => 0,
            },
            LutLang::Program(_)
            | LutLang::Const(_)
            | LutLang::Var(_)
            | LutLang::DC
            | LutLang::Nor(_)
            | LutLang::Mux(_)
            | LutLang::And(_)
            | LutLang::Xor(_)
            | LutLang::Not(_)
            | LutLang::Bus(_)
            | LutLang::Reg(_) => 0,
        };
        enode.fold(op_cost, |sum, id| sum + costs(id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A struct to facilitate certain analyses on LUT expressions.
/// For example, finding common subexpressions, testing if a expression is canonical,
/// getting lut counts, or model checking.
pub struct LutExprInfo<'a> {
    /// The expression
    expr: &'a RecExpr<LutLang>,
    /// The root of the expression
    root: Id,
}

impl<'a> LutExprInfo<'a> {
    /// Create a new LutExprInfo from a given expression.
    pub fn new(expr: &'a RecExpr<LutLang>) -> Self {
        let root = (expr.as_ref().len() - 1).into();
        Self { expr, root }
    }

    /// Return a copy of the expression.
    pub fn get_expr(&self) -> RecExpr<LutLang> {
        self.expr.clone()
    }

    /// Return the root Id
    pub fn get_root(&self) -> Id {
        self.root
    }

    /// Look at a subexpression rooted at `root`.
    pub fn with_root(&self, root: Id) -> Option<Self> {
        if root >= self.expr.as_ref().len().into() {
            None
        } else {
            Some(Self {
                expr: self.expr,
                root,
            })
        }
    }

    /// This funcion returns true if the expression represents the same boolean function
    pub fn check(&self, other: &RecExpr<LutLang>) -> Check {
        LutLang::func_equiv(self.expr, other)
    }

    /// Return whether node `node` dominates node `other` within the expression.
    /// This function calls [LutLang::deep_equals], so this is an expensive call.
    pub fn dominates(&self, n: Id, other: Id) -> Result<bool, String> {
        let largest_id: Id = (self.expr.as_ref().len() - 1).into();
        if n > largest_id || other > largest_id {
            return Err("Node id out of bounds".to_string());
        }

        if n == other {
            return Ok(true);
        }

        let other_node = &self.expr[other];
        let node = &self.expr[n];

        if node.deep_equals(other_node, self.expr) {
            return Ok(true);
        }

        for child in self.expr[other].children() {
            if self.dominates(n, *child)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Returns the number of luts in the given expr.
    pub fn get_lut_count(&self) -> u64 {
        NumKLUTsCostFn::new(LutSize::Any).cost_rec(self.expr)
    }

    /// Returns the number of k-luts in the given expr.
    pub fn get_lut_count_k(&self, k: usize) -> u64 {
        let size = LutSize::Size(k);
        NumKLUTsCostFn::new(size).cost_rec(self.expr)
    }

    /// Returns `true` is the expression has common subexpressions that need to be eliminated
    pub fn is_reduntant(&self) -> bool {
        let slice = self.expr.as_ref();

        for i in 0..slice.len() {
            let n = &slice[i];

            // We honestly don't care about redundant program leaves
            if matches!(n, LutLang::Program(_)) {
                continue;
            }

            for o in slice.iter().skip(i + 1) {
                if n.deep_equals(o, self.expr) {
                    return true;
                }
            }
        }

        false
    }

    /// Returns `true` if the expression contains unmapped logic gates
    pub fn contains_gates(&self) -> bool {
        let slice = self.expr.as_ref();

        for n in slice {
            match n {
                LutLang::And(_)
                | LutLang::Xor(_)
                | LutLang::Nor(_)
                | LutLang::Mux(_)
                | LutLang::Not(_) => return true,
                _ => (),
            }
        }
        false
    }

    /// Returns `true` if the expression is canonical
    pub fn is_canonical(&self) -> bool {
        !(self.is_reduntant() || self.contains_gates())
    }
}
