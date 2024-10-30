![](https://github.com/matth2k/lut-synth/actions/workflows/rust.yml/badge.svg)

# lut-synth: LUT Network Synthesis with E-Graphs

## Description
An early experiment on representing LUT networks within E-Graphs for logic synthesis

### Dependencies
* [rustup](https://rustup.rs/)
  * Crates
    * [egg](https://docs.rs/egg/latest/egg/)
    * [bitvec](https://docs.rs/bitvec/latest/bitvec/)
    * [clap](https://docs.rs/clap/latest/clap/)
    * [indicatif](https://docs.rs/indicatif/latest/indicatif/)
* VSCode
  * [Rust Analyzer Extension](https://rust-analyzer.github.io/)
  * [VerilogHDL Extension](https://marketplace.visualstudio.com/items?itemName=mshr-h.VerilogHDL)
* [Verible](https://github.com/chipsalliance/verible)

### Installing
`cargo build`

`cargo run < examples.txt # Run the synthesizer on a few examples`

### Docs

You can generate most of the documentation with `cargo doc`.

Here is a rough outline of the type system defined by `LutLang`:

`<LutLang> ::= <Program> | <Node>`

`<Node> ::= <Const> | x | <Input> | NOR <Node> <Node> | MUX <Node> <Node> <Node> | LUT <Program> Node ... Node`

`<Const> ::= false | true // Base type is a bool`

`<Input> ::= <String> // Any string is parsed as an input variable`

`<Program> ::= <u64> // Can store a program for up to 6 bits`