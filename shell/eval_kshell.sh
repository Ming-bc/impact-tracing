#!/bin/bash

cd ../

for ((i=1; i<=10; i++))
    do
        cargo test test_fuzz_ours
        python3 ../../Traceability-Evaluation/src/comp_ks.py
    done