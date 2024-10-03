import networkx as nx
import pydot
import numpy as np

DOT_DIR_MSG = "../graphs/msg.dot"
DOT_DIR_EMAIL = "../graphs/email.dot"
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
