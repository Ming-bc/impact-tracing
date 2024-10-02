from collections import defaultdict
from influence import file_update, import_node_index
import networkx as nx
import shutil
import csv

DIR_FUZ_VAL = "python/inputs/data_fuzzy_value.txt"
DIR_NODE_INDEX = "python/inputs/id_map_fwd.txt"
DOT_DIR_FWD_GRAPH = "python/graphs/graph_real.dot"
DOT_DIR_FWD_UNGRAPH = "python/graphs/graph_real_undirected.dot"

def import_fuz_val_raw(in_dir):
    fuz_val = defaultdict(list)
    with open(in_dir, "r", encoding="utf-8") as f:
        for line in f:
            line = line.replace("\n", "").split(',')
            fuz_val[line[0]] = [line[1],line[2]]
    return fuz_val

def insert_with_check(item, val_dict: dict):
    if item in val_dict:
        return val_dict[item]
    else:
        return 0
    
# 1. Generate influences
# import graph from .dot file
g_directed = nx.nx_agraph.read_dot(DOT_DIR_FWD_GRAPH)

# import id_map from .txt file
node_wt_dict = import_node_index(DIR_NODE_INDEX)

# (3) K-shell
shutil.copyfile(DOT_DIR_FWD_GRAPH, DOT_DIR_FWD_UNGRAPH)
file_update(DOT_DIR_FWD_UNGRAPH)
g_undirected = nx.nx_agraph.read_dot(DOT_DIR_FWD_UNGRAPH)

k_shell_dict = {}
shell_node_count = 1
shell_index = 0
while shell_node_count != 0:
    shell_index += 1
    k_g = nx.k_shell(g_undirected.to_undirected(), shell_index)
    shell_node_count = len(k_g.nodes)

max_shell_index = shell_index - 1
for i in range(1, max_shell_index+1):
    for n in nx.k_shell(g_undirected.to_undirected(), i).nodes:
        k_shell_dict[node_wt_dict[n]] = i

# 2. Import fuzzy value
id_itp_val = import_fuz_val_raw(DIR_FUZ_VAL)

# 3. Integrate fuzzy value and influence
for id in id_itp_val:
    val_k_shell = insert_with_check(id, k_shell_dict)
    id_itp_val[id].append(val_k_shell)

# 4. Write into a .csv file
header = ['id', 'is_ture_positve', 'fuzzy_value', 'k-shell']

with open('python/inputs/fuz_val_and_inf.csv', 'w+', encoding='UTF8', newline='') as f:
    writer = csv.writer(f)
    writer.writerow(header)
    
    for id in id_itp_val:
        w_list = id_itp_val[id]
        w_list.insert(0, id)
        writer.writerow(w_list)