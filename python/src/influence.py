import networkx as nx
import pydot
import numpy as np

DOT_DIR_MSG = "../graphs/msg.dot"
DOT_DIR_EMAIL = "../graphs/email.dot"
# DOT_DIR_FWD_GRAPH = "../../Efficient-Traceback-for-EEMS/output/fwd_graph.dot"
DOT_DIR_FWD_UNGRAPH = "../graphs/fwd_graph.dot"
DIR_FUZZY_VALUE = "../graphs/leveled_fuz_val.txt"
DIR_NODE_INDEX = "../graphs/real2graph.txt"

def degree_analysis(graph):
    edge_list = []
    for i in range(0,191):
        degree = nx.degree(graph, str(i))
        if isinstance(degree, nx.classes.reportviews.DegreeView): 
            continue
        else: edge_list.append(degree)
    print("max: %d, median: %d, mean: %.1f" %(np.max(edge_list), np.median(edge_list), np.mean(edge_list)))

def str_replace(in_dir, old_str, new_str):
    file_data = ""
    with open(in_dir, "r", encoding="utf-8") as f:
        for line in f:
            if old_str in line:
                line = line.replace(old_str, new_str)
            file_data += line
    with open(in_dir,"w",encoding="utf-8") as f:
        f.write(file_data)

def file_update(in_dir):
    str_replace(in_dir, " [ ]", ";")
    str_replace(in_dir, "->", "--")
    str_replace(in_dir, "digraph", "strict graph")

def dot_to_svg(dir):
    (graph,) = pydot.graph_from_dot_file(dir)
    graph.write_svg("example.svg")

def import_fuzzy_value(in_dir):
    fuzzy_value_dict = {}
    with open(in_dir, "r", encoding="utf-8") as f:
        for line in f:
            line = line.replace("\n", "").split(',')
            fuzzy_value_dict[line.pop(0)] = line.pop()
    return fuzzy_value_dict

def import_node_index(in_dir):
    node_index_dict = {}
    with open(in_dir, "r", encoding="utf-8") as f:
        for line in f:
            line = line.replace("\n", "").split(',')
            node_index_dict[line.pop(0)] = line.pop()
    return node_index_dict

# read A and B from a line with the form "A [ label = "B" ]"

# file_update(DOT_DIR_FWD_UNGRAPH)

# G = nx.nx_agraph.read_dot(DOT_DIR_FWD_UNGRAPH)

# # dot_to_svg(DOT_DIR_FWD_UNGRAPH)

# degree_analysis(G)

# # 1. find max k-shell nodes
# shell_size = 1
# shell_index = 1
# while shell_size != 0:
#     k_g = nx.k_shell(G.to_undirected(), shell_index)
#     shell_size = len(k_g.nodes)
#     shell_index += 1

# # 2. import fuzzy value
# max_influence_nodes = nx.k_shell(G.to_undirected(), shell_index - 2).nodes
# fuz_val_dict = import_fuzzy_value(DIR_FUZZY_VALUE)

# # 3. compare fuzzy value and influence
# node_weight_dict = import_node_index(DIR_NODE_INDEX)
# #   fuzzy value / influence
# fuz_in_inf_4 = 0
# fuz_in_inf_3 = 0
# inf_in_fuz = 0
# for n in max_influence_nodes:
#     weight = node_weight_dict[n]
#     # print("fwd %s, sys %s, level %s" %(n, index, fuz_val_dict[index]))
#     if weight in fuz_val_dict:
#         if fuz_val_dict[weight] == "4":
#             fuz_in_inf_4 += 1
#         elif fuz_val_dict[weight] == "3":
#             fuz_in_inf_3 += 1
#     else:
#         fuz_in_inf_4 += 1

# print("K-shell %d contains %d, we find %d + %d" %(shell_index - 2, len(max_influence_nodes), fuz_in_inf_4, fuz_in_inf_3))
