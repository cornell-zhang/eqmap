#!/bin/bash
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
