#!/usr/bin/env python3

from pathlib import Path
import networkx as nx
from argparse import ArgumentParser
import os

"""
Reads a graph from a JSON file, calculates centrality scores and writes to a file.
"""


def read_graph(path):
    print("Reading graph from {}".format(path))
    G = nx.read_gml(path)
    G = clean(G)
    return G


def get_greatest_scc(graph):
    """removes all nodes that are not in the largest strongly connected component"""
    largest = max(nx.strongly_connected_components(graph), key=len)
    to_be_removed = set(graph.nodes) - largest
    graph.remove_nodes_from(to_be_removed)
    print(
        "Removed {} nodes by keeping largest strongly connected component".format(
            len(to_be_removed)
        )
    )
    return graph


def remove_no_data_nodes(graph):
    """
    removes nodes from the graph that do not have any associated node data
    for easier data evaluation
    """
    nodes = list(graph.nodes(data="id"))
    count = 0
    for node, identifier in nodes:
        if identifier is None:
            graph.remove_node(node)
            count += 1
    print("Removed {} node(s) without node data from the graph".format(count))
    return graph


def remove_invalid_edges(graph):
    """
    removes edges that do not specify a htlc_maximum_msat value
    """
    count = 0
    for edge in list(graph.edges(data=True)):
        if "htlc_maximum_msat" not in edge[2].keys():
            graph.remove_edge(*edge[:2])
            count += 1
    print(
        "Removed {} edge(s) from the graph without set htlx_maximum_msat value".format(
            count
        )
    )
    return graph


def clean(graph):
    print("start cleaning graph")
    G = get_greatest_scc(graph)
    return G


def betweenness_centrality(graph, output_dir):
    print("Computing betweenness centrality.")
    output_path = os.path.join(output_dir, "betweenness_centrality.txt")
    betweenness = nx.betweenness_centrality(graph)
    betweenness_sorted = dict(
        sorted(betweenness.items(), key=lambda item: item[1], reverse=True)
    )
    write(betweenness_sorted, output_path)
    print("Successfully written to {}".format(output_path))


def degree_centrality(graph, output_dir):
    print("Computing degree centrality.")
    output_path = os.path.join(output_dir, "degree_centrality.txt")
    degree_centrality = nx.degree_centrality(graph)
    degree_centrality_sorted = dict(
        sorted(degree_centrality.items(), key=lambda item: item[1], reverse=True)
    )
    write(degree_centrality_sorted, output_path)
    print("Successfully written to {}".format(output_path))


def write(scores, output_path):
    with open(output_path, "w") as f:
        for node in scores:
            f.write("%s\n" % node)


if __name__ == "__main__":
    parser = ArgumentParser()
    parser.add_argument(
        "-i",
        "--input",
        help="Path to graphml file describing the graph.",
        type=Path,
        default="../data/gossip-20210906-1000UTC.gml",
    )
    parser.add_argument(
        "-o",
        "--output",
        dest="output_dir",
        help="Path to the directory where result files should be stored.",
        default="./",
    )
    args = parser.parse_args()
    path_to_file = args.input
    output_path = args.output_dir
    G = read_graph(path=path_to_file)
    print(
        "Got graph with {} nodes and {} edges.".format(
            G.number_of_nodes(), G.number_of_edges()
        )
    )
    betweenness_centrality(G, output_path)
    degree_centrality(G, output_path)
