#!/bin/bash

ul="_"
suffix=".txt"

inf_dist_name="output/inf_dist/k_shell"
thd_fpr_name="output/thd_fpr_fix_step/thd_fpr"
inf_detect_name="output/inf_detect/inf_detect"
fuz_fpr_name="output/fuz_fpr/fuz_fpr"

thd_fpr_dir=$thd_fpr_name$suffix
inf_dist_dir=$inf_dist_name$suffix
inf_detect_dir=$inf_detect_name$suffix
fuz_fpr_dir=$fuz_fpr_name$suffix

for ((i=1; i<=50; i++))
    do
        cargo test test_fuzz_ours
        python3 python/src/raw_csv_gen.py
        cargo test gen_graph_csv

        extend_thd_fpr_dir=$thd_fpr_name$ul$i$suffix
        mv $thd_fpr_dir $extend_thd_fpr_dir

        extend_inf_dist_dir=$inf_dist_name$ul$i$suffix
        mv $inf_dist_dir $extend_inf_dist_dir

        extend_inf_detect_dir=$inf_detect_name$ul$i$suffix
        mv $inf_detect_dir $extend_inf_detect_dir

        extend_fuz_fpr_dir=$fuz_fpr_name$ul$i$suffix
        mv $fuz_fpr_dir $extend_fuz_fpr_dir
    done

python3 python/src/calc_effect.py

rm -f output/thd_fpr_fix_step/*
rm -f output/inf_dist/*
rm -f output/inf_detect/*
rm -f output/fuz_fpr/*
