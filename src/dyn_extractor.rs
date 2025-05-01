/*!

`lut-synth`: LUT Network Synthesis with E-Graphs

An exact extractor using dynamic programming.

*/

use super::asic::CellLang;
use super::verilog::PrimitiveType;
use egg::{Analysis, EGraph, Id, Language, RecExpr};
use serde_json::de;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

/// Cost function trait for dynamic programming extractor.
pub trait CostFunction<L: Language> {
    /// The int or float type used for the cost.
    type Cost: PartialOrd + std::fmt::Debug + Clone;
    /// Computes the cost of the given node.
    fn cost(&self, enode: &L) -> Self::Cost;

    /// How to combine the costs
    fn fold(&self, a: Self::Cost, b: Self::Cost) -> Self::Cost;

    /// Skip this node to filter it out of the solution
    fn skip(&self, enode: &L) -> bool;
}

fn deep_equals<L>(a: Id, a_e: &RecExpr<L>, b: Id, b_e: &RecExpr<L>) -> bool
where
    L: Language + std::fmt::Display,
{
    let mut an = a_e[a].clone();
    let bn = &b_e[b];

    if std::mem::discriminant(&an) != std::mem::discriminant(bn) {
        return false;
    }

    if an.children().len() != bn.children().len() {
        return false;
    }

    for (c, &d) in an.children_mut().iter_mut().zip(bn.children()) {
        if !deep_equals(*c, a_e, d, b_e) {
            return false;
        }
        *c = d;
    }

    if an == *bn {
        eprintln!("Found a match: {} == {}", an, bn);
        true
    } else {
        eprintln!("Found a match: {} == {}", an, bn);
        false
    }
}

fn merge_expr<L>(mut exprs: Vec<RecExpr<L>>) -> (RecExpr<L>, Vec<Id>)
where
    L: Language + std::fmt::Display,
{
    if exprs.is_empty() {
        return (RecExpr::default(), Vec::new());
    }

    let mut expr = exprs.remove(0);
    let mut mapping: Vec<Id> = vec![(expr.len() - 1).into()];
    for sub in exprs.into_iter() {
        let mut remapping: HashMap<Id, Id> = HashMap::new();
        for (b, n) in sub.iter().enumerate() {
            let mut inserted = false;
            for a in 0..expr.len() {
                if deep_equals(a.into(), &expr, b.into(), &sub) {
                    remapping.insert(b.into(), a.into());
                    inserted = true;
                    break;
                }
            }
            if !inserted {
                remapping.insert(
                    b.into(),
                    expr.add(n.clone().map_children(|c| remapping[&c])),
                );
            }
        }
        mapping.push(remapping[&(sub.len() - 1).into()]);
    }

    (expr, mapping)
}

/// An extractor similar to [egg::Extractor] with [egg:CostFunction]. However, it uses dynamic programming.
pub struct DynExtractor<'a, CF: CostFunction<L>, L: Language, A: Analysis<L>> {
    cost_function: CF,
    best_exprs: HashMap<Id, Option<RecExpr<L>>>,
    visited: HashSet<L>,
    egraph: &'a EGraph<L, A>,
}

impl<'a, CF, L, A> DynExtractor<'a, CF, L, A>
where
    CF: CostFunction<L>,
    CF::Cost: Default,
    L: Language + std::fmt::Display,
    A: Analysis<L>,
{
    pub fn new(egraph: &'a EGraph<L, A>, cost_function: CF) -> Self {
        DynExtractor {
            cost_function,
            best_exprs: HashMap::new(),
            visited: HashSet::new(),
            egraph,
        }
    }

    /// Find the best expression for the given eclass.
    pub fn find_best_expression(&mut self, eclass: Id) -> Option<RecExpr<L>> {
        if self.best_exprs.contains_key(&eclass) {
            return self.best_exprs[&eclass].clone();
        }

        let mut best_e: Option<RecExpr<L>> = None;
        let mut best_cost: Option<CF::Cost> = None;
        for node in self.egraph[eclass].nodes.iter() {
            if self.cost_function.skip(node) {
                continue;
            }

            if self.visited.contains(node) {
                continue;
            }

            self.visited.insert(node.clone());

            let c_expr: Vec<Option<RecExpr<L>>> = node
                .children()
                .iter()
                .map(|x| self.find_best_expression(*x))
                .collect();

            let mut impossible_node = false;
            for c in c_expr.iter() {
                if c.is_none() {
                    impossible_node = true;
                    break;
                }
            }

            if impossible_node {
                self.visited.remove(node);
                continue;
            }

            let c_expr: Vec<RecExpr<L>> = c_expr.into_iter().map(|x| x.unwrap()).collect();
            let (mut expr, children) = merge_expr(c_expr);
            let mut remapped = node.clone();
            for (i, child) in remapped.children_mut().iter_mut().enumerate() {
                *child = children[i];
            }
            expr.add(remapped);

            let total_cost = expr
                .iter()
                .map(|x| self.cost_function.cost(x))
                .fold(CF::Cost::default(), |a, b| self.cost_function.fold(a, b));

            if best_cost.is_none() || total_cost <= *best_cost.as_ref().unwrap() {
                best_cost = Some(total_cost);
                best_e = Some(expr);
            }
            self.visited.remove(node);
        }

        self.best_exprs.insert(eclass, best_e.clone());
        best_e
    }
}

pub struct ASICCostFunction;

impl CostFunction<CellLang> for ASICCostFunction {
    type Cost = f32;

    fn cost(&self, enode: &CellLang) -> Self::Cost {
        match enode {
            CellLang::Const(_) | CellLang::Var(_) => PrimitiveType::INV.get_min_area().unwrap(),
            CellLang::Bus(_) => 0.0,
            CellLang::Cell(n, _l) => {
                let prim = PrimitiveType::from_str(n.as_str()).unwrap();
                prim.get_min_area().unwrap_or(1.33)
            }
            _ => f32::MAX,
        }
        // match enode {
        //     CellLang::Const(_) | CellLang::Var(_) => PrimitiveType::INV.get_min_area().unwrap(),
        //     CellLang::Bus(_) => 0.0,
        //     CellLang::Cell(n, _l) => 1.0,
        //     _ => f32::MAX,
        // }
    }

    fn fold(&self, a: Self::Cost, b: Self::Cost) -> Self::Cost {
        a + b
    }

    fn skip(&self, enode: &CellLang) -> bool {
        matches!(enode, CellLang::Or(_) | CellLang::And(_) | CellLang::Inv(_))
    }
}

#[test]
fn test_deep_equals() {
    let expra: RecExpr<CellLang> = "(INV (AND_X1 a b))".parse().unwrap();
    let exprb: RecExpr<CellLang> = "(AND_X1 a b)".parse().unwrap();
    assert!(deep_equals(2.into(), &expra, 2.into(), &exprb));
    let vec = vec![expra.clone(), exprb.clone(), expra, exprb];
    let (expr, mapping) = merge_expr(vec);
    eprintln!("Merged expr: {:?}", expr);
    eprintln!("Mapping : {:?}", mapping);
    assert!(expr.len() == 4);
}
