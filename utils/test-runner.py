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
import os
import subprocess


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "directory", type=str, default=None, help="root directory containing tests"
    )
    parser.add_argument("-j", "--jobs", type=int, default=4, help="number of jobs")
    args = parser.parse_args()

    if not os.path.exists(args.directory) or not os.path.isdir(args.directory):
        print("Invalid directory")
        sys.exit(1)

    job = list()  # (root, file, cmd)
    jobs = list()  # list of lists

    directory = args.directory.strip().rstrip("/")
    for root, _dirs, files in os.walk(directory):
        for file in files:
            path = os.path.join(root, file)
            with open(path, "r") as f:
                lines = f.readlines()
                cmd = lines[0].strip()
                if not cmd.startswith("// RUN:"):
                    continue
                cmd = cmd.removeprefix("// RUN:").strip().replace("%s", path)
                if len(job) < args.jobs:
                    job.append((root, file, cmd))
                else:
                    jobs.append(job)
                    job = [(root, file, cmd)]

    jobs.append(job)

    tests = list()
    passes = list()
    failures = list()

    lastLen = 0
    for job in jobs:
        processes = list()
        for root, file, cmd in job:
            p = subprocess.Popen(cmd, shell=True, stderr=subprocess.PIPE)
            processes.append((root, file, p, cmd))
        local = list()
        for root, file, p, cmd in processes:
            p.wait()
            if p.returncode == 0:
                passes.append((root, file))
            else:
                reason = p.stderr.read().decode("utf-8")
                failures.append((root, file, cmd, reason))
            tests.append((root, file))
            local.append(file)

        last = local.pop()
        llen = 2
        print("  ", end="", file=sys.stderr)
        for r in local:
            l = f"{r} \N{HEAVY CHECK MARK}  "
            llen += len(l)
            print(l, end="", file=sys.stderr)
        l = f"{last} \N{HEAVY CHECK MARK}  "
        llen += len(l)
        if llen < lastLen:
            l += " " * (lastLen - llen)
        print(l, end="\r", file=sys.stderr)
        lastLen = llen

    print("\r\n", file=sys.stderr)
    for root, file, cmd, reason in failures:
        print(f"FAIL: {root}/{file}", file=sys.stderr)
        print(f"{cmd}", file=sys.stderr)
        print("=" * 80, file=sys.stderr)
        print(reason, file=sys.stderr)

    print(f"Ran:      {len(tests)} tests")
    print(f"Passes:   {len(passes)}")
    print(f"Failures: {len(failures)}")

    if len(failures) > 0:
        sys.exit(1)
    else:
        sys.exit(0)
