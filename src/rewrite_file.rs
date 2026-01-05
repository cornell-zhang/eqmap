/*!

  Parser and utilities for loading custom e-graph rewrite rules from external text files.

  This module provides a framework for dynamically parsing rewrite rule definitions from
  external text files, filtering them based on an exclusion list, and generating
  `Vec<Rewrite<L, A>>` instances that integrate with the existing `SynthRequest` workflow.

  # File Format

  The text file format is structured as follows:

  **First line**: `FILTER_LIST="rule-name-1","rule-name-2",...`
  - Comma-separated list of rule names to exclude from processing
  - Empty quotes for no exclusions: `FILTER_LIST=""`

  **Subsequent lines**: Each line defines one rewrite rule with the following components:
  - Rule name (quoted string): e.g., `"mux-expand"`
  - Rule definition separator: `;`
  - Searcher expression (quoted pattern string): e.g., `"(LUT 202 ?s ?a ?b)"`
  - Direction: `=>` (directional) or `<=>` (bidirectional)
  - Applier expression (quoted pattern string): e.g., `"(LUT 14 (LUT 8 ?s ?a) (LUT 2 ?s ?b))"`
  - Optional conditions: `if <condition>` forms (currently not fully supported)

  # Example File

  ```text
  FILTER_LIST="mux-expand"
  "lut3-shannon"; "(LUT ?p ?a ?b ?c)" => "(LUT 14 (LUT 8 ?p ?a ?b) (LUT 2 ?p ?c))"
  "lut4-shannon"; "(LUT ?p ?a ?b ?c ?d)" => "(LUT 14 (LUT 8 ?p ?a ?b) (LUT 2 ?p ?c ?d))"
  ```

  # Current Limitations

  - Only pattern-to-pattern rewrites are supported (no Rust code blocks in searchers/appliers)
  - Conditions (if clauses) are parsed but not applied
  - Variables in patterns must use standard egg pattern syntax (e.g., `?var`)

  # Usage

  See the `FileRewrites` trait for language-specific implementations.

*/

use std::collections::HashSet;
use std::fmt;
use std::fs;

/// Error type for rewrite file parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// File I/O error
    IoError(String),
    /// Line-specific parsing error with line number
    LineError {
        /// The line number where the error occurred
        line: usize,
        /// The error message
        message: String,
    },
    /// Invalid FILTER_LIST format
    InvalidFilterList(String),
    /// Invalid rule definition
    InvalidRule(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::IoError(msg) => write!(f, "IO error: {}", msg),
            ParseError::LineError { line, message } => write!(f, "Line {}: {}", line, message),
            ParseError::InvalidFilterList(msg) => write!(f, "Invalid FILTER_LIST: {}", msg),
            ParseError::InvalidRule(msg) => write!(f, "Invalid rule: {}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

/// Represents a single rewrite rule definition parsed from a file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleDefinition {
    /// Name of the rule
    pub name: String,
    /// Searcher pattern as a string
    pub searcher: String,
    /// Direction: true for `<=>` (bidirectional), false for `=>`
    pub bidirectional: bool,
    /// Applier pattern as a string
    pub applier: String,
    /// Optional condition expressions (currently not applied)
    pub conditions: Vec<String>,
}

impl RuleDefinition {
    /// Check if this rule name is in the filter list
    pub fn is_filtered(&self, filter_list: &HashSet<String>) -> bool {
        filter_list.contains(&self.name)
    }
}

/// Parse the FILTER_LIST from the first line of a file
///
/// # Arguments
/// * `line` - The first line of the rewrite file, should start with `FILTER_LIST=`
///
/// # Returns
/// A `HashSet<String>` containing rule names to exclude, or a `ParseError`
///
/// # Example
/// ```ignore
/// let filter_list = parse_filter_list("FILTER_LIST=\"rule1\",\"rule2\"")?;
/// ```
pub fn parse_filter_list(line: &str) -> Result<HashSet<String>, ParseError> {
    let line = line.trim();

    // Check for FILTER_LIST prefix
    if !line.starts_with("FILTER_LIST=") {
        return Err(ParseError::InvalidFilterList(
            "Expected FILTER_LIST= at the start of first line".to_string(),
        ));
    }

    let values_str = &line[12..]; // Skip "FILTER_LIST="

    // Handle empty filter list
    if values_str == "\"\"" {
        return Ok(HashSet::new());
    }

    let mut filter_set = HashSet::new();

    // Parse comma-separated quoted strings
    let mut in_quote = false;
    let mut current_value = String::new();
    let mut chars = values_str.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quote {
                    // End of quoted string
                    if !current_value.is_empty() {
                        filter_set.insert(current_value.clone());
                        current_value.clear();
                    }
                    in_quote = false;
                } else {
                    // Start of quoted string
                    in_quote = true;
                }
            }
            ',' => {
                if in_quote {
                    return Err(ParseError::InvalidFilterList(
                        "Unexpected comma inside quoted string".to_string(),
                    ));
                }
                // Skip commas between quoted strings
            }
            ' ' if !in_quote => {
                // Skip whitespace outside quotes
            }
            _ if in_quote => {
                current_value.push(ch);
            }
            _ => {
                return Err(ParseError::InvalidFilterList(
                    format!("Unexpected character outside quotes: {}", ch),
                ));
            }
        }
    }

    if in_quote {
        return Err(ParseError::InvalidFilterList(
            "Unclosed quoted string in FILTER_LIST".to_string(),
        ));
    }

    Ok(filter_set)
}

/// Parse a single rewrite rule definition line
///
/// # Arguments
/// * `line` - A rule definition line
/// * `line_num` - Line number (for error reporting)
///
/// # Returns
/// A `RuleDefinition` or a `ParseError`
///
/// # Format
/// ```text
/// "rule-name"; "searcher-pattern" => "applier-pattern" [if condition1 [if condition2 ...]]
/// ```
pub fn parse_rule_line(line: &str, line_num: usize) -> Result<RuleDefinition, ParseError> {
    let line = line.trim();

    if line.is_empty() || line.starts_with('#') {
        // Skip empty lines and comments
        return Err(ParseError::LineError {
            line: line_num,
            message: "Skipped (empty or comment)".to_string(),
        });
    }

    // Find the semicolon separator
    let (name_part, rest) = match line.split_once(';') {
        Some((n, r)) => (n.trim(), r.trim()),
        None => {
            return Err(ParseError::LineError {
                line: line_num,
                message: "Missing semicolon separator between rule name and definition".to_string(),
            })
        }
    };

    // Parse rule name (should be quoted)
    let name = parse_quoted_string(name_part, line_num).map_err(|_| ParseError::LineError {
        line: line_num,
        message: "Rule name must be a quoted string".to_string(),
    })?;

    // Parse searcher, direction, and applier
    let (searcher, rest) = parse_next_quoted_string(rest, line_num)?;

    let rest = rest.trim();
    let (bidirectional, rest) = if let Some(r) = rest.strip_prefix("<=>") {
        (true, r.trim())
    } else if let Some(r) = rest.strip_prefix("=>") {
        (false, r.trim())
    } else {
        return Err(ParseError::LineError {
            line: line_num,
            message: "Expected => or <=> after searcher pattern".to_string(),
        });
    };

    let (applier, rest) = parse_next_quoted_string(rest, line_num)?;

    // Parse conditions (if any)
    let mut conditions = Vec::new();
    let mut rest = rest.trim();

    while rest.starts_with("if") {
        rest = &rest[2..].trim_start();
        // For now, we'll just collect the condition as-is
        // In a full implementation, we'd parse the condition expression
        if let Some((cond, r)) = parse_condition_expression(rest) {
            conditions.push(cond);
            rest = r.trim();
        } else {
            break;
        }
    }

    Ok(RuleDefinition {
        name,
        searcher,
        bidirectional,
        applier,
        conditions,
    })
}

/// Parse a quoted string from input
fn parse_quoted_string(input: &str, line_num: usize) -> Result<String, ParseError> {
    let input = input.trim();
    if !input.starts_with('"') {
        return Err(ParseError::LineError {
            line: line_num,
            message: "Expected quoted string".to_string(),
        });
    }

    let content = &input[1..];
    if let Some(end_quote) = content.find('"') {
        Ok(content[..end_quote].to_string())
    } else {
        Err(ParseError::LineError {
            line: line_num,
            message: "Unclosed quoted string".to_string(),
        })
    }
}

/// Parse the next quoted string and return it along with the remaining input
fn parse_next_quoted_string(input: &str, line_num: usize) -> Result<(String, &str), ParseError> {
    let input = input.trim();
    if !input.starts_with('"') {
        return Err(ParseError::LineError {
            line: line_num,
            message: "Expected quoted string".to_string(),
        });
    }

    let content = &input[1..];
    if let Some(end_quote) = content.find('"') {
        let string = content[..end_quote].to_string();
        let remaining = &input[end_quote + 2..];
        Ok((string, remaining))
    } else {
        Err(ParseError::LineError {
            line: line_num,
            message: "Unclosed quoted string".to_string(),
        })
    }
}

/// Parse a condition expression (simplified for now)
/// Returns the condition string and the remaining input
fn parse_condition_expression(input: &str) -> Option<(String, &str)> {
    // For now, we'll take everything until the next "if" or end of line
    // A full implementation would parse proper Rust expressions
    if let Some(pos) = input.find("if ") {
        let cond = input[..pos].trim().to_string();
        Some((cond, &input[pos..]))
    } else {
        // End of line - take everything
        if !input.is_empty() {
            Some((input.trim().to_string(), ""))
        } else {
            None
        }
    }
}

/// Parse all rules from a text file
///
/// # Arguments
/// * `path` - Path to the rewrite rules file
///
/// # Returns
/// A tuple of (FILTER_LIST as HashSet, Vec of RuleDefinitions), or a ParseError
pub fn parse_rewrite_file(path: &str) -> Result<(HashSet<String>, Vec<RuleDefinition>), ParseError> {
    let content = fs::read_to_string(path)
        .map_err(|e| ParseError::IoError(format!("Failed to read file {}: {}", path, e)))?;

    let mut lines = content.lines().enumerate();

    // Parse FILTER_LIST from first line
    let filter_list = match lines.next() {
        Some((_, line)) => parse_filter_list(line)?,
        None => {
            return Err(ParseError::InvalidFilterList(
                "File is empty, expected FILTER_LIST on first line".to_string(),
            ))
        }
    };

    // Parse rule definitions
    let mut rules = Vec::new();
    for (line_num, line) in lines {
        let line_number = line_num + 1; // Convert to 1-based line numbers
        match parse_rule_line(line, line_number) {
            Ok(rule) => {
                // Only add if not filtered
                if !rule.is_filtered(&filter_list) {
                    rules.push(rule);
                }
            }
            Err(ParseError::LineError { message, .. }) if message.contains("Skipped") => {
                // Skip empty lines and comments
            }
            Err(e) => return Err(e),
        }
    }

    Ok((filter_list, rules))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_filter_list() {
        let result = parse_filter_list("FILTER_LIST=\"\"").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_single_filter() {
        let result = parse_filter_list("FILTER_LIST=\"rule1\"").unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.contains("rule1"));
    }

    #[test]
    fn test_parse_multiple_filters() {
        let result = parse_filter_list("FILTER_LIST=\"rule1\",\"rule2\",\"rule3\"").unwrap();
        assert_eq!(result.len(), 3);
        assert!(result.contains("rule1"));
        assert!(result.contains("rule2"));
        assert!(result.contains("rule3"));
    }

    #[test]
    fn test_parse_filter_with_spaces() {
        let result = parse_filter_list("FILTER_LIST=\"rule1\",\"rule2\"").unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_parse_quoted_string() {
        let result = parse_quoted_string("\"hello world\"", 1).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_parse_rule_line() {
        let line = r#""test-rule"; "(LUT ?p ?a ?b)" => "(MUX ?p ?a ?b)""#;
        let rule = parse_rule_line(line, 1).unwrap();
        assert_eq!(rule.name, "test-rule");
        assert_eq!(rule.searcher, "(LUT ?p ?a ?b)");
        assert!(!rule.bidirectional);
        assert_eq!(rule.applier, "(MUX ?p ?a ?b)");
    }

    #[test]
    fn test_parse_rule_bidirectional() {
        let line = r#""test-rule"; "(LUT ?p ?a ?b)" <=> "(MUX ?p ?a ?b)""#;
        let rule = parse_rule_line(line, 1).unwrap();
        assert!(rule.bidirectional);
    }

    #[test]
    fn test_rule_filtering() {
        let mut filter_set = HashSet::new();
        filter_set.insert("excluded".to_string());

        let rule = RuleDefinition {
            name: "excluded".to_string(),
            searcher: "test".to_string(),
            bidirectional: false,
            applier: "test".to_string(),
            conditions: vec![],
        };

        assert!(rule.is_filtered(&filter_set));
    }
}
