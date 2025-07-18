#!/bin/env python3

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
