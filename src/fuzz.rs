/*!

  Random extraction for fuzzing

*/
use egg::RecExpr;
use rand::{prelude::*, rng};
/// A randomized extractor to use for program fuzzing.
pub struct RandomExtract {
    choice_func: fn(usize, usize) -> usize,
}

impl RandomExtract {
    /// Build an equivalent expression with random choices.
    /// The function returns the ID of term in the output expression.
    fn extract_term<L, A>(
        &self,
        egraph: &egg::EGraph<L, A>,
        id: egg::Id,
        expr: &mut egg::RecExpr<L>,
    ) -> egg::Id
    where
        L: egg::Language,
        A: egg::Analysis<L>,
    {
        let choices = &egraph[id].nodes;
        let choice = (self.choice_func)(0, choices.len());
        let node = choices[choice]
            .clone()
            .map_children(|child| self.extract_term(egraph, child, expr));

        expr.add(node)
    }

    /// Create a randomized extractor using the rand crate.
    pub fn new() -> Self {
        RandomExtract {
            choice_func: |min, max| rng().random_range(min..max),
        }
    }

    /// Use an arbitrary choice function to perform extraction with
    pub fn with_choice_func(choice_func: fn(usize, usize) -> usize) -> Self {
        RandomExtract { choice_func }
    }

    /// Extract a random expression from the egraph using rand crate.
    pub fn extract<L, A>(&self, egraph: &egg::EGraph<L, A>, id: egg::Id) -> RecExpr<L>
    where
        L: egg::Language,
        A: egg::Analysis<L>,
    {
        let mut expr = RecExpr::default();
        let id = egraph.find(id);
        self.extract_term(egraph, id, &mut expr);
        expr
    }
}

impl Default for RandomExtract {
    fn default() -> Self {
        Self::new()
    }
}
