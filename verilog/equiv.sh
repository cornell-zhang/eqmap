#!/bin/bash -e

# Copyright 2025 The EqMap Authors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.


echo "Checking $1 against $2 (golden model)"

SCRIPT=equiv.ys
SIMLIB=simlib.v

echo "read_verilog $SIMLIB" > equiv.ys
echo "read_verilog $2" >> equiv.ys
echo "hierarchy -auto-top" >> equiv.ys
echo "flatten" >> equiv.ys
echo "proc" >> equiv.ys
echo "rename -top gold" >> equiv.ys
echo "design -stash gold" >> equiv.ys

echo "read_verilog $SIMLIB" >> equiv.ys
echo "read_verilog $1" >> equiv.ys
echo "hierarchy -auto-top" >> equiv.ys
echo "flatten" >> equiv.ys
echo "proc" >> equiv.ys
echo "rename -top rewritten" >> equiv.ys
echo "design -stash rewritten" >> equiv.ys

echo "design -copy-from gold gold" >> equiv.ys
echo "design -copy-from rewritten rewritten" >> equiv.ys

echo "equiv_make gold rewritten equiv" >> equiv.ys
echo "equiv_simple equiv" >> equiv.ys
echo "equiv_status equiv" >> equiv.ys

yosys -s equiv.ys
