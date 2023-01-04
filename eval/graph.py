import networkx as nx
from networkx.readwrite import json_graph
from argparse import ArgumentParser


def get_greatest_scc(graph):
    """removes all nodes that are not in the largest strongly connected component"""
    largest = max(nx.strongly_connected_components(graph), key=len)
    print("Removed {} nodes.".format(graph.number_of_nodes()))
    return largest


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
    remove_invalid_edges(graph)
    remove_no_data_nodes(graph)
    get_greatest_scc(graph)
    print(">>>")
    return graph


def read_graph(file):
    return nx.read_gml(file)


if __name__ == "__main__":
    parser = ArgumentParser()
    parser.add_argument(
        "-i",
        "--input",
        dest="graph_file",
        help="Path to the graph files.",
        required=True,
    )

    args = parser.parse_args()
    graph_file = args.graph_file
    graph = read_graph(graph_file)
    print(graph)
    graph = clean(graph)
