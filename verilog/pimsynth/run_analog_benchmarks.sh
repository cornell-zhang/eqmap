#!/bin/bash

source ../utils/setup.sh

pimpath="/zhang/PIMsynth_fork"
benchmarks=$(ls benchmarks/*_int16*.v)
submodule_list=$(ls submodules/*.v)

for benchmark_path in $benchmarks; do
    benchmark_file=$(basename "$benchmark_path")
    benchmark_name="${benchmark_file%.v}"

    echo "Processing $benchmark_name..."

    # Step 1: PIMsynth generation (to C)
    python3 "$pimpath/bit_serial_compiler.py" --verilog $submodule_list "$benchmark_path" --genlib $pimpath/src-genlib/inv_maj_and_or.genlib --num-regs 6 --output "${benchmark_name}_pimsynth" --to-stage c --pim-mode analog

    # Step 2: Eqmap mapping
    pim "$benchmark_path" "${benchmark_name}_eqmap.v" --no-assert --min-depth -s 100000 -t 10 -n 10000000

    # Step 3: Verilog to BLIF using yosys
    yosys -p "read_verilog ${benchmark_name}_eqmap.v; write_blif -gates ${benchmark_name}_eqmap.blif"

    # Step 4: Format conversion
    python3 convert_format.py "${benchmark_name}_eqmap.blif" "${benchmark_name}_converted.blif"

    # Step 5: Final compilation (to C)
    python3 "$pimpath/bit_serial_compiler.py" --blif "${benchmark_name}_converted.blif" --num-regs 6 --output "${benchmark_name}_eqmap" --from-stage blif --to-stage c --pim-mode analog
done

# Step 6: Run Python script for analysis
python3 gen_pim_histogram.py

# Step 7: Cleanup
rm -f *.v *.abc *.run_abc.sh *.run_yosys.sh *.run_blif2c.sh *.yosys *.yosys.log *.blif
