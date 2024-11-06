/*!

  The lut module defines the grammar used to represent LUTs, gates, and principal inputs.

*/
use std::collections::HashMap;

use super::analysis::LutAnalysis;
use bitvec::prelude::*;
use egg::define_language;
use egg::CostFunction;
use egg::Id;
use egg::Language;
use egg::RecExpr;
use egg::Symbol;

define_language! {
    /// Definitions of e-node types. [Program] is the only node type that is not a net/signal.
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
                "NOR" | "LUT" | "MUX" | "AND" | "XOR" | "NOT" | "BUS" => Err(
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

    /// Evaluates the boolean value of a [LutLang] node contained in `expr` given the input state `inputs`
    fn eval_rec(&self, inputs: &HashMap<String, bool>, expr: &RecExpr<Self>) -> BitVec {
        match self {
            LutLang::Const(b) => bitvec!(usize, Lsb0; *b as usize; 1),
            LutLang::Var(s) => bitvec!(usize, Lsb0; *inputs.get(s.as_str()).unwrap() as usize; 1),
            LutLang::Program(_) => panic!("Program node should not be evaluated"),
            LutLang::DC => bitvec!(usize, Lsb0; 0; 1),
            LutLang::Nor(a) => {
                let a0 = &a[0];
                let a1 = &a[1];
                !(expr[*a0].eval_rec(inputs, expr) | expr[*a1].eval_rec(inputs, expr))
            }
            LutLang::And(a) => {
                let a0 = &a[0];
                let a1 = &a[1];
                expr[*a0].eval_rec(inputs, expr) & expr[*a1].eval_rec(inputs, expr)
            }
            LutLang::Xor(a) => {
                let a0 = &a[0];
                let a1 = &a[1];
                expr[*a0].eval_rec(inputs, expr) ^ expr[*a1].eval_rec(inputs, expr)
            }
            LutLang::Not(a) => {
                let a0 = &a[0];
                !expr[*a0].eval_rec(inputs, expr)
            }
            LutLang::Mux(a) => {
                let a0 = &a[0];
                let a1 = &a[1];
                let a2 = &a[2];
                let sel = expr[*a0].eval_rec(inputs, expr);
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
                let x: Vec<bool> = a[1..]
                    .iter()
                    .map(|id| expr[*id].eval_rec(inputs, expr)[0])
                    .collect();

                let t = eval_lut(p, &x);
                bitvec!(usize, Lsb0; t as usize; 1)
            }
            LutLang::Bus(a) => {
                let mut bv: BitVec = BitVec::with_capacity(a.len());
                for id in a.iter().rev() {
                    bv.push(expr[*id].eval_rec(inputs, expr)[0]);
                }
                bv
            }
        }
    }

    /// This funcion evaluates the `expr` under the certain `inputs`
    pub fn eval(expr: &RecExpr<Self>, inputs: &HashMap<String, bool>) -> BitVec {
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
            | (LutLang::Bus(_), LutLang::Bus(_)) => {
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
    /// this funcion returns true if they represent the same boolean function
    pub fn func_equiv(expr: &RecExpr<Self>, other: &RecExpr<Self>) -> bool {
        let root = &expr[(expr.as_ref().len() - 1).into()];
        let inputs = root.get_input_set(expr);
        for i in 0..1 << inputs.len() {
            let input_map = inputs
                .iter()
                .cloned()
                .zip((0..inputs.len()).map(|j| (i >> j) & 1 == 1))
                .collect();

            if !Self::eval(expr, &input_map) == Self::eval(other, &input_map) {
                return false;
            }
        }
        true
    }
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
            | LutLang::Bus(_) => 0,
        };
        enode.fold(op_cost, |sum, id| sum + costs(id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A struct to facilitate certain analyses on LUT expressions.
/// For example, finding common subexpressions, testing if a expression is canonical,
/// getting lut counts, or model checking.
pub struct LutExprInfo {
    /// The expression
    expr: RecExpr<LutLang>,
    /// The root of the expression
    root: Id,
}

impl LutExprInfo {
    /// Create a new LutExprInfo from a given expression.
    pub fn new(expr: RecExpr<LutLang>) -> Self {
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
                expr: self.expr.clone(),
                root,
            })
        }
    }

    /// This funcion returns true if the expression represents the same boolean function
    pub fn check(&self, other: &RecExpr<LutLang>) -> bool {
        LutLang::func_equiv(&self.expr, other)
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

        if node.deep_equals(other_node, &self.expr) {
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
        NumKLUTsCostFn::new(LutSize::Any).cost_rec(&self.expr)
    }

    /// Returns the number of k-luts in the given expr.
    pub fn get_lut_count_k(&self, k: usize) -> u64 {
        let size = LutSize::Size(k);
        NumKLUTsCostFn::new(size).cost_rec(&self.expr)
    }
}
