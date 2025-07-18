#!/bin/zsh

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
