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
        cd ../
        cargo test test_fuzz_ours

        cd ../Traceability-Evaluation/src/
        python3 raw_csv_gen.py

        cd ../../Efficient-Traceback-for-EEMS

        cargo test gen_graph_csv

        extend_thd_fpr_dir=$thd_fpr_name$ul$i$suffix
        mv $thd_fpr_dir $extend_thd_fpr_dir

        extend_inf_dist_dir=$inf_dist_name$ul$i$suffix
        mv $inf_dist_dir $extend_inf_dist_dir

        extend_inf_detect_dir=$inf_detect_name$ul$i$suffix
        mv $inf_detect_dir $extend_inf_detect_dir

        extend_fuz_fpr_dir=$fuz_fpr_name$ul$i$suffix
        mv $fuz_fpr_dir $extend_fuz_fpr_dir

        cd src/
    done

cd ../../Traceability-Evaluation/src/draw/
python3 thd_fpr.py
python3 inf_dist.py
python3 fuz_fpr.py
python3 traceability_thd.py

cd ../../outputs/
cp -f inf_fpr_dist.csv trace_fpr/detect.csv
cp -f fuz_fpr.csv privacy/priv.csv
cp -f thd_fpr_fix_step.csv correctness/correct.csv

cd ../../Efficient-Traceback-for-EEMS/output/
# rm -f thd_fpr_fix_step/*
# rm -f inf_dist/*
# rm -f inf_detect_dist/*
# rm -f fuz_fpr/*
