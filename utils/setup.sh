#!/bin/bash
PWDL=$(pwd)
IDIR=$(realpath $(dirname $BASH_SOURCE)/..)
cd $IDIR
cargo build --release --features default,$1,$2,$3,$4

chmod +x ./bin/*
chmod +x ./utils/*.py
export PATH=$IDIR/target/release:$IDIR/bin:$IDIR/utils:$PATH
cd $PWDL
echo "SUCCESS"
