# EqMap Rewrite File Feature

## Overview

The `rewrite_file` feature enables users to define custom e-graph rewrite rules in external text files, allowing for dynamic and flexible optimization without modifying source code. This feature is particularly useful for researching new decomposition strategies, gate mapping rules, and logic optimization patterns.

## Feature Flag

Enable the feature in your build:

```bash
cargo build --features rewrite_file
```

Or add to your `Cargo.toml`:

```toml
[features]
default = ["dyn_decomp", "egg/lp", "rewrite_file"]
```

## Command-Line Usage

All EqMap tools support the `-F` or `--rewrite-file` option:

```bash
eqmap_fpga -F rules.txt input.v output.v
eqmap_asic -F rules.txt input.v output.v
opt -F rules.txt expression
optcell -F rules.txt expression
```

If a rewrite file is provided, it **replaces** the default ruleset. To use custom rules in addition to standard rules, you'll need to include them in the file.

## File Format

### Structure

The file consists of:
1. **FILTER_LIST** (first line): Rules to exclude
2. **Rule definitions** (subsequent lines): One rule per line

### Example File

```
FILTER_LIST="lut6-shannon"
"lut3-shannon"; "(LUT ?p ?a ?b ?c)" => "(LUT 14 (LUT 8 ?p ?a ?b) (LUT 2 ?p ?c))"
"lut4-shannon"; "(LUT ?p ?a ?b ?c ?d)" => "(LUT 14 (LUT 8 ?p ?a ?b) (LUT 2 ?p ?c ?d))"
"and-gate"; "(LUT 8 ?a ?b)" <=> "(AND ?a ?b)"
```

### FILTER_LIST Syntax

The first line must start with `FILTER_LIST=` followed by a comma-separated list of rule names (in quotes):

```
FILTER_LIST=""                           # Empty filter list (no rules excluded)
FILTER_LIST="rule1"                      # Exclude one rule
FILTER_LIST="rule1","rule2","rule3"      # Exclude multiple rules
```

### Rule Definition Syntax

Each rule line follows this format:

```
"rule-name"; "searcher-pattern" => "applier-pattern"
"rule-name"; "searcher-pattern" <=> "applier-pattern"
```

- **Rule name** (quoted): A unique identifier for the rule (e.g., `"lut3-shannon"`)
- **Separator**: Semicolon (`;`) separates the name from the definition
- **Searcher pattern** (quoted): The pattern to match in the e-graph
- **Direction**: 
  - `=>` for unidirectional rules (searcher → applier only)
  - `<=>` for bidirectional rules (both searcher → applier and applier → searcher)
- **Applier pattern** (quoted): The replacement pattern

### Pattern Syntax

Patterns follow egg's pattern syntax:

- **Variables**: Prefixed with `?` (e.g., `?a`, `?p`, `?x`)
- **Operators**: Use the language's constructor syntax
- **LutLang patterns**: 
  - `(LUT <program> <input1> <input2> ...)`
  - `(AND ?a ?b)`, `(OR ?a ?b)`, `(XOR ?a ?b)`, `(NOT ?a)`, `(MUX ?s ?a ?b)`
- **CellLang patterns**: Standard cell names like `(AND ?a ?b)`, `(OR ?a ?b)`, etc.

### Examples

#### Decompose a 3-LUT using Shannon expansion
```
"lut3-shannon"; "(LUT ?p ?a ?b ?c)" => "(LUT 14 (LUT 8 ?p ?a ?b) (LUT 2 ?p ?c))"
```

#### Bidirectional LUT-to-gate mapping
```
"lut-to-and"; "(LUT 8 ?a ?b)" <=> "(AND ?a ?b)"
```

#### 2-to-1 Multiplexer (LUT program 14)
```
"mux-pattern"; "(LUT 14 ?s ?a ?b)" <=> "(MUX ?s ?a ?b)"
```

#### NOT gate (LUT program 1 for single input)
```
"not-gate"; "(LUT 1 ?a)" <=> "(NOT ?a)"
```

## Limitations and Notes

### Current Limitations

1. **Pattern-to-pattern only**: Rules must be expressed as pattern rewrites. Code-based searchers and appliers (e.g., custom Rust functions) are not yet supported.

2. **No condition support**: The parser recognizes `if` clauses in rule definitions but does not yet apply them. Conditional rewrites may be added in future versions.

3. **LutLang only for now**: CellLang support is available but may have limited compatibility due to trait bounds. LutLang is recommended for custom rewrite files.

4. **Bidirectional expansion**: Bidirectional rules (`<=>`) are automatically split into two separate forward/backward rules internally.

### Important Behavior

- When a rewrite file is specified, it **completely replaces** the default ruleset
- The FILTER_LIST is parsed but currently serves documentation purposes
- Large numbers of rules may impact e-graph growth and performance
- Rules are applied during the saturation process, subject to the iteration and node limits

## Examples Provided

### `examples/lut_shannon_rules.txt`
Demonstrates Shannon expansion rules for 3-6 LUTs:
```bash
eqmap_fpga -F examples/lut_shannon_rules.txt input.v output.v
```

### `examples/gate_rules.txt`
Shows bidirectional LUT-to-gate mapping rules:
```bash
eqmap_fpga -F examples/gate_rules.txt circuit.v optimized.v
```

## Troubleshooting

### "Failed to load rewrite file"
- Check that the file path is correct and readable
- Verify the FILTER_LIST format (must start with `FILTER_LIST=`)
- Ensure each rule line is properly formatted with semicolon separator

### "Invalid rule definition"
- Check that rule names and patterns are properly quoted
- Use `=>` or `<=>` (not `->` or `<==>`)
- Verify pattern syntax matches your language (LutLang or CellLang)

### Rules not being applied
- Check the iteration limit (`-n`) and node limit (`-s`)
- Verify rules are compatible with your input circuit
- Use `-v/--verbose` to see which rules are being applied

## Future Enhancements

Potential improvements for future versions:

1. **Conditional rewrites**: Full support for `if <condition>` clauses
2. **Code-based rules**: Support for custom searcher/applier constructors
3. **Rule composition**: Combining rules into higher-level strategies
4. **Performance profiling**: Per-rule statistics on applicability
5. **Rule validation**: Pre-checking that patterns are well-formed

## Implementation Details

### Parser

The rewrite file parser (`rewrite_file.rs`):
- Reads the entire file into memory
- Parses the FILTER_LIST from the first line
- Parses each subsequent line as a rule definition
- Returns errors with line numbers for debugging

### Trait-Based Generation

The `FileRewrites` trait (`file_rewrites.rs`):
- Abstracts over different language types (LutLang, CellLang)
- Creates pattern-based rewrite rules from file definitions
- Generates both forward and reverse rules for bidirectional definitions

### Integration

Each binary tool:
- Accepts `-F/--rewrite-file <PATH>` argument
- Loads and applies custom rules when the argument is provided
- Falls back to default rules if not specified
- Reports the number of loaded rules in verbose mode

## See Also

- [EqMap Documentation](https://github.com/matth2k/eqmap)
- [egg Library Documentation](https://docs.rs/egg/)
- [Pattern Syntax Guide](https://docs.rs/egg/latest/egg/struct.Pattern.html)
