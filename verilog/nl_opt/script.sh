#!/bin/bash

# Source the EqMap tools
source ../../utils/setup.sh

# Check out the driving example. There is a mini-diagram inside.
cat feedback.v | grep //

# Check out the nl_opt tool we will using
nl_opt --help

# We will be comparing graph partitioning techniques
# 1. Isolating all register to register paths (disconnect-registers)
# 2. A more minimal feedback edge set (disconnect-arc-set)

# First let's just visualize the initial netlist.
# Do you have dot to create an image?
which dot

# It should look pretty similar
nl_opt feedback.v --passes clean-vis,dot-graph | dot -Tpng > feedback.png
echo "Open feedback.png in any image viewer"
read -n 1 -s -r -p "Press any key to continue"

# We can isolate the combinational logic by disconnecting all the registers
nl_opt feedback.v --passes disconnect-registers,clean-vis,dot-graph | dot -Tpng > feedback_disconn_regs.png
echo "Open feedback_disconn_regs.png in any image viewer"
read -n 1 -s -r -p "Press any key to continue"

# You can see that there was no logic between the registers. So they are basically completely separated from the graph.

# However, if we use a more minimal feedback edge set, we can isolate the critical feedback edge and keep the rest of the graph intact.
nl_opt feedback.v --passes disconnect-arc-set,clean-vis,dot-graph | dot -Tpng > feedback_disconn_arc_set.png
echo "Open feedback_disconn_arc_set.png in any image viewer"
read -n 1 -s -r -p "Press any key to continue"

# You can see the consequences of this technique in action with eqmap

# Optimize reg2reg paths individually (--no retime)
eqmap_fpga feedback.v --no-retime -k 3 | nl_opt --passes clean-vis,dot-graph | dot -Tpng > r2r.png
echo "Open r2r.png in any image viewer"
read -n 1 -s -r -p "Press any key to continue"

# Optimize with only feedback edge remapped
eqmap_fpga feedback.v -k 3 | nl_opt --passes clean-vis,dot-graph | dot -Tpng > arc.png
echo "Open arc.png in any image viewer"
read -n 1 -s -r -p "Press any key to continue"

# You can see how much better the optimization result is when the graph is left more intact
# The entire register chains are seen as structurally equivalent. And then the AND3 can optimize away.

echo "Done!"