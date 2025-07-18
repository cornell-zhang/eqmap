#!/bin/bash

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

FORMAT="real %e s\nuser %U s\nsys %S s\nmem %M KB"
FILE="time.txt"
TIME_ARGS="-f $FORMAT --append -o $FILE"
MAIN_ARGS="tests/lutlang/hard_examples.txt -k 4 -t 60 -s 15000"
set -exo pipefail

# Collect timing information
echo "Timing information" > $FILE
echo "==================" >> $FILE
echo "debug, no proof" >> $FILE
/bin/time -f "$FORMAT" --append -o $FILE cargo run $MAIN_ARGS 2>>/dev/null
echo "==================" >> $FILE
echo "debug, proof" >> $FILE
/bin/time -f "$FORMAT" --append -o $FILE cargo run $MAIN_ARGS --verbose 2>>/dev/null
echo "==================" >> $FILE
echo "release, no proof" >> $FILE
/bin/time -f "$FORMAT" --append -o $FILE cargo run --release $MAIN_ARGS 2>>/dev/null
echo "==================" >> $FILE
echo "release, proof" >> $FILE
/bin/time -f "$FORMAT" --append -o $FILE cargo run --release $MAIN_ARGS --verbose 2>>/dev/null
echo "==================" >> $FILE
cat $FILE
echo "SUCCESS"
