import numpy as np
from glob import glob
import os

def cal_mean(file_dir):
    matrixs = []
    file_paths = glob(os.path.join(file_dir, "*.txt"))
    for file in file_paths:
        f = open(file, 'r')
        content = f.readlines()
        
        matrix = []
        for line in content:
            line = line[1:-2]
            line = [float(num) for num in line.split(', ')]
            matrix.append(line)
        np.array(matrix, dtype=np.float32)

        matrixs.append(matrix)
    return np.mean(matrixs, axis=0)

# output a matrix as csv file
def output_csv(matrix, file_name):
    with open(file_name, 'w') as f:
        for line in matrix:
            for num in line:
                f.write(str(num) + ',')
            f.write('\n')

output_csv(cal_mean("output/fuz_fpr"), "python/outputs/priv.csv")
output_csv(cal_mean("output/thd_fpr_fix_step"), "python/outputs/correct.csv")
output_csv(cal_mean("output/inf_dist"), "python/outputs/influence.csv")
output_csv(cal_mean("output/inf_detect"), "python/outputs/detect.csv")