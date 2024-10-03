# Experimental results
This folder includes our results on the specified metrics, where the output is formated as follows:
- **Traceability:** Each line in *detect.csv* represents (1-shell detection rate, 2-shell detection rate, ...), and the threshold increases from top to bottom. 
- **Correctness:** Each line in *correct.csv* represents (false positives in the output, output size, output FPR = (false positives / output size)), and the threshold increases from top to bottom. 
- **Privacy:** Each line in *priv.csv* represents (false positives in the range, all vertices in the range, interval FPR = (false positives / all vertices)), and the range of membership value increases from top to bottom. 