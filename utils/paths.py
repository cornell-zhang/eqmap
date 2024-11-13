import argparse
import networkx as nx
import itertools
import matplotlib.pyplot as plt


# Get the longest shortest path for every element in Sn with tranposition generators
def find_longest_path(graph):
    longest_path = []
    dest = 0
    for node in graph.nodes:
        shortest_path = [None for _ in graph.nodes]
        for path in nx.all_shortest_paths(graph, dest, node):
            if len(path) <= len(shortest_path):
                shortest_path = path
        if shortest_path[0] != None and len(shortest_path) > len(longest_path):
            longest_path = shortest_path
    return longest_path


def is_transposition(u, v):
    neq = 0
    prev_index = 0
    index = 0
    for i in range(len(u)):
        if u[i] != v[i]:
            neq += 1
            prev_index = index
            index = i
    if neq != 2:
        return False

    return (
        u[prev_index] == v[index]
        and u[index] == v[prev_index]
        and prev_index + 1 == index
    )


def get_transposition(u, v):
    neq = 0
    prev_index = 0
    index = 0
    for i in range(len(u)):
        if u[i] != v[i]:
            neq += 1
            prev_index = index
            index = i
    if neq != 2:
        return None

    return (prev_index, index)


def gen_group(n):
    group = list(itertools.permutations(range(1, n + 1)))
    G = nx.Graph()
    for i, u in enumerate(group):
        for j, v in enumerate(group[i + 1 :], start=i + 1):
            if is_transposition(u, v):
                G.add_edges_from([(i, j)])

    for u in group:
        if G.edges(u) != n - 1:
            RuntimeError("Error")

    return group, G


def get_transpositions(nodes):
    transpositions = []
    for i in range(len(nodes) - 1):
        u = nodes[i]
        v = nodes[i + 1]
        transpositions.append(get_transposition(u, v))
    return transpositions


# l1 containts l2
def contains(l1, l2):
    if len(l2) == 0:
        return True
    for i in range(len(l1) - len(l2)):
        if l1[i] == l2[0]:
            if l1[i : i + len(l2)] == l2:
                return True
    return False


def make_lattice(elements):
    G = nx.DiGraph()
    for i, (l1, e1) in enumerate(elements):
        for j, (l2, e2) in enumerate(elements):
            if e1 != e2:
                if contains(l1, l2):
                    G.add_edge(j, i)


# cargo llvm-cov --all-features --workspace --json
if __name__ == "__main__":

    parser = argparse.ArgumentParser(
        prog="S_n",
        description="The program returns elements of S_n sorted by their Coxeter length under (S_n, transpositions)",
    )
    parser.add_argument("n", type=int, help="Generate S_n")
    args = parser.parse_args()

    group, G = gen_group(args.n)

    print(G)

    descending = []
    while G.number_of_nodes() > 0:
        longest_path = find_longest_path(G)
        nodes = [group[i] for i in longest_path]
        transpositions = get_transpositions(nodes)
        if len(longest_path) > 0:
            descending.append((transpositions, group[longest_path[-1]]))
            print(
                f"{group[longest_path[-1]]} reduces to {transpositions} (len {len(longest_path) - 1})"
            )
        if len(longest_path) >= 2:
            G.remove_nodes_from([longest_path[-1]])
        else:
            break
