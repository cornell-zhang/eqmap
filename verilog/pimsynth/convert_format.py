#!/usr/bin/env python3
"""
BLIF Format Converter
Converts BLIF files from one gate format to another.
"""

import re
import sys


def convert_line(line, const_counter=None):
    """
    Convert a single line according to the specified rules.
    Returns the converted line or None if the line should be removed.
    If const_counter is provided, it should be a list with one element [counter] to maintain state.
    """
    stripped = line.rstrip("\n")

    # Rule 1: Convert ".names $false [name]" to ".gate zero O=[name]"
    match = re.match(r"^\.names\s+\$false\s+(\S+)$", stripped)
    if match:
        return f".gate zero O={match.group(1)}\n"

    # Rule 2: Convert ".names $true [name]" to ".gate one O=[name]"
    match = re.match(r"^\.names\s+\$true\s+(\S+)$", stripped)
    if match:
        return f".gate one O={match.group(1)}\n"

    # Rule 3: Convert ".names [name 1] [name 2]" to ".gate copy a=[name 1] O=[name 2]"
    match = re.match(r"^\.names\s+(\S+)\s+(\S+)$", stripped)
    if match:
        return f".gate copy a={match.group(1)} O={match.group(2)}\n"

    # Rule 4: Remove lines starting with "1" or ".names"
    if stripped.startswith("1") or stripped.startswith(".names"):
        return None

    # Rule 5: Convert INV_X1 format to inv1 format
    match = re.match(r"^\.gate\s+INV_X1\s+A=(\S+)\s+ZN=(\S+)$", stripped)
    if match:
        return f".gate inv1 a={match.group(1)} O={match.group(2)}\n"

    # New Rule: Convert MAJ3_X1 with any combination of $true/$false in A1/A2/A3
    match = re.match(
        r"^\.gate\s+MAJ3_X1\s+A1=(\S+)\s+A2=(\S+)\s+A3=(\S+)\s+ZN=(\S+)$", stripped
    )
    if match:
        a1, a2, a3, zn = match.groups()
        new_lines = []
        const_assignments = {"A1": a1, "A2": a2, "A3": a3}

        # Check each input for $true/$false and replace with const_x
        for port, value in const_assignments.items():
            if value in ["$true", "$false"]:
                if const_counter is not None:
                    const_num = const_counter[0]
                    const_counter[0] += 1
                    const_gate = "one" if value == "$true" else "zero"
                    new_lines.append(f".gate {const_gate} O=const_{const_num}\n")
                    const_assignments[port] = f"const_{const_num}"

        # Create the modified maj3 gate (converted from MAJ3_X1)
        new_lines.append(
            f".gate maj3 a={const_assignments['A1']} b={const_assignments['A2']} c={const_assignments['A3']} O={zn}\n"
        )

        # If we made changes, return the new lines, otherwise fall through to regular conversion
        # if len(new_lines) > 1:  # More than just the original gate line
        return new_lines

    # Rule 7: Convert AND2_X1 format to and2 format
    match = re.match(r"^\.gate\s+AND2_X1\s+A1=(\S+)\s+A2=(\S+)\s+ZN=(\S+)$", stripped)
    if match:
        return f".gate and2 a={match.group(1)} b={match.group(2)} O={match.group(3)}\n"

    # Rule 8: Convert OR2_X1 format to or2 format
    match = re.match(r"^\.gate\s+OR2_X1\s+A1=(\S+)\s+A2=(\S+)\s+ZN=(\S+)$", stripped)
    if match:
        return f".gate or2 a={match.group(1)} b={match.group(2)} O={match.group(3)}\n"

    # Rule 9: Convert OR_X1 format to or2 format
    match = re.match(r"^\.gate\s+OR_X1\s+A=(\S+)\s+B=(\S+)\s+Y=(\S+)$", stripped)
    if match:
        return f".gate or2 a={match.group(1)} b={match.group(2)} O={match.group(3)}\n"

    # Rule 10: Convert AND_X1 format to and2 format
    match = re.match(r"^\.gate\s+AND_X1\s+A=(\S+)\s+B=(\S+)\s+Y=(\S+)$", stripped)
    if match:
        return f".gate and2 a={match.group(1)} b={match.group(2)} O={match.group(3)}\n"

    # Keep all other lines unchanged
    return line


def convert_file(input_path, output_path):
    """
    Convert the entire file from input_path to output_path.
    """
    try:
        const_counter = [0]  # Using list to maintain mutable state across calls
        with open(input_path, "r") as infile:
            with open(output_path, "w") as outfile:
                for line in infile:
                    converted = convert_line(line, const_counter)
                    if converted is not None:
                        if isinstance(converted, list):
                            # Handle multiple lines returned as list
                            for converted_line in converted:
                                outfile.write(converted_line)
                        else:
                            # Handle single line
                            outfile.write(converted)
        print(f"Successfully converted {input_path} to {output_path}")
    except FileNotFoundError:
        print(f"Error: Input file '{input_path}' not found.", file=sys.stderr)
        sys.exit(1)
    except IOError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def main():
    if len(sys.argv) != 3:
        print(
            "Usage: python convert_format.py <input_file> <output_file>",
            file=sys.stderr,
        )
        sys.exit(1)

    input_path = sys.argv[1]
    output_path = sys.argv[2]

    convert_file(input_path, output_path)


if __name__ == "__main__":
    main()
