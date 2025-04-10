#!/bin/zsh
PWDL=$(pwd)
IDIR=$(realpath $(dirname ${(%):-%x})/..)
cd $IDIR

if cargo build --release --features default,$1,$2,$3,$4; then

    chmod +x ./bin/*
    chmod +x ./utils/*.py
    export PATH=$IDIR/target/release:$IDIR/bin:$IDIR/utils:$PATH
    cd $PWDL
    echo "SUCCESS"
else
    cd $PWDL
    echo "FAILURE"
fi
