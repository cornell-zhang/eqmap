/*!

  The rewrite module defines the collection of rewrite rules for LUT networks.
  These rules roughly fall into 5 categories:
  Shannon decomposition, general cut-fusion, general cut-decomposition (using DSD), LUT symmetry, constant evaluation, and gate conversion.
  Gate conversion is only present for the sake of running without structural synthesis.

  This module also contains all the appliers for LUT rewrite rules. This code is the most delicate and requires the most testing.

*/
use super::analysis::LutAnalysis;
use super::lut;
use super::lut::to_bitvec;
use bitvec::{bitvec, order::Lsb0, vec::BitVec};
use egg::{rewrite, Analysis, Applier, Pattern, PatternAst, Rewrite, Subst, Var};
use std::collections::{HashMap, HashSet};

/// Returns a list of structural mappings of logic functions to LUTs.
/// For example, MUXes are mapped to 3-LUTs and AND gates to 2-LUTs.
pub fn struct_lut_map<A>() -> Vec<Rewrite<lut::LutLang, A>>
where
    A: Analysis<lut::LutLang> + std::default::Default,
{
    let mut rules: Vec<Rewrite<lut::LutLang, A>> = Vec::new();
    // Logic element conversions
    rules.push(rewrite!("nor2-conversion"; "(NOR ?a ?b)" => "(LUT 1 ?a ?b)"));
    rules.push(rewrite!("and2-conversion"; "(AND ?a ?b)" => "(LUT 8 ?a ?b)"));
    rules.push(rewrite!("xor2-conversion"; "(XOR ?a ?b)" => "(LUT 6 ?a ?b)"));
    rules.push(rewrite!("inverter-conversion"; "(NOT ?a)" => "(LUT 1 ?a)"));
    // s? a : b
    rules.push(rewrite!("mux2-1-conversion"; "(MUX ?s ?a ?b)" => "(LUT 202 ?s ?a ?b)"));

    rules
}

/// Move registers around LUTs
pub fn register_retiming<A>() -> Vec<Rewrite<lut::LutLang, A>>
where
    A: Analysis<lut::LutLang> + std::default::Default,
{
    let mut rules: Vec<Rewrite<lut::LutLang, A>> = Vec::new();
    // Logic element conversions
    rules.append(&mut rewrite!("lut1-retime"; "(LUT ?p (REG ?a))" <=> "(REG (LUT ?p ?a))"));
    rules.append(
        &mut rewrite!("lut2-retime"; "(LUT ?p (REG ?a) (REG ?b))" <=> "(REG (LUT ?p ?a ?b))"),
    );
    rules.append(
        &mut rewrite!("lut3-retime"; "(LUT ?p (REG ?a) (REG ?b) (REG ?c))" <=> "(REG (LUT ?p ?a ?b ?c))"),
    );
    rules.append(
        &mut rewrite!("lut4-retime"; "(LUT ?p (REG ?a) (REG ?b) (REG ?c) (REG ?d))" <=> "(REG (LUT ?p ?a ?b ?c ?d))"),
    );
    rules.append(
        &mut rewrite!("lut5-retime"; "(LUT ?p (REG ?a) (REG ?b) (REG ?c) (REG ?d) (REG ?e))" <=> "(REG (LUT ?p ?a ?b ?c ?d ?e))"),
    );
    rules.append(
        &mut rewrite!("lut6-retime"; "(LUT ?p (REG ?a) (REG ?b) (REG ?c) (REG ?d) (REG ?e) (REG ?f))" <=> "(REG (LUT ?p ?a ?b ?c ?d ?e ?f))"),
    );

    rules
}

/// Returns a list of rules for permuting inputs in LUTs. Each instance of these rules forms a group under composition (https://en.wikipedia.org/wiki/Symmetric_group).
/// Each of these groups have k-1 generators.
pub fn permute_groups() -> Vec<Rewrite<lut::LutLang, LutAnalysis>> {
    let mut rules: Vec<Rewrite<lut::LutLang, LutAnalysis>> = Vec::new();
    // LUT permutation groups
    rules.push(rewrite!("lut2-permute"; "(LUT ?p ?a ?b)" 
        => {PermuteInput::new(1, "?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap()])}));

    for i in 1..3 {
        let rname = format!("lut3-permute-{}", i);
        rules.push(rewrite!(rname; "(LUT ?p ?a ?b ?c)" 
        => {PermuteInput::new(i, "?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap()])}));
    }

    for i in 1..4 {
        let rname = format!("lut4-permute-{}", i);
        rules.push(rewrite!(rname; "(LUT ?p ?a ?b ?c ?d)" 
        => {PermuteInput::new(i, "?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?d".parse().unwrap()])}));
    }

    for i in 1..5 {
        let rname = format!("lut5-permute-{}", i);
        rules.push(rewrite!(rname; "(LUT ?p ?a ?b ?c ?d ?e)" 
        => {PermuteInput::new(i, "?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?d".parse().unwrap(), "?e".parse().unwrap()])}));
    }

    for i in 1..6 {
        let rname = format!("lut6-permute-{}", i);
        rules.push(rewrite!(rname; "(LUT ?p ?a ?b ?c ?d ?e ?f)" 
        => {PermuteInput::new(i, "?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?d".parse().unwrap(), "?e".parse().unwrap(), "?f".parse().unwrap()])}));
    }

    rules
}

/// Condenses a k-Cut when it takes the form of a Shannon expansion
pub fn shannon_expansion() -> Vec<Rewrite<lut::LutLang, LutAnalysis>> {
    let mut rules: Vec<Rewrite<lut::LutLang, LutAnalysis>> = Vec::new();

    // Condense Shannon expansion
    rules.push(rewrite!("mux-make-disjoint-c1"; "(LUT 202 ?s true ?a)" => "(NOT (NOR ?s ?a))"));
    rules.push(rewrite!("mux-make-disjoint-c0"; "(LUT 202 ?s false ?a)" => "(AND (NOT ?s) ?a)"));
    rules.push(rewrite!("mux-make-disjoint-xor"; "(LUT 202 ?s (NOT ?a) ?a)" => "(XOR ?s ?a)"));
    rules.push(rewrite!("lut2-shannon"; "(LUT 202 ?s (LUT ?p ?a ?b) (LUT ?q ?a ?b))" => {ShannonCondense::new("?s".parse().unwrap(), "?p".parse().unwrap(), "?q".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap()])}));
    rules.push(rewrite!("lut3-shannon"; "(LUT 202 ?s (LUT ?p ?a ?b ?c) (LUT ?q ?a ?b ?c))" => {ShannonCondense::new("?s".parse().unwrap(), "?p".parse().unwrap(), "?q".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap()])}));
    rules.push(rewrite!("lut4-shannon"; "(LUT 202 ?s (LUT ?p ?a ?b ?c ?d) (LUT ?q ?a ?b ?c ?d))" => {ShannonCondense::new("?s".parse().unwrap(), "?p".parse().unwrap(), "?q".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?d".parse().unwrap()])}));
    rules.push(rewrite!("lut5-shannon"; "(LUT 202 ?s (LUT ?p ?a ?b ?c ?d ?e) (LUT ?q ?a ?b ?c ?d ?e))" => {ShannonCondense::new("?s".parse().unwrap(), "?p".parse().unwrap(), "?q".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?d".parse().unwrap(), "?e".parse().unwrap()])}));

    rules
}

fn p_q_cut_fuse(p: usize, q: usize) -> Rewrite<lut::LutLang, LutAnalysis> {
    assert!(p <= lut::LutLang::MAX_LUT_SIZE);
    assert!(q <= lut::LutLang::MAX_LUT_SIZE);
    let mut pi: Vec<String> = Vec::new();
    let mut qi: Vec<String> = Vec::new();
    for i in 0..p {
        pi.push(format!("?p{}", i));
    }
    for i in 0..q {
        qi.push(format!("?q{}", i));
    }
    let pattern: Pattern<lut::LutLang> =
        format!("(LUT ?pp {} (LUT ?qp {}))", pi.join(" "), qi.join(" "))
            .parse()
            .unwrap();
    let applier = FuseCut::new(
        "?pp".parse().unwrap(),
        pi.iter().map(|f| f.parse().unwrap()).collect(),
        "?qp".parse().unwrap(),
        qi.iter().map(|f| f.parse().unwrap()).collect(),
    );
    rewrite!(format!("lut{}-{}-fuse", p + 1, q); pattern => applier)
}

/// Generally condenses a k-Cut to a single LUT. This rule works even when inputs are not mutually-exclusive.
/// When k > 6, the rule does no rewriting (instead of crashing).
pub fn general_cut_fusion() -> Vec<Rewrite<lut::LutLang, LutAnalysis>> {
    let mut rules: Vec<Rewrite<lut::LutLang, LutAnalysis>> = Vec::new();
    // LUT fuse inputs (exclusive or not, sometimes the opposite of DSD)
    for p in 0..6 {
        for q in 1..6 {
            rules.push(p_q_cut_fuse(p, q));
        }
    }

    rules
}

/// Returns a list of rules for evaluating constant LUTs
pub fn constant_luts<A>() -> Vec<Rewrite<lut::LutLang, A>>
where
    A: Analysis<lut::LutLang>,
{
    let mut rules: Vec<Rewrite<lut::LutLang, A>> = Vec::new();
    // LUT fuse inputs (exclusive or not, sometimes the opposite of DSD)
    for k in 2..7 {
        let mask = if k < 6 { (1 << (1 << k)) - 1 } else { u64::MAX };
        let vars = (0..k).map(|i| format!("?v{}", i)).collect::<Vec<String>>();
        let pattern_true: Pattern<lut::LutLang> = format!("(LUT {} {})", mask, vars.join(" "))
            .parse()
            .unwrap();
        let pattern_false: Pattern<lut::LutLang> =
            format!("(LUT 0 {})", vars.join(" ")).parse().unwrap();
        let rname_true = format!("lut{}-const-true", k);
        let rname_false = format!("lut{}-const-false", k);
        rules.push(rewrite!(rname_true; pattern_true => "true"));
        rules.push(rewrite!(rname_false; pattern_false => "false"));
    }

    rules
}

/// Known decompositions of LUTs based on disjoint support decompositions
pub fn known_decompositions() -> Vec<Rewrite<lut::LutLang, LutAnalysis>> {
    let mut rules: Vec<Rewrite<lut::LutLang, LutAnalysis>> = Vec::new();
    // https://people.eecs.berkeley.edu/~alanmi/publications/2008/iccad08_lp.pdf
    // Boolean Factoring and Decomposition of Logic Networks
    rules.push(rewrite!("mux4-1-dsd"; "(LUT 18374951396690406058 ?s1 ?s0 ?a ?b ?c ?d)" => "(LUT 51952 ?s1 (LUT 61642 ?s1 ?s0 ?c ?d) ?a ?b)"));
    rules
}

/// Canonicalizes LUTs with redundant inputs
pub fn redundant_inputs() -> Vec<Rewrite<lut::LutLang, LutAnalysis>> {
    let mut rules: Vec<Rewrite<lut::LutLang, LutAnalysis>> = Vec::new();
    rules.push(rewrite!("lut3-redundant-mux"; "(LUT 202 ?s ?a ?a)" => "?a"));
    rules.push(rewrite!("lut2-redundant"; "(LUT ?p ?a ?a)" => {CombineAlikeInputs::new("?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?a".parse().unwrap()])}));
    rules.push(rewrite!("lut3-redundant"; "(LUT ?p ?a ?b ?b)" => {CombineAlikeInputs::new("?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?b".parse().unwrap()])}));
    rules.push(rewrite!("lut4-redundant"; "(LUT ?p ?a ?b ?c ?c)" => {CombineAlikeInputs::new("?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?c".parse().unwrap()])}));
    rules.push(rewrite!("lut5-redundant"; "(LUT ?p ?a ?b ?c ?d ?d)" => {CombineAlikeInputs::new("?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?d".parse().unwrap(), "?d".parse().unwrap()])}));
    rules.push(rewrite!("lut6-redundant"; "(LUT ?p ?a ?b ?c ?d ?e ?e)" => {CombineAlikeInputs::new("?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?d".parse().unwrap(), "?e".parse().unwrap(), "?e".parse().unwrap()])}));

    rules
}

/// Returns a list of all rules except for disjoint support decomposition
pub fn all_rules_minus_dsd() -> Vec<Rewrite<lut::LutLang, LutAnalysis>> {
    let mut rules: Vec<Rewrite<lut::LutLang, LutAnalysis>> = Vec::new();

    // Structural mappings of gates to LUTs
    rules.append(&mut struct_lut_map());

    // Evaluate constant programs
    rules.append(&mut constant_luts());

    // Evaluate constant inputs (impl as modify-analysis for multi-input cases)
    rules.push(rewrite!("lut1-const-false"; "(LUT 0 ?a)" => "false"));
    rules.push(rewrite!("lut1-const-true"; "(LUT 3 ?a)" => "true"));
    rules.push(rewrite!("lut1-const-id"; "(LUT 2 ?a)" => "?a"));
    rules.push(rewrite!("lut2-invariant"; "(LUT 12 ?a ?b)" => "(LUT 2 ?a)"));
    rules.push(rewrite!("lut1-const-true-inv"; "(LUT 1 false)" => "true"));
    rules.push(rewrite!("lut1-const-false-inv"; "(LUT 1 true)" => "false"));

    // Remove redundant inputs
    rules.append(&mut redundant_inputs());

    // TODO: DSD an input 6-LUT into two 4-LUTs
    // DSD with one shared variable: an k-LUT (k even) into two (N/2 + 1)-LUTS

    // LUT permutation groups
    rules.append(&mut permute_groups());

    // Condense Shannon expansion (a special case of fusing inputs)
    rules.append(&mut shannon_expansion());
    rules.append(&mut general_cut_fusion());

    // LUT fuse non-mutually exclusive inputs (hard, opposite of DSD)
    rules
}

fn zip_vars_with_ids(vars: &[Var], subst: &Subst) -> HashMap<egg::Id, Var> {
    vars.iter().map(|v| (subst[*v], *v)).collect()
}

/// Boilerplate code for unioning in custom Appliers while still generating meaningful proofs
fn union_with_lut_pattern<A>(
    old_ast: &PatternAst<lut::LutLang>,
    program: u64,
    new_lut: &lut::LutLang,
    vars: &[Var],
    subst: &egg::Subst,
    rule_name: egg::Symbol,
    egraph: &mut egg::EGraph<lut::LutLang, A>,
) -> Vec<egg::Id>
where
    A: Analysis<lut::LutLang> + std::default::Default,
{
    match new_lut {
        lut::LutLang::Lut(l) => {
            let id_to_var = zip_vars_with_ids(vars, subst);
            let c = l.clone();
            let var_list = c[1..]
                .iter()
                .map(|id| id_to_var[id].to_string())
                .collect::<Vec<String>>();
            let new_ast: PatternAst<lut::LutLang> =
                format!("(LUT {} {})", program, var_list.join(" "))
                    .parse()
                    .unwrap();
            let (id, b) = egraph.union_instantiations(old_ast, &new_ast, subst, rule_name);
            if b {
                vec![id]
            } else {
                vec![]
            }
        }
        _ => panic!("Expected LUT in union_with_lut_pattern"),
    }
}

/// A rewrite applier for permuting input `pos` with input `pos - 1` from the msb.
/// This means that a `pos` of 1 refers to the input second from the left when printed to a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermuteInput {
    /// Position of the input to permute
    pos: usize,
    program: Var,
    /// List of operands with msb first
    vars: Vec<Var>,
}

impl PermuteInput {
    /// Create a new [PermuteInput] applier given a transposition at `pos` in `operands`
    pub fn new(pos: usize, program: Var, vars: Vec<Var>) -> Self {
        Self { pos, program, vars }
    }
}

impl Applier<lut::LutLang, LutAnalysis> for PermuteInput {
    fn apply_one(
        &self,
        egraph: &mut egg::EGraph<lut::LutLang, LutAnalysis>,
        eclass: egg::Id,
        subst: &egg::Subst,
        searcher_ast: Option<&egg::PatternAst<lut::LutLang>>,
        rule_name: egg::Symbol,
    ) -> Vec<egg::Id> {
        let operands = self
            .vars
            .iter()
            .map(|v| subst[*v])
            .collect::<Vec<egg::Id>>();
        let program = egraph[subst[self.program]]
            .data
            .get_program()
            .expect("Expected program");

        assert!(self.pos > 0);

        if operands[self.pos] == operands[self.pos - 1] {
            return vec![];
        }

        let pos_from_lsb = (operands.len() - 1) - self.pos;
        let new_program = lut::swap_pos(&program, operands.len(), pos_from_lsb);
        let new_program_id = egraph.add(lut::LutLang::Program(new_program));

        assert!(self.pos < operands.len());

        let mut swaperands = operands.clone();
        swaperands.swap(self.pos, self.pos - 1);

        let mut c = Vec::from(&[new_program_id]);
        c.append(&mut swaperands);

        let new_node = lut::LutLang::Lut(c.into());

        match searcher_ast {
            Some(ast) => union_with_lut_pattern(
                ast,
                new_program,
                &new_node,
                &self.vars,
                subst,
                rule_name,
                egraph,
            ),
            None => {
                let new_lut = egraph.add(new_node);
                if egraph.union_trusted(eclass, new_lut, rule_name) {
                    vec![new_lut]
                } else {
                    vec![]
                }
            }
        }
    }
}

/// A rewrite applier for combining two inputs that are the same.
/// The redundant inputs *must* be in the rightmost position in the LUT when printed to a string.
/// This means the last two elements in `vars` must be the same
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombineAlikeInputs {
    /// The program
    program: Var,
    /// The redundant inputs must be at the last two positions
    vars: Vec<Var>,
}

impl CombineAlikeInputs {
    /// Create an applier that combines duplicated inputs to a LUT.
    /// The last two elements in `vars` must be the same.
    pub fn new(program: Var, vars: Vec<Var>) -> Self {
        Self { program, vars }
    }
}

impl Applier<lut::LutLang, LutAnalysis> for CombineAlikeInputs {
    fn apply_one(
        &self,
        egraph: &mut egg::EGraph<lut::LutLang, LutAnalysis>,
        eclass: egg::Id,
        subst: &egg::Subst,
        searcher_ast: Option<&egg::PatternAst<lut::LutLang>>,
        rule_name: egg::Symbol,
    ) -> Vec<egg::Id> {
        let mut operands = self
            .vars
            .iter()
            .map(|v| subst[*v])
            .collect::<Vec<egg::Id>>();
        let olen = operands.len();
        assert!(operands[olen - 1] == operands[olen - 2]);
        let program = egraph[subst[self.program]]
            .data
            .get_program()
            .expect("Expected program");
        let k = operands.len();
        // Handle the mux case as a special case
        if k == 3 && program == 202 {
            return vec![];
        }
        let mut new_prog = bitvec!(usize, Lsb0; 0; 1 << (k-1));
        for i in 0..(1 << (k - 2)) {
            let eval_e = lut::eval_lut_bv(program, &lut::to_bitvec(i << 2, 1 << k).unwrap());
            new_prog.set((i << 1) as usize, eval_e);
            let eval_o = lut::eval_lut_bv(program, &lut::to_bitvec((i << 2) + 3, 1 << k).unwrap());
            new_prog.set((i << 1) as usize + 1, eval_o);
        }
        let new_prog = lut::from_bitvec(&new_prog);
        let new_prog_id = egraph.add(lut::LutLang::Program(new_prog));
        let mut c = Vec::from(&[new_prog_id]);
        operands.pop();
        c.append(&mut operands);
        let new_node = lut::LutLang::Lut(c.into());

        match searcher_ast {
            Some(ast) => union_with_lut_pattern(
                ast, new_prog, &new_node, &self.vars, subst, rule_name, egraph,
            ),
            None => {
                let new_lut = egraph.add(new_node);
                if egraph.union_trusted(eclass, new_lut, rule_name) {
                    vec![new_lut]
                } else {
                    vec![]
                }
            }
        }
    }
}

/// A rewrite applier for condensing Shannon expansions into a single LUT.
/// The matched eclass *must* be a 2:1 mux (i.e. `LUT 202`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShannonCondense {
    /// The sel input
    sel: Var,
    /// The program to use when sel=0
    p: Var,
    /// The program to use when sel=1
    q: Var,
    /// The inputs (must not have duplicates)
    vars: Vec<Var>,
}

impl ShannonCondense {
    /// Create a new applier for condensing Shannon expansions. This is a special case of [FuseCut].
    /// The matched node should take the form `(LUT 202 ?s (LUT ?p ?a ?b ... ) (LUT ?q ?a ?b ...))`
    pub fn new(sel: Var, p: Var, q: Var, vars: Vec<Var>) -> Self {
        Self { sel, p, q, vars }
    }
}

impl Applier<lut::LutLang, LutAnalysis> for ShannonCondense {
    fn apply_one(
        &self,
        egraph: &mut egg::EGraph<lut::LutLang, LutAnalysis>,
        eclass: egg::Id,
        subst: &egg::Subst,
        searcher_ast: Option<&egg::PatternAst<lut::LutLang>>,
        rule_name: egg::Symbol,
    ) -> Vec<egg::Id> {
        let mut operands = self
            .vars
            .iter()
            .map(|v| subst[*v])
            .collect::<Vec<egg::Id>>();
        let p = egraph[subst[self.p]]
            .data
            .get_program()
            .expect("Expected program");
        let q = egraph[subst[self.q]]
            .data
            .get_program()
            .expect("Expected program");
        if p == q {
            return vec![];
        }
        let k = operands.len();
        assert!(k <= 5);
        let new_prog = p << (1 << k) | q;
        let new_prog_id = egraph.add(lut::LutLang::Program(new_prog));
        let sel = subst[self.sel];
        let mut c = Vec::from(&[new_prog_id, sel]);
        c.append(&mut operands);
        assert!(c.len() == k + 2);
        let new_node = lut::LutLang::Lut(c.into());

        match searcher_ast {
            Some(ast) => {
                let mut vars = self.vars.clone();
                vars.push(self.sel);
                union_with_lut_pattern(ast, new_prog, &new_node, &vars, subst, rule_name, egraph)
            }
            None => {
                let new_lut = egraph.add(new_node);
                if egraph.union_trusted(eclass, new_lut, rule_name) {
                    vec![new_lut]
                } else {
                    vec![]
                }
            }
        }
    }
}

/// A pattern for compiling a k-sized cut of logic elements into a single LUT
/// This applier works even when inputs are not mutually-exclusive.
/// If the inputs are mutually exclusive and form a cut larger than 6, the applier returns nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuseCut {
    /// The root program
    root_p: Var,
    /// Direct inputs to the root
    root: Vec<Var>,
    /// Child program
    rhs_p: Var,
    /// Child inputs
    rhs: Vec<Var>,
}

impl FuseCut {
    /// Create a new applier for fusing two cut of logic elements into a single LUT.
    /// The union of `root` and `rhs` must be no larger than 6 total nodes.
    /// The matched node should take the form `(LUT ?root_p ?root ... (LUT ?rhs_p ?rhs ...))`
    pub fn new(root_p: Var, root: Vec<Var>, rhs_p: Var, rhs: Vec<Var>) -> Self {
        Self {
            root_p,
            root,
            rhs_p,
            rhs,
        }
    }

    /// Given the state of the cut set to the state `bv`, return the state of the inputs found in `inputs`.
    /// `pos_map` contains the offsets of the inputs in larger cut contained in `bv`. The offsets are relative to the msb.
    /// Finally, remember that `bv` is lsb first, whereas [egg::Id] arrays are msb first.
    fn get_input_vec(bv: &BitVec, pos_map: &HashMap<egg::Id, usize>, inputs: &[egg::Id]) -> BitVec {
        assert!(inputs.len() <= lut::LutLang::MAX_LUT_SIZE);
        assert!(inputs.len() <= bv.len());
        let mut new_bv = bitvec!(usize, Lsb0; 0; inputs.len());
        for (i, input) in inputs.iter().enumerate() {
            let pos = pos_map[input];
            new_bv.set(inputs.len() - 1 - i, *bv.get(bv.len() - 1 - pos).unwrap());
        }
        new_bv
    }

    /// Returns true if there are repeats in the operands
    fn has_repeats(operands: &[egg::Id]) -> bool {
        let vset: HashSet<egg::Id> = HashSet::from_iter(operands.iter().cloned());
        vset.len() < operands.len()
    }

    /// Returns of a map corresponding to the sorting of the [egg:Id] in `vset`
    fn get_sorted_map(vset: &HashSet<egg::Id>) -> HashMap<egg::Id, usize> {
        let mut s = Vec::from_iter(vset.iter().cloned());
        s.sort();
        let mut pos_map: HashMap<egg::Id, usize> = HashMap::default();
        for (i, v) in s.iter().enumerate() {
            pos_map.insert(*v, i);
        }
        pos_map
    }
}

impl Applier<lut::LutLang, LutAnalysis> for FuseCut {
    fn apply_one(
        &self,
        egraph: &mut egg::EGraph<lut::LutLang, LutAnalysis>,
        eclass: egg::Id,
        subst: &egg::Subst,
        searcher_ast: Option<&egg::PatternAst<lut::LutLang>>,
        rule_name: egg::Symbol,
    ) -> Vec<egg::Id> {
        let root_operands = self
            .root
            .iter()
            .map(|v| subst[*v])
            .collect::<Vec<egg::Id>>();
        let rhs_operands = self.rhs.iter().map(|v| subst[*v]).collect::<Vec<egg::Id>>();
        if FuseCut::has_repeats(&root_operands) || FuseCut::has_repeats(&rhs_operands) {
            return vec![];
        }
        let root_program = egraph[subst[self.root_p]]
            .data
            .get_program()
            .expect("Expected program");
        let rhs_program = egraph[subst[self.rhs_p]]
            .data
            .get_program()
            .expect("Expected program");

        let mut vset: HashSet<egg::Id> = HashSet::new();
        for v in root_operands.iter().chain(rhs_operands.iter()) {
            vset.insert(*v);
        }
        let nk = vset.len();
        // Let this be a soft error, because we might not know at match time that we don't have a feasible cut
        if nk > 6 {
            return vec![];
        }
        let pos_map = FuseCut::get_sorted_map(&vset);
        let mut new_prog = bitvec!(usize, Lsb0; 0; 1 << nk);
        // sweep inputs
        for i in 0..(1 << nk) {
            let bv = to_bitvec(i, nk).unwrap();
            let rhs_bv = FuseCut::get_input_vec(&bv, &pos_map, &rhs_operands);
            let rhs_eval = lut::eval_lut_bv(rhs_program, &rhs_bv);
            let mut root_bv = bitvec!(usize, Lsb0; 0; root_operands.len() + 1);
            root_bv.set(0, rhs_eval);
            let rbvl = root_bv.len();
            for (j, root_op) in root_operands.iter().enumerate() {
                let pos = pos_map[root_op];
                root_bv.set(rbvl - 1 - j, *bv.get(bv.len() - 1 - pos).unwrap());
            }
            new_prog.set(i as usize, lut::eval_lut_bv(root_program, &root_bv));
        }
        let new_prog = lut::from_bitvec(&new_prog);
        let mut c = vec![egraph.add(lut::LutLang::Program(new_prog)); nk + 1];
        for (&k, &v) in pos_map.iter() {
            c[v + 1] = k;
        }

        let new_node = lut::LutLang::Lut(c.clone().into());

        match searcher_ast {
            Some(ast) => {
                let all_vars: Vec<Var> = self
                    .root
                    .iter()
                    .cloned()
                    .chain(self.rhs.iter().cloned())
                    .collect();
                union_with_lut_pattern(
                    ast, new_prog, &new_node, &all_vars, subst, rule_name, egraph,
                )
            }
            None => {
                let new_lut = egraph.add(new_node);
                if egraph.union_trusted(eclass, new_lut, rule_name) {
                    vec![new_lut]
                } else {
                    vec![]
                }
            }
        }
    }
}
