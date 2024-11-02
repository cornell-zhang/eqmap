import argparse
import sys
import os
import json

# cargo llvm-cov --all-features --workspace --json
if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "-p",
        "--percent",
        dest="percent",
        required=False,
        help="Minimum code coverage per line per source file",
        type=float,
        default=80.0,
    )
    parser.add_argument(
        "-w",
        "--whitelist",
        dest="whitelist",
        nargs="+",
        help="Files to whitelist",
        type=str,
        default=["main.rs"],
    )
    parser.add_argument(
        "input", nargs="?", type=argparse.FileType("r"), default=sys.stdin
    )
    parser.add_argument(
        "output", nargs="?", type=argparse.FileType("w"), default=sys.stdout
    )
    args = parser.parse_args()

    whitelisted = set(args.whitelist)
    data = json.load(args.input)
    data = data["data"]
    percent = args.percent
    passed = True
    for datum in data:
        files = datum["files"]
        for record in files:
            filePassed = True
            name = record["filename"]
            stem = os.path.basename(name)
            if stem in whitelisted:
                continue
            lineCoverage = record["summary"]["lines"]["percent"]
            if lineCoverage < percent:
                print(f"{name}: Only {lineCoverage:.2f}% by line", file=args.output)
                passed = False
                with open(name, "r") as f:
                    lines = f.readlines()
                    printed = set()
                    for segment in record["segments"]:
                        line = segment[0] - 1
                        executed = segment[2] != 0
                        if not executed:
                            txt = lines[line - 1].strip("\n")
                            content = txt.strip()
                            if (
                                line not in printed
                                and len(content) > 3
                                and not content.startswith("//")
                                and not content.startswith("}")
                                and not content.startswith("#")
                            ):
                                print(
                                    f"({lineCoverage:.2f}%) {stem}:{line}: {txt}",
                                    file=args.output,
                                )
                                printed.add(line)

    sys.exit(0 if passed else 1)
