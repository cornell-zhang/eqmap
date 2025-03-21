/*!

  Random extraction for fuzzing

*/
use egg::RecExpr;
use rand::{prelude::*, rng};
/// A randomized extractor to use for program fuzzing.
pub struct RandomExtract<L> {
    choice_func: fn(&[L]) -> usize,
}

impl<L> RandomExtract<L>
where
    L: egg::Language,
{
    /// Build an equivalent expression with random choices.
    /// The function returns the ID of term in the output expression.
    fn extract_term<A>(
        &self,
        egraph: &egg::EGraph<L, A>,
        id: egg::Id,
        expr: &mut egg::RecExpr<L>,
    ) -> egg::Id
    where
        A: egg::Analysis<L>,
    {
        let choices = &egraph[id].nodes;
        let choice = (self.choice_func)(choices);
        let node = choices[choice]
            .clone()
            .map_children(|child| self.extract_term(egraph, child, expr));

        expr.add(node)
    }

    /// Create a randomized extractor using the rand crate.
    pub fn new() -> Self {
        RandomExtract {
            choice_func: |choices| rng().random_range(0..choices.len()),
        }
    }

    /// Use an arbitrary choice function to perform extraction with
    pub fn with_choice_func(choice_func: fn(&[L]) -> usize) -> Self {
        RandomExtract { choice_func }
    }

    /// Extract a random expression from the egraph using rand crate.
    pub fn extract<A>(&self, egraph: &egg::EGraph<L, A>, id: egg::Id) -> RecExpr<L>
    where
        A: egg::Analysis<L>,
    {
        let mut expr = RecExpr::default();
        let id = egraph.find(id);
        self.extract_term(egraph, id, &mut expr);
        expr
    }
}

impl<L> Default for RandomExtract<L>
where
    L: egg::Language,
{
    fn default() -> Self {
        Self::new()
    }
}
