# Verilog Sources

The files in this directory need to be distributed with the tool in order to be able to run most experiments.

- `celllang.v` This file provides the tech mapping library for Yosys to convert the RTL logic into a netlist of AND, OR, and INV gates. It is meant to be used with the `msynth` and `cellmap` tools.

- `lutlang.v` This file provides the techmapping library for Yosys to convert the RTL logic into a netlist of XOR, NOR, AND, and INV gates. It is meant to be used with the `lvv` and `epak` tools.

- `simlib.v` This is used to do RTL equivalence checking on FPGA LUT netlists.

- `stdcells.v` Based on the Nangate 45nm Open Cell Library used to do RTL equivalence checking on synthesized ASIC netlists.
