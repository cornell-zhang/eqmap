#!/bin/env python3
import argparse
import sys
import json

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "inputs", nargs="*", type=argparse.FileType("r"), default=[sys.stdin]
    )
    parser.add_argument(
        "output", nargs="?", type=argparse.FileType("w"), default=sys.stdout
    )
    args = parser.parse_args()

    modules = {}
    version = {}
    mergedRpt = {"version": version}
    for inputData in args.inputs:
        data = json.load(inputData)
        name = None
        record = dict()
        for k, v in data.items():
            if k == "name":
                name = v.strip("\\")
            elif k == "version":
                mergedRpt[k] = v
            else:
                record[k] = v

        if name is None:
            print(f"Name of record not found", file=sys.stderr)
            sys.exit(1)

        if name in modules:
            print(f"Duplicate name found {name}", file=sys.stderr)
            sys.exit(1)

        modules[name] = record

    mergedRpt["modules"] = modules

    with args.output as f:
        json.dump(mergedRpt, args.output, indent=2, sort_keys=True)
        f.write("\n")
