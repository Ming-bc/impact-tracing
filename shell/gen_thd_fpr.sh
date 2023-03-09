#!/bin/bash

ul="_"
suffix=".txt"

thd_fpr_name="output/thd_fpr_fix_step/thd_fpr"
inf_dist_name="output/inf_dist/k_shell"
inf_fpr_name="output/inf_fpr_dist/inf_fpr"
fuz_fpr_name="output/fuz_fpr/fuz_fpr"

thd_fpr_dir=$thd_fpr_name$suffix
inf_dist_dir=$inf_dist_name$suffix
inf_fpr_dir=$inf_fpr_name$suffix
fuz_fpr_dir=$fuz_fpr_name$suffix


for ((i=1; i<=20; i++))
    do
        cd ../
        cargo test test_fuzz_ours

        cd ../Traceability-Evaluation/src/
        python3 raw_csv_gen.py

        cd ../../Efficient-Traceback-for-EEMS

        cargo test test_thd_fpr
        cargo test test_inf_dist
        cargo test test_inf_fpr
        cargo test test_fuz_fpr

        extend_thd_fpr_dir=$thd_fpr_name$ul$i$suffix
        mv $thd_fpr_dir $extend_thd_fpr_dir

        extend_inf_dist_dir=$inf_dist_name$ul$i$suffix
        mv $inf_dist_dir $extend_inf_dist_dir

        extend_inf_fpr_dir=$inf_fpr_name$ul$i$suffix
        mv $inf_fpr_dir $extend_inf_fpr_dir

        extend_fuz_fpr_dir=$fuz_fpr_name$ul$i$suffix
        mv $fuz_fpr_dir $extend_fuz_fpr_dir

        cd src/
    done

