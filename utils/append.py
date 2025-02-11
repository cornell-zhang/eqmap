#!/bin/env python3
import argparse
import sys
import json

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "inputs", nargs="*", type=argparse.FileType("r"), default=[sys.stdin]
    )
    parser.add_argument("--version", type=str, default="UnknownAppended")
    args = parser.parse_args()

    modules = {}
    version = None
    mergedRpt = dict()
    for inputData in args.inputs:
        data = json.load(inputData)
        m = data["modules"]

        for k in m:
            if k in modules:
                print(f"Duplicate module found {k}", file=sys.stderr)
                sys.exit(1)
            else:
                modules[k] = m[k]

        if version is None and "version" in data:
            version = data["version"]
        elif "version" in data and version != data["version"]:
            print(f"Version mismatch {version} != {data['version']}", file=sys.stderr)
            sys.exit(1)

    mergedRpt["modules"] = modules
    mergedRpt["version"] = version

    json.dump(mergedRpt, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
