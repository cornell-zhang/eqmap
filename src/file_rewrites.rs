/*!

  Language-agnostic trait for generating e-graph rewrite rules from external files.

  This module defines the `FileRewrites` trait that abstracts over different language types
  (e.g., LutLang, CellLang) and provides implementations for dynamically parsing and
  generating rewrite rules from external text files.

  # Trait Design

  The `FileRewrites` trait is generic over a language `L` and includes an associated
  `Analysis` type. Implementors must provide the `file_rewrites()` method that:

  1. Opens and reads the file at the given path
  2. Parses the FILTER_LIST and rule definitions using the rewrite_file parser
  3. Filters out rules based on the exclusion list
  4. Generates `egg::Rewrite` instances for each remaining rule
  5. Returns a `Vec<egg::Rewrite<L, Self::Analysis>>`

  # Pattern-to-Pattern Rewrites

  The current implementation supports only pattern-to-pattern rewrites using egg's
  pattern syntax. Searchers and appliers are parsed as standard egg patterns:

  ```text
  "(LUT ?p ?a ?b)" => "(MUX ?p ?a ?b)"
  ```

  Variables are prefixed with `?` and can be reused in the applier pattern.

  # Future Extensions

  To support code-based searchers and appliers, consider:
  - Creating a registry of pre-defined searcher/applier constructors
  - Using procedural macros to generate rules at compile time
  - Creating a domain-specific language for expressing decomposition functions

*/

use egg::{Language, Rewrite, Analysis, Pattern, PatternAst};

/// A trait for language types that can generate rewrite rules from external files
///
/// # Associated Types
/// * `Analysis` - The analysis type used with this language (e.g., `LutAnalysis`, `CellAnalysis`)
///
/// # Methods
/// * `file_rewrites()` - Parses a file and generates a vector of rewrite rules
///
/// # Example
///
/// ```ignore
/// let rules = LutLang::file_rewrites("rules.txt")?;
/// ```
pub trait FileRewrites: Language
where
    Self: Sized,
{
    /// The analysis type associated with this language
    type Analysis: Analysis<Self>;

    /// Parse and generate rewrite rules from a file path
    ///
    /// # Arguments
    /// * `path` - Path to the rewrite rules file
    ///
    /// # Returns
    /// A vector of `Rewrite` instances, or a boxed error
    ///
    /// # Errors
    /// Returns an error if:
    /// - The file cannot be read
    /// - The file format is invalid
    /// - Rule patterns cannot be parsed
    fn file_rewrites(
        path: &str,
    ) -> Result<Vec<Rewrite<Self, Self::Analysis>>, Box<dyn std::error::Error>>;
}

/// Helper function to create pattern-based rewrite rules
///
/// This function takes the searcher and applier pattern strings and creates
/// rewrite rules. For bidirectional rules, it creates both forward and reverse.
///
/// # Arguments
/// * `name` - Name of the rule (for identification)
/// * `searcher_str` - Searcher pattern as a string
/// * `applier_str` - Applier pattern as a string
/// * `bidirectional` - If true, creates both forward and reverse rules
///
/// # Returns
/// A `Vec<Rewrite>` containing one or two rules, or an error if patterns cannot be parsed
#[cfg(feature = "rewrite_file")]
pub fn create_pattern_rewrites<L, A>(
    name: &str,
    searcher_str: &str,
    applier_str: &str,
    bidirectional: bool,
) -> Result<Vec<Rewrite<L, A>>, Box<dyn std::error::Error>>
where
    L: Language + Send + Sync + egg::FromOp + 'static,
    A: Analysis<L>,
    <L as egg::FromOp>::Error: std::error::Error + 'static,
{
    // Parse the searcher pattern
    let searcher_ast: PatternAst<L> = searcher_str.parse()
        .map_err(|e: egg::RecExprParseError<_>| Box::new(e) as Box<dyn std::error::Error>)?;
    let searcher = Pattern::new(searcher_ast);

    // Parse the applier pattern
    let applier_ast: PatternAst<L> = applier_str.parse()
        .map_err(|e: egg::RecExprParseError<_>| Box::new(e) as Box<dyn std::error::Error>)?;
    let applier = Pattern::new(applier_ast);

    let mut rewrites = Vec::new();

    // Create the forward rewrite
    let forward_name = name.to_string();
    let forward = Rewrite::new(forward_name, searcher, applier)?;
    rewrites.push(forward);

    // For bidirectional rules, also create the reverse rewrite
    if bidirectional {
        let searcher_ast: PatternAst<L> = applier_str.parse()
            .map_err(|e: egg::RecExprParseError<_>| Box::new(e) as Box<dyn std::error::Error>)?;
        let searcher_rev = Pattern::new(searcher_ast);

        let applier_ast: PatternAst<L> = searcher_str.parse()
            .map_err(|e: egg::RecExprParseError<_>| Box::new(e) as Box<dyn std::error::Error>)?;
        let applier_rev = Pattern::new(applier_ast);

        let reverse_name = format!("{}-rev", name);
        let reverse = Rewrite::new(reverse_name, searcher_rev, applier_rev)?;
        rewrites.push(reverse);
    }

    Ok(rewrites)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trait_exists() {
        // This is a compile-time check that the trait is properly defined
        // Concrete implementations will be provided in language-specific modules
    }
}
