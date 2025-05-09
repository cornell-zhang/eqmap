#!/bin/bash -e

echo "Checking $1 against $2 (golden model)"

SCRIPT=equiv.ys
SIMLIB="$(dirname $0)/stdcells.v"

echo "read_verilog $SIMLIB" > equiv.ys
echo "read_verilog $2" >> equiv.ys
echo "hierarchy -auto-top" >> equiv.ys
echo "flatten" >> equiv.ys
echo "rename -top gold" >> equiv.ys
echo "design -stash gold" >> equiv.ys

echo "read_verilog $SIMLIB" >> equiv.ys
echo "read_verilog $1" >> equiv.ys
echo "hierarchy -auto-top" >> equiv.ys
echo "flatten" >> equiv.ys
echo "rename -top rewritten" >> equiv.ys
echo "design -stash rewritten" >> equiv.ys

echo "design -copy-from gold gold" >> equiv.ys
echo "design -copy-from rewritten rewritten" >> equiv.ys

echo "equiv_make gold rewritten equiv" >> equiv.ys
echo "equiv_simple equiv" >> equiv.ys
echo "equiv_status equiv" >> equiv.ys

yosys -s equiv.ys
