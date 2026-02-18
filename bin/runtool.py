#!/usr/bin/env python3

import os
import subprocess
import sys
from pathlib import Path


def get_script_dir():
    return Path(__file__).parent.resolve()


def get_lib_paths():
    script_dir = get_script_dir()
    lutlang_lib = script_dir.parent / "verilog" / "lutlang.v"
    celllang_lib = script_dir.parent / "verilog" / "celllang.v"
    simlib = script_dir.parent / "verilog" / "simlib.v"
    return {
        "lutlang": str(lutlang_lib.resolve()),
        "celllang": str(celllang_lib.resolve()),
        "simlib": str(simlib.resolve()),
    }


def check_dependencies(tool_name):
    """Check if required tools are available"""
    try:
        subprocess.run(["which", "yosys"], check=True, stdout=subprocess.DEVNULL)
    except subprocess.CalledProcessError:
        print("yosys not found in PATH")
        sys.exit(1)

    try:
        subprocess.run(["which", tool_name], check=True, stdout=subprocess.DEVNULL)
    except subprocess.CalledProcessError:
        print(f"{tool_name} not found in PATH")
        sys.exit(1)


def run_equiv_sh(input_file, output_file):
    """Run equiv.sh script to check equivalence"""
    try:
        # Get the directory where equiv.sh should be located
        script_dir = get_script_dir()
        equiv_sh_path = script_dir.parent / "verilog" / "equiv.sh"

        # Ensure we're running from the directory containing the files
        cwd = os.getcwd()
        input_path = Path(input_file).resolve()
        output_path = Path(output_file).resolve()

        # Run equiv.sh from the directory containing the files
        result = subprocess.run(
            [str(equiv_sh_path), str(input_path), str(output_path)], cwd=cwd
        )
        if result.returncode != 0:
            print(f"equiv.sh failed with exit code {result.returncode}")
        return result.returncode
    except FileNotFoundError:
        print("equiv.sh not found in expected location")
        return 1
    except Exception as e:
        print(f"Error running equiv.sh: {e}")
        return 1


def generate_makefile_content(input_file, lib_path, tool_name, synth_target):
    """Generate Makefile content for synthesis"""
    content = [
        "SRCS=$(wildcard *.v)",
        "# Set both Yosys and Vivado to use Ultrascale+ Arch",
        "FAMILY=xcup",
        "PART=xczu3eg-sbva484-1-i",
        "# flatten design before synthesis, no clock buffers, no IO buffers, no carry logic, no MUXes",
        "SYNTH_OPT=-flatten -noclkbuf -noiopad -nocarry -nowidelut -nosrl -ise",
        "YOSYS=yosys # Yosys 0.33 (git sha1 2584903a060)",
        "XILINX_VIVADO?=$(realpath $(dirname $(which vivado))/..)",
        "VIVADO=$(XILINX_VIVADO)/bin/vivado",
        "",
        ".PHONY: all clean",
        "",
        f"all: {input_file}.{synth_target}",
        "",
        "clean:",
        "\trm -f *.xil *.synth *.ys *.dot *.png",
        "",
    ]

    # Add synthesis rules
    content.extend(
        [
            "%.v.synth: %_synth.ys",
            "\t+$(YOSYS) -s $<" + (" >> /dev/null" if "msynth" in sys.argv[0] else ""),
            "",
            "%.v.yxil: %_yxil.ys",
            "\t+$(YOSYS) -s $<",
            "",
            "%.v.vxil: %_vxil.tcl",
            "\t+$(VIVADO) -mode tcl -source $< -nolog -nojournal",
            "",
            "%.v.inline: %_inline.ys",
            "\t+$(YOSYS) -s $<",
            "",
            "# This script synthesizes to LUTs",
            "%_yxil.ys: %.v",
            '\t@echo "read_verilog $<" > $@',
            '\t@echo "synth_xilinx -family $(FAMILY) $(SYNTH_OPT)" >> $@',
            '\t@echo "clean -purge" >> $@',
            '\t@echo "splitnets -ports -format _" >> $@'
            if "resynth" not in sys.argv[0]
            else '\t@echo "splitnets -format _" >> $@',
            '\t@echo "write_verilog -simple-lhs $<.yxil" >> $@',
            "",
            "# This script synthesizes to AND, NOR, XOR, INV, MUXes",
            "%_synth.ys: %.v",
            '\t@echo "read_verilog $<" > $@',
            f'\t@echo "techmap -map {lib_path}" >> $@',
            '\t@echo "clean -purge" >> $@',
            '\t@echo "splitnets -format _" >> $@'
            if "msynth" not in sys.argv[0]
            else '\t@echo "splitnets -ports -format _" >> $@',
            '\t@echo "write_verilog -simple-lhs $<.synth" >> $@',
            "",
            "%_vxil.tcl: %.v",
            '\t@echo "add_files $<" > $@',
            '\t@echo "synth_design -top $* -mode out_of_context -part $(PART)" >> $@',
            '\t@echo "write_verilog -force $<.vxil" >> $@',
            '\t@echo "quit" >> $@',
            "",
            "%_inline.ys: %.v",
            f'\t@echo "read_verilog {get_lib_paths()["simlib"]}" > $@',
            '\t@echo "read_verilog $<" >> $@',
            '\t@echo "flatten" >> $@',
            '\t@echo "clean -purge" >> $@',
            '\t@echo "write_verilog -simple-lhs -noattr $<.inline" >> $@',
        ]
    )

    return "\n".join(content)


def run_eqmap(
    input_file, output_file, *args, synth_target="yxil", tool_name="eqmap_fpga"
):
    """Main eqmap functionality"""
    libs = get_lib_paths()
    lib_path = libs["lutlang"] if tool_name == "eqmap_fpga" else libs["celllang"]

    if not os.path.isfile(input_file):
        print("First argument must be the input file.")
        print(f"Usage: {sys.argv[0]} <input.v> <output.v> [options]")
        print("Check that file exists and it is the first argument")
        sys.exit(1)

    check_dependencies(tool_name)

    mkfile_path = f"{input_file}.mk"

    # Generate Makefile
    makefile_content = generate_makefile_content(
        input_file, lib_path, tool_name, synth_target
    )
    with open(mkfile_path, "w") as f:
        f.write(makefile_content)

    # Run make
    try:
        subprocess.run(
            ["make", "-f", mkfile_path, f"{input_file}.{synth_target}"],
            stderr=subprocess.STDOUT,
        )
    except subprocess.CalledProcessError as e:
        print(f"Make failed with exit code {e.returncode}")
        sys.exit(1)

    # Run the tool with output file parameter
    tool_args = [tool_name, f"{input_file}.{synth_target}", output_file] + list(args)
    try:
        result = subprocess.run(tool_args)
        if result.returncode != 0:
            print(f"Tool {tool_name} failed with exit code {result.returncode}")
            # Cleanup before exiting
            try:
                os.remove(f"{input_file}.{synth_target}")
                os.remove(mkfile_path)
            except OSError:
                pass
            sys.exit(result.returncode)
    except FileNotFoundError:
        print(f"Tool {tool_name} not found in PATH")
        # Cleanup before exiting
        try:
            os.remove(f"{input_file}.{synth_target}")
            os.remove(mkfile_path)
        except OSError:
            pass
        sys.exit(1)

    # Run equiv.sh script
    equiv_return_code = run_equiv_sh(input_file, output_file)

    # Cleanup
    try:
        os.remove(f"{input_file}.{synth_target}")
        os.remove(mkfile_path)
    except OSError:
        pass

    # Exit with equiv.sh return code if it failed
    if equiv_return_code != 0:
        sys.exit(equiv_return_code)


def run_eqmap_vivado(input_file, output_file, *args):
    """Run eqmap with Vivado backend"""
    run_eqmap(
        input_file, output_file, *args, synth_target="vxil", tool_name="eqmap_fpga"
    )


def run_fam(*args):
    """Direct wrapper for eqmap_fpga"""
    if len(args) < 2:
        print("Usage: fam <input.v> <output.v> [options]")
        sys.exit(1)

    try:
        result = subprocess.run(["eqmap_fpga"] + list(args))
        if result.returncode == 0:
            # Run equiv.sh if the tool succeeded
            run_equiv_sh(args[0], args[1])
        else:
            sys.exit(result.returncode)
    except FileNotFoundError:
        print("eqmap_fpga not found in PATH")
        sys.exit(1)


def run_lvv(*args):
    """Direct wrapper for eqmap"""
    if len(args) < 2:
        print("Usage: lvv <input.v> <output.v> [options]")
        sys.exit(1)

    try:
        result = subprocess.run(["runtool"] + ["eqmap"] + list(args))
        if result.returncode == 0:
            # Run equiv.sh if the tool succeeded
            run_equiv_sh(args[0], args[1])
        else:
            sys.exit(result.returncode)
    except FileNotFoundError:
        print("eqmap not found in PATH")
        sys.exit(1)


def run_lvv_vivado(*args):
    """Direct wrapper for eqmap_vivado"""
    if len(args) < 2:
        print("Usage: lvv-vivado <input.v> <output.v> [options]")
        sys.exit(1)

    try:
        result = subprocess.run(["runtool"] + ["eqmap_vivado"] + list(args))
        if result.returncode == 0:
            # Run equiv.sh if the tool succeeded
            run_equiv_sh(args[0], args[1])
        else:
            sys.exit(result.returncode)
    except FileNotFoundError:
        print("eqmap_vivado not found in PATH")
        sys.exit(1)


def run_msynth(input_file, output_file, *args):
    """ASIC mapping functionality"""
    run_eqmap(
        input_file, output_file, *args, synth_target="synth", tool_name="eqmap_asic"
    )


def run_opt_verilog(*args):
    """Optimization using parse-verilog and cargo"""
    if "--help" in args:
        try:
            subprocess.run(["cargo", "run", "--quiet", "--release", "--", "--help"])
        except subprocess.CalledProcessError:
            pass
        return

    if len(args) < 2:
        print("Usage: opt-verilog <input.v> <output.v> [options]")
        sys.exit(1)

    input_file = args[0]
    output_file = args[1]
    other_args = args[2:] if len(args) > 2 else []

    try:
        # Build release version silently
        subprocess.run(["cargo", "build", "--release"], stderr=subprocess.DEVNULL)

        # Run pipeline
        with open(output_file, "w") as f:
            parse_proc = subprocess.Popen(
                ["parse-verilog", "--", input_file], stdout=subprocess.PIPE
            )
            cargo_proc = subprocess.Popen(
                ["cargo", "run", "--quiet", "--release", "--"] + list(other_args),
                stdin=parse_proc.stdout,
                stdout=f,
            )
            if parse_proc.stdout:
                parse_proc.stdout.close()
            cargo_proc.communicate()

        # Run equiv.sh after successful completion
        run_equiv_sh(input_file, output_file)
    except FileNotFoundError as e:
        print(f"Required tool not found: {e}")
        sys.exit(1)
    except subprocess.CalledProcessError as e:
        print(f"Command failed with exit code {e.returncode}")
        sys.exit(1)


def run_resynth(input_file, output_file, *args):
    """FPGA resynthesis functionality"""
    run_eqmap(
        input_file, output_file, *args, synth_target="synth", tool_name="eqmap_fpga"
    )


def main():
    if len(sys.argv) < 2:
        print("Usage: runtool <command> [args...]")
        print(
            "Commands: eqmap, eqmap_vivado, fam, lvv, lvv-vivado, msynth, opt-verilog, resynth"
        )
        sys.exit(1)

    command = sys.argv[1]
    args = sys.argv[2:]

    # Map commands to functions
    commands = {
        "eqmap": lambda: (
            run_eqmap(*args)
            if len(args) >= 2
            else print("Usage: eqmap <input.v> <output.v> [options]")
        ),
        "eqmap_vivado": lambda: (
            run_eqmap_vivado(*args)
            if len(args) >= 2
            else print("Usage: eqmap_vivado <input.v> <output.v> [options]")
        ),
        "fam": lambda: run_fam(*args),
        "lvv": lambda: run_lvv(*args),
        "lvv-vivado": lambda: run_lvv_vivado(*args),
        "msynth": lambda: (
            run_msynth(*args)
            if len(args) >= 2
            else print("Usage: msynth <input.v> <output.v> [options]")
        ),
        "opt-verilog": lambda: run_opt_verilog(*args),
        "resynth": lambda: (
            run_resynth(*args)
            if len(args) >= 2
            else print("Usage: resynth <input.v> <output.v> [options]")
        ),
    }

    if command in commands:
        commands[command]()
    else:
        print(f"Unknown command: {command}")
        print(
            "Available commands: eqmap, eqmap_vivado, fam, lvv, lvv-vivado, msynth, opt-verilog, resynth"
        )
        sys.exit(1)


if __name__ == "__main__":
    main()
