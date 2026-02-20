/*!

  Boolean rewriting techniques for LUT re-synthesis.

*/
use super::analysis::LutAnalysis;
use super::lut;
use super::lut::to_bitvec;
use bitvec::{bitvec, order::Lsb0, vec::BitVec};
use egg::{
    Analysis, Applier, FromOp, Language, Pattern, PatternAst, Rewrite, Subst, Symbol, Var, rewrite,
};
use std::fmt::Display;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::{
    collections::{HashMap, HashSet},
    io::{BufRead, BufReader, Read},
};

/// Returns a list of structural mappings of logic functions to LUTs.
/// For example, MUXes are mapped to 3-LUTs and AND gates to 2-LUTs.
pub fn struct_lut_map<A>(bidirectional: bool) -> Vec<Rewrite<lut::LutLang, A>>
where
    A: Analysis<lut::LutLang> + std::default::Default,
{
    let mut rules: Vec<Rewrite<lut::LutLang, A>> = Vec::new();
    // Logic element conversions
    if bidirectional {
        rules.append(&mut rewrite!("nor2-conversion"; "(NOR ?a ?b)" <=> "(LUT 1 ?a ?b)"));
        rules.append(&mut rewrite!("and2-conversion"; "(AND ?a ?b)" <=> "(LUT 8 ?a ?b)"));
        rules.append(&mut rewrite!("xor2-conversion"; "(XOR ?a ?b)" <=> "(LUT 6 ?a ?b)"));
        rules.push(rewrite!("or2-conversion"; "(LUT 14 ?a ?b)" => "(NOT (NOR ?a ?b))"));
        rules.push(rewrite!("and-one-inv-conversion"; "(LUT 2 ?a ?b)" => "(AND (NOT ?a) ?b)"));
        rules.append(&mut rewrite!("xor2-nor-nand"; "(XOR ?a ?b)" <=> "(NOT (NOR (AND (NOT ?a) ?b) (AND (NOT ?b) ?a)))"));
        rules.append(
            &mut rewrite!("and-distributivity"; "(AND ?a (NOT (NOR ?b ?c)))" <=> "(NOT (NOR (AND ?a ?b) (AND ?a ?c)))"),
        );
        rules.append(
            &mut rewrite!("or-distributivity"; "(NOT (NOR ?a (AND ?b ?c)))" <=> "(AND (NOT (NOR ?a ?b)) (NOT (NOR ?a ?c)))"),
        );
        rules.append(
            &mut rewrite!("and-associativity"; "(AND (AND ?a ?b) ?c)" <=> "(AND ?a (AND ?b ?c))"),
        );
        rules.append(
            &mut rewrite!("or-associativity"; "(NOT (NOR ?a (NOT (NOR ?b ?c))))" <=> "(NOT (NOR (NOT (NOR ?a ?b)) ?c))"),
        );
        rules.append(&mut rewrite!("demorgan"; "(NOR ?a ?b)" <=> "(AND (NOT ?a) (NOT ?b))"));
    } else {
        rules.push(rewrite!("nor2-conversion"; "(NOR ?a ?b)" => "(LUT 1 ?a ?b)"));
        rules.push(rewrite!("and2-conversion"; "(AND ?a ?b)" => "(LUT 8 ?a ?b)"));
        rules.push(rewrite!("xor2-conversion"; "(XOR ?a ?b)" => "(LUT 6 ?a ?b)"));
    }

    rules.append(&mut rewrite!("inverter-conversion"; "(NOT ?a)" <=> "(LUT 1 ?a)"));
    // s? a : b
    rules.append(&mut rewrite!("mux2-1-conversion"; "(MUX ?s ?a ?b)" <=> "(LUT 202 ?s ?a ?b)"));

    rules
}

/// Move registers around LUTs
pub fn register_retiming<A>() -> Vec<Rewrite<lut::LutLang, A>>
where
    A: Analysis<lut::LutLang> + std::default::Default,
{
    let mut rules: Vec<Rewrite<lut::LutLang, A>> = Vec::new();
    // Logic element conversions
    rules.append(&mut rewrite!("lut1-retime"; "(LUT ?p (REG ?a ?clk ?ce ?rst))" <=> "(REG (LUT ?p ?a) ?clk ?ce ?rst)"));
    rules.append(
        &mut rewrite!("lut2-retime"; "(LUT ?p (REG ?a ?clk ?ce ?rst) (REG ?b ?clk ?ce ?rst))" <=> "(REG (LUT ?p ?a ?b) ?clk ?ce ?rst)"),
    );
    rules.append(
        &mut rewrite!("lut3-retime"; "(LUT ?p (REG ?a ?clk ?ce ?rst) (REG ?b ?clk ?ce ?rst) (REG ?c ?clk ?ce ?rst))" <=> "(REG (LUT ?p ?a ?b ?c) ?clk ?ce ?rst)"),
    );
    rules.append(
        &mut rewrite!("lut4-retime"; "(LUT ?p (REG ?a ?clk ?ce ?rst) (REG ?b ?clk ?ce ?rst) (REG ?c ?clk ?ce ?rst) (REG ?d ?clk ?ce ?rst))" <=> "(REG (LUT ?p ?a ?b ?c ?d) ?clk ?ce ?rst)"),
    );
    rules.append(
        &mut rewrite!("lut5-retime"; "(LUT ?p (REG ?a ?clk ?ce ?rst) (REG ?b ?clk ?ce ?rst) (REG ?c ?clk ?ce ?rst) (REG ?d ?clk ?ce ?rst) (REG ?e ?clk ?ce ?rst))" <=> "(REG (LUT ?p ?a ?b ?c ?d ?e) ?clk ?ce ?rst)"),
    );
    rules.append(
        &mut rewrite!("lut6-retime"; "(LUT ?p (REG ?a ?clk ?ce ?rst) (REG ?b ?clk ?ce ?rst) (REG ?c ?clk ?ce ?rst) (REG ?d ?clk ?ce ?rst) (REG ?e ?clk ?ce ?rst) (REG ?f ?clk ?ce ?rst))" <=> "(REG (LUT ?p ?a ?b ?c ?d ?e ?f) ?clk ?ce ?rst)"),
    );

    rules
}

/// Returns a list of rules for permuting inputs in LUTs. Each instance of these rules forms a group under composition (<https://en.wikipedia.org/wiki/Symmetric_group>).
/// Each of these groups have k-1 generators.
pub fn permute_groups() -> Vec<Rewrite<lut::LutLang, LutAnalysis>> {
    let mut rules: Vec<Rewrite<lut::LutLang, LutAnalysis>> = Vec::new();
    // LUT permutation groups
    rules.push(rewrite!("lut2-permute"; "(LUT ?p ?a ?b)" 
        => {PermuteInput::new(1, "?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap()])}));

    for i in 1..3 {
        let rname = format!("lut3-permute-{i}");
        rules.push(rewrite!(rname; "(LUT ?p ?a ?b ?c)" 
        => {PermuteInput::new(i, "?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap()])}));
    }

    for i in 1..4 {
        let rname = format!("lut4-permute-{i}");
        rules.push(rewrite!(rname; "(LUT ?p ?a ?b ?c ?d)" 
        => {PermuteInput::new(i, "?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?d".parse().unwrap()])}));
    }

    for i in 1..5 {
        let rname = format!("lut5-permute-{i}");
        rules.push(rewrite!(rname; "(LUT ?p ?a ?b ?c ?d ?e)" 
        => {PermuteInput::new(i, "?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?d".parse().unwrap(), "?e".parse().unwrap()])}));
    }

    for i in 1..6 {
        let rname = format!("lut6-permute-{i}");
        rules.push(rewrite!(rname; "(LUT ?p ?a ?b ?c ?d ?e ?f)" 
        => {PermuteInput::new(i, "?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?d".parse().unwrap(), "?e".parse().unwrap(), "?f".parse().unwrap()])}));
    }

    rules
}

/// Condenses two cofactors along a single boolean term into one combined function
pub fn condense_cofactors() -> Vec<Rewrite<lut::LutLang, LutAnalysis>> {
    let mut rules: Vec<Rewrite<lut::LutLang, LutAnalysis>> = Vec::new();

    // Condense Shannon expansion
    // TODO(matth2k): Rewrite these in terms of LUTs
    rules.push(rewrite!("mux-make-disjoint-or"; "(LUT 202 ?s true ?a)" => "(LUT 14 ?s ?a)"));
    rules.push(rewrite!("mux-make-disjoint-or-not"; "(LUT 202 ?s ?a true)" => "(LUT 14 (LUT 8 ?s ?a) (LUT 1 ?s))"));
    rules.push(rewrite!("mux-make-disjoint-and"; "(LUT 202 ?s ?a false)" => "(LUT 8 ?s ?a)"));
    rules.push(rewrite!("mux-make-disjoint-and-not"; "(LUT 202 ?s false ?a)" => "(LUT 2 ?s ?a)"));
    rules.push(rewrite!("mux-make-disjoint-xor"; "(LUT 202 ?s (NOT ?a) ?a)" => "(LUT 6 ?s ?a)"));
    rules.push(rewrite!("mux-make-disjoint-xnor"; "(LUT 202 ?s ?a (NOT ?a))" => "(LUT 9 ?s ?a)"));
    rules.push(rewrite!("lut2-shannon-condense"; "(LUT 202 ?s (LUT ?p ?a ?b) (LUT ?q ?a ?b))" => {ShannonCondense::new("?s".parse().unwrap(), "?p".parse().unwrap(), "?q".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap()])}));
    rules.push(rewrite!("lut3-shannon-condense"; "(LUT 202 ?s (LUT ?p ?a ?b ?c) (LUT ?q ?a ?b ?c))" => {ShannonCondense::new("?s".parse().unwrap(), "?p".parse().unwrap(), "?q".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap()])}));
    rules.push(rewrite!("lut4-shannon-condense"; "(LUT 202 ?s (LUT ?p ?a ?b ?c ?d) (LUT ?q ?a ?b ?c ?d))" => {ShannonCondense::new("?s".parse().unwrap(), "?p".parse().unwrap(), "?q".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?d".parse().unwrap()])}));
    rules.push(rewrite!("lut5-shannon-condense"; "(LUT 202 ?s (LUT ?p ?a ?b ?c ?d ?e) (LUT ?q ?a ?b ?c ?d ?e))" => {ShannonCondense::new("?s".parse().unwrap(), "?p".parse().unwrap(), "?q".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?d".parse().unwrap(), "?e".parse().unwrap()])}));

    rules
}

fn p_q_cut_fuse(p: usize, q: usize) -> Rewrite<lut::LutLang, LutAnalysis> {
    assert!(p <= lut::LutLang::MAX_LUT_SIZE);
    assert!(q <= lut::LutLang::MAX_LUT_SIZE);
    let mut pi: Vec<String> = Vec::new();
    let mut qi: Vec<String> = Vec::new();
    for i in 0..p {
        pi.push(format!("?p{i}"));
    }
    for i in 0..q {
        qi.push(format!("?q{i}"));
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
        let vars = (0..k).map(|i| format!("?v{i}")).collect::<Vec<String>>();
        let pattern_true: Pattern<lut::LutLang> = format!("(LUT {} {})", mask, vars.join(" "))
            .parse()
            .unwrap();
        let pattern_false: Pattern<lut::LutLang> =
            format!("(LUT 0 {})", vars.join(" ")).parse().unwrap();
        let rname_true = format!("lut{k}-const-true");
        let rname_false = format!("lut{k}-const-false");
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

/// Find dynamic decompositions of LUTs at runtime.
/// Finds compositions in any variable order when `any_order` is true
#[cfg(feature = "dyn_decomp")]
pub fn dyn_decompositions(any_order: bool) -> Vec<Rewrite<lut::LutLang, LutAnalysis>> {
    let mut rules: Vec<Rewrite<lut::LutLang, LutAnalysis>> = Vec::new();
    rules.push(
        rewrite!("mux-expand"; "(LUT 202 ?s ?a ?b)" => "(LUT 14 (LUT 8 ?s ?a) (LUT 2 ?s ?b))"),
    );
    rules.push(rewrite!("lut3-shannon-expand"; "(LUT ?p ?a ?b ?c)" => {decomp::ShannonExpand::new("?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap()], any_order)}));
    rules.push(rewrite!("lut4-shannon-expand"; "(LUT ?p ?a ?b ?c ?d)" => {decomp::ShannonExpand::new("?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?d".parse().unwrap()], any_order)}));
    rules.push(rewrite!("lut5-shannon-expand"; "(LUT ?p ?a ?b ?c ?d ?e)" => {decomp::ShannonExpand::new("?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?d".parse().unwrap(), "?e".parse().unwrap()], any_order)}));
    rules.push(rewrite!("lut6-shannon-expand"; "(LUT ?p ?a ?b ?c ?d ?e ?f)" => {decomp::ShannonExpand::new("?p".parse().unwrap(), vec!["?a".parse().unwrap(), "?b".parse().unwrap(), "?c".parse().unwrap(), "?d".parse().unwrap(), "?e".parse().unwrap(), "?f".parse().unwrap()], any_order)}));
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

/// Returns a list of all static LUT rewrite rules
/// `bidirectional` determines if gates are inserted for 2-LUTs
pub fn all_static_rules(bidirectional: bool) -> Vec<Rewrite<lut::LutLang, LutAnalysis>> {
    let mut rules: Vec<Rewrite<lut::LutLang, LutAnalysis>> = Vec::new();

    // Structural mappings of gates to LUTs
    rules.append(&mut struct_lut_map(bidirectional));

    // Evaluate constant programs
    rules.append(&mut constant_luts());

    // Evaluate constant inputs (impl as modify-analysis for multi-input cases)
    rules.push(rewrite!("lut1-const-false"; "(LUT 0 ?a)" => "false"));
    rules.push(rewrite!("lut1-const-true"; "(LUT 3 ?a)" => "true"));
    rules.push(rewrite!("lut1-const-id"; "(LUT 2 ?a)" => "?a"));
    rules.push(rewrite!("lut2-invariant"; "(LUT 12 ?a ?b)" => "(LUT 2 ?a)"));
    rules.push(rewrite!("lut1-const-true-inv"; "(LUT 1 false)" => "true"));
    rules.push(rewrite!("lut1-const-false-inv"; "(LUT 1 true)" => "false"));
    rules.push(rewrite!("double-complement"; "(NOT (NOT ?a))" => "?a"));

    // Remove redundant inputs
    rules.append(&mut redundant_inputs());

    // TODO: DSD an input 6-LUT into two 4-LUTs
    // DSD with one shared variable: an k-LUT (k even) into two (N/2 + 1)-LUTS

    // LUT permutation groups
    rules.append(&mut permute_groups());

    // Condense cofactors and general cuts
    rules.append(&mut condense_cofactors());
    rules.append(&mut general_cut_fusion());

    // Compile-time decompositions
    rules.append(&mut known_decompositions());

    // LUT fuse non-mutually exclusive inputs (hard, opposite of DSD)
    rules
}

/// Returns a list of all lutpacking rules except for run-time calculated decompositions
pub fn lutpacking_rules() -> Vec<Rewrite<lut::LutLang, LutAnalysis>> {
    all_static_rules(false)
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
            if b { vec![id] } else { vec![] }
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
        let new_prog = (p << (1 << k)) | q;
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

/// A module dedicated to dynamically finding decompositions of LUTs
#[cfg(feature = "dyn_decomp")]
pub mod decomp {

    use crate::{
        analysis::{self, LutAnalysis},
        lut::{self, LutLang, from_bitvec, to_bitvec},
    };
    use bitvec::prelude::*;
    use egg::{Analysis, Applier, Id, Var};

    #[derive(Debug, Clone, PartialEq, Eq)]
    /// A data type for folding LUTs
    enum AbstractNode {
        /// A LUT node with [u64] configuration
        Lut(u64, Vec<Id>),
        /// A constant true/false node
        Const(bool),
        /// An indirect node
        Node(Id),
    }

    impl AbstractNode {
        /// Returns the number of children of the node
        pub fn num_inputs(&self) -> usize {
            match self {
                AbstractNode::Lut(_, inputs) => inputs.len(),
                AbstractNode::Node(_) => 1,
                _ => 0,
            }
        }

        /// Put this [AbstractNode] into the `egraph`
        pub fn construct<A>(self, egraph: &mut egg::EGraph<LutLang, A>) -> Id
        where
            A: Analysis<LutLang>,
        {
            match self {
                AbstractNode::Lut(program, mut inputs) => {
                    let pid = egraph.add(LutLang::Program(program));
                    let mut c = vec![pid];
                    c.append(&mut inputs);
                    egraph.add(LutLang::Lut(c.into()))
                }
                AbstractNode::Const(b) => egraph.add(LutLang::Const(b)),
                AbstractNode::Node(id) => id,
            }
        }

        /// Given a program `p` and a set of inputs `inputs`, this function returns a simplification of the LUT
        fn fold_lut(self) -> Self {
            if let Self::Lut(program, inputs) = self {
                let k = inputs.len();

                if k <= 1 {
                    match program & 3 {
                        0 => return Self::Const(false),
                        3 => return Self::Const(true),
                        2 => return Self::Node(inputs[0]),
                        1 => return Self::Lut(1, inputs),
                        _ => unreachable!(),
                    }
                }

                // Evaluate invariant inputs
                for pos in 0..k {
                    let pbv = to_bitvec(program, 1 << k).unwrap();
                    let mut nbv: BitVec<usize, Lsb0> = BitVec::with_capacity(1 << (k - 1));
                    for i in 0..(1 << (k - 1)) {
                        let mut index_lo = to_bitvec(i, k - 1).unwrap();
                        let mut index_hi = index_lo.clone();
                        index_lo.insert(k - pos - 1, false);
                        index_hi.insert(k - pos - 1, true);
                        let index_lo = from_bitvec(&index_lo) as usize;
                        let index_hi = from_bitvec(&index_hi) as usize;
                        if pbv[index_lo] != pbv[index_hi] {
                            break;
                        }
                        nbv.push(pbv[index_lo]);
                    }
                    if nbv.len() == 1 << (k - 1) {
                        let np = from_bitvec(&nbv);
                        let mut c = inputs;
                        c.remove(pos);
                        return Self::Lut(np, c);
                    } else {
                        continue;
                    }
                }
                Self::Lut(program, inputs)
            } else {
                self
            }
        }
    }

    /// Given a program `p` and a set of inputs `inputs`, this function returns a simplification of the LUT
    fn fold_lut_greedily(program: u64, inputs: Vec<Id>) -> AbstractNode {
        let init = AbstractNode::Lut(program, inputs);
        let mut current = init;
        loop {
            let next = current.clone().fold_lut();
            if next == current {
                break;
            }
            current = next;
        }
        current
    }
    /// A rewrite applier for expanding a LUT along its two cofactors in the most-significant operand.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ShannonExpand {
        /// The program
        program: Var,
        /// The redundant inputs must be at the last two positions
        vars: Vec<Var>,
        /// Can decompose in any variable order (expensive)
        any_order: bool,
    }

    impl ShannonExpand {
        /// Create an applier that combines duplicated inputs to a LUT.
        /// The last two elements in `vars` must be the same.
        pub fn new(program: Var, vars: Vec<Var>, any_order: bool) -> Self {
            Self {
                program,
                vars,
                any_order,
            }
        }

        fn cuts_overlap(
            egraph: &mut egg::EGraph<lut::LutLang, analysis::LutAnalysis>,
            children: &[egg::Id],
        ) -> bool {
            for (i, a) in children.iter().enumerate() {
                let ac = egraph[*a].data.get_cut();
                // Don't want inverters to be elaborated on
                if ac.len() == 1 && !egraph[*a].data.is_an_input() {
                    return true;
                }
                for b in children.iter().skip(i + 1) {
                    if a == b {
                        return true;
                    }
                    let bc = egraph[*b].data.get_cut();
                    if ac.intersection(bc).count() > 0 {
                        return true;
                    }
                }
            }
            false
        }
    }

    impl Applier<LutLang, LutAnalysis> for ShannonExpand {
        fn apply_one(
            &self,
            egraph: &mut egg::EGraph<LutLang, LutAnalysis>,
            eclass: egg::Id,
            subst: &egg::Subst,
            searcher_ast: Option<&egg::PatternAst<LutLang>>,
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
            let k = operands.len();
            // Only going to decompose in three variables or more
            if k <= 2 || program == 0 || program.count_ones() == (1 << k) {
                return vec![];
            }
            if k == 3 && program == 202 {
                return vec![];
            }
            // Can only decompose in one variable order or else the e-graph will explode
            if self.any_order {
                if !operands[1..].windows(2).all(|w| w[0] <= w[1]) {
                    return vec![];
                }
            } else if !operands.windows(2).all(|w| w[0] <= w[1]) {
                return vec![];
            }

            // No overlapping cuts of children
            if operands.contains(&eclass) {
                return vec![];
            }
            if Self::cuts_overlap(egraph, &operands) {
                return vec![];
            }

            let (c1, c0) = lut::cofactors_in_msb(&program, k);
            let c1 = fold_lut_greedily(c1, operands[1..].to_vec());
            let c0 = fold_lut_greedily(c0, operands[1..].to_vec());

            // Don't want to decompose when one of the cofactors is constant
            // TODO: Pre-fold this into a different form
            if c0.num_inputs() == 0 || c1.num_inputs() == 0 {
                return vec![];
            }

            if searcher_ast.is_some() {
                todo!("Implement pattern update for ShannonExpand");
            }

            let c1_id = c1.construct(egraph);
            let c0_id = c0.construct(egraph);
            let mux_p = egraph.add(lut::LutLang::Program(202));
            let new_node = lut::LutLang::Lut(vec![mux_p, operands[0], c1_id, c0_id].into());
            let new_lut = egraph.add(new_node);
            if egraph.union_trusted(eclass, new_lut, rule_name) {
                vec![new_lut]
            } else {
                vec![]
            }
        }
    }

    #[test]
    fn test_decomp() {
        let expr: egg::RecExpr<lut::LutLang> = "(LUT 61642 s1 s0 c d)".parse().unwrap();
        let mut rules = super::lutpacking_rules();
        rules.append(&mut super::dyn_decompositions(false));

        use crate::driver::{SynthReport, SynthRequest};
        let mut req = SynthRequest::default()
            .with_expr(expr)
            .with_rules(rules)
            .with_k(3)
            .with_asserts()
            .without_progress_bar()
            .with_joint_limits(20, 20_000, 30);

        let ans = req.synth::<SynthReport>().unwrap().get_expr().to_string();
        assert_eq!(ans, "(LUT 202 s1 s0 (LUT 202 s0 c d))");
    }
}

/// Load and manage groups of rewrite rules
#[derive(Clone)]
pub struct RewriteManager<L, A>
where
    L: Language + FromOp,
    A: Analysis<L> + Clone,
{
    db: HashMap<String, Rewrite<L, A>>,
    active: HashMap<String, Rewrite<L, A>>,
    categories: HashMap<String, Vec<String>>,
}

impl<L, A> Default for RewriteManager<L, A>
where
    L: Language + FromOp,
    A: Analysis<L> + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<L, A> Hash for RewriteManager<L, A>
where
    L: Language + FromOp + Display + 'static,
    A: Analysis<L> + Clone + 'static,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let strings: Vec<String> = self.active.values().map(|r| format!("{r:?}")).collect();
        strings.hash(state);
    }
}

impl<L, A> RewriteManager<L, A>
where
    L: Language + FromOp,
    A: Analysis<L> + Clone,
{
    /// Create a new empty [RewriteManager]
    pub fn new() -> Self {
        Self {
            db: HashMap::new(),
            active: HashMap::new(),
            categories: HashMap::new(),
        }
    }

    /// Insert a category of rewrites from an iterator.
    /// Returns an error if any rewrite name already exists in the database (by name)
    pub fn insert_category<I>(&mut self, category: String, rewrites: I) -> Result<(), Rewrite<L, A>>
    where
        I: IntoIterator<Item = Rewrite<L, A>>,
    {
        let category = self.categories.entry(category).or_default();
        for rw in rewrites {
            let name = rw.name.to_string();
            if self.db.contains_key(&name) {
                return Err(rw);
            }
            self.db.insert(name.clone(), rw);
            category.push(name);
        }
        Ok(())
    }

    /// Inserts an individual rewrite rule into the category.
    /// Returns an error if the rewrite name already exists in the database (by name)
    pub fn insert_into_category(
        &mut self,
        category: String,
        rewrite: Rewrite<L, A>,
    ) -> Result<(), Rewrite<L, A>> {
        let name = rewrite.name.to_string();
        if self.db.contains_key(&name) {
            return Err(rewrite);
        }
        self.db.insert(name.clone(), rewrite);
        let category = self.categories.entry(category).or_default();
        category.push(name);
        Ok(())
    }

    /// Insert a rule on its own.
    /// Returns an error if the rewrite name already exists in the database (by name)
    pub fn insert_rule(&mut self, rewrite: Rewrite<L, A>) -> Result<(), Rewrite<L, A>> {
        let name = rewrite.name.to_string();
        if self.db.contains_key(&name) {
            return Err(rewrite);
        }
        self.db.insert(name, rewrite);
        Ok(())
    }

    /// Enables an entire category of rewrites.
    /// Returns the number of rewrite rules added to the active set.
    pub fn enable_category(&mut self, category: &str) -> Option<usize> {
        if let Some(rw_names) = self.categories.get(category) {
            let mut count = 0;
            for name in rw_names {
                if self
                    .active
                    .insert(name.clone(), self.db[name].clone())
                    .is_none()
                {
                    count += 1;
                }
            }
            Some(count)
        } else {
            None
        }
    }

    /// Enables an individual rewrite rule by name.
    /// Returns true if the rule was added to the active set.
    pub fn enable_rule(&mut self, name: &str) -> bool {
        if let Some(rw) = self.db.get(name) {
            self.active.insert(name.to_string(), rw.clone()).is_none()
        } else {
            false
        }
    }

    /// Disables an entire category of rewrites.
    /// Returns the number of rewrite rules removed from the active set.
    pub fn disable_category(&mut self, category: &str) -> Option<usize> {
        if let Some(rw_names) = self.categories.get(category) {
            let mut count = 0;
            for name in rw_names {
                if self.active.remove(name).is_some() {
                    count += 1;
                }
            }
            Some(count)
        } else {
            None
        }
    }

    /// Disables an individual rewrite rule by name.
    /// Returns true if the rule was removed from the active set.
    pub fn disable_rule(&mut self, name: &str) -> bool {
        self.active.remove(name).is_some()
    }

    /// Returns the active rewrite rules
    pub fn active_rules(self) -> Vec<Rewrite<L, A>> {
        self.active.into_values().collect()
    }

    /// Returns an iterator to all the categories of rewrites
    pub fn categories(&self) -> impl Iterator<Item = &String> {
        self.categories.keys()
    }

    /// Returns the hash of active rewrites rules as a string
    pub fn rules_hash(&self) -> String
    where
        L: Language + FromOp + Display + 'static,
        A: Analysis<L> + Clone + 'static,
    {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        let hash_value = s.finish();
        format!("{:x}", hash_value)
    }

    /// Returns the number of activated rewrite rules
    pub fn num_active(&self) -> usize {
        self.active.len()
    }
}

impl<L, A> RewriteManager<L, A>
where
    L: Language + FromOp + Send + Sync + 'static,
    A: Analysis<L> + Clone,
{
    /// Constructs and inserts a rewrite rule in place
    fn construct_rule(
        &mut self,
        name: &str,
        lhs: &str,
        rhs: &str,
        bidirectional: bool,
        category: Option<String>,
    ) -> Result<Rewrite<L, A>, String> {
        let lhsp: Pattern<L> = lhs
            .parse()
            .map_err(|e: egg::RecExprParseError<_>| format!("lhs: {:?}", e))?;
        let rhsp: Pattern<L> = rhs
            .parse()
            .map_err(|e: egg::RecExprParseError<_>| format!("rhs: {:?}", e))?;
        let rw: Rewrite<L, A> = Rewrite::new(Symbol::new(name), lhsp, rhsp)?;
        if let Some(cat) = category.clone() {
            self.insert_into_category(cat, rw.clone())
                .map_err(|r| format!("Rule already exists: {}", r.name))?;
        } else {
            self.insert_rule(rw.clone())
                .map_err(|r| format!("Rule already exists: {}", r.name))?;
        }

        if bidirectional {
            self.construct_rule(&format!("{name}-rev"), rhs, lhs, false, category)?;
        }

        Ok(rw)
    }

    /// Parse rewrite rules from a reader, line by line.
    /// Lines starting with `#` are comments and ignored.
    /// Category lines end with a colon `:` and set the category for subsequent rules.
    /// Rules are formatted as `name: lhs => rhs` or `name: lhs <=> rhs` for bidirectional rules.
    /// Here is an example file:
    /// ```text
    /// # Algebraic rules
    /// algebraic:
    ///     commutative-and: (AND2 ?a ?b) => (AND2 ?b ?a)
    /// ```
    pub fn parse_rules(&mut self, file: impl Read) -> Result<(), String> {
        let mut category: Option<String> = None;
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|e| format!("Rewrite reader: {:?}", e))?;

            // Skip comments and empty lines
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Category line has no rule. Only colon.
            if let Some(line) = line.strip_suffix(':') {
                category = Some(line.to_string());
                continue;
            }

            let bidrectional = line.contains("<=>");

            let (name, rule) = line
                .split_once(":")
                .ok_or_else(|| format!("Rule misformatted: {}", line))?;

            let (lhs, rhs) = if bidrectional {
                rule.split_once("<=>")
                    .ok_or_else(|| format!("Bidirectional rule misformatted: {}", line))?
            } else {
                rule.split_once("=>")
                    .ok_or_else(|| format!("Rule misformatted: {}", line))?
            };

            self.construct_rule(
                name.trim(),
                lhs.trim(),
                rhs.trim(),
                bidrectional,
                category.clone(),
            )?;
        }

        Ok(())
    }
}

#[test]
fn test_parse_rules() {
    let mut manager = RewriteManager::<lut::LutLang, LutAnalysis>::new();

    assert!(
        manager
            .construct_rule(
                "test-rule",
                "(LUT 3 ?a)",
                "true",
                false,
                Some("constant-folding".to_string())
            )
            .is_ok()
    );

    // Repeated rule
    assert!(
        manager
            .construct_rule(
                "test-rule",
                "(LUT 3 ?a)",
                "true",
                false,
                Some("constant-folding".to_string())
            )
            .is_err()
    );

    // Bad syntax
    assert!(
        manager
            .construct_rule(
                "test-rule2",
                "(LUTf 3 ?a)",
                "true",
                false,
                Some("constant-folding".to_string())
            )
            .is_err()
    );

    assert_eq!(manager.clone().active_rules().len(), 0);

    manager.enable_rule("test-rule");

    assert_eq!(manager.clone().active_rules().len(), 1);

    // Can't be bidirectional
    assert!(
        manager
            .construct_rule(
                "test-rule3",
                "(LUT 3 ?a)",
                "true",
                true,
                Some("constant-folding".to_string())
            )
            .is_err()
    );
}
