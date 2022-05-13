"""
Written by Dietrich
Plots a given set of .result FPS measurement files
"""

import matplotlib.pyplot as plt
import sys
import numpy as np

def help():
    print("usage: python graph.py filename.result [opt: plot_type] [opt: index_to_plot]")

def add_to(data : list[tuple[str, list[list[int]]]], name : str, to_add : list[int]):
    if len(to_add) == 0:
        return
    for item in data:
        if item[0] == name:
            item[1].append(to_add)
            return
    data.append((name, [to_add]))

def collect_all(filename : str) -> list[list[int]]:
    data = []
    to_add = []
    prev_name = ""
    with open(filename, "r") as f:
        for line in f:
            line = line.strip()
            if len(line) == 0:
                continue
            if line.startswith("---"):
                name = line[3:-3]
                if prev_name:
                    add_to(data, prev_name, to_add)
                prev_name = name
                to_add = []
            else:
                to_add.append(int(line))
        if prev_name:
            add_to(data, prev_name, to_add)
    return data

def build_box_total(data : list[tuple[str, list[list[int]]]]):
    # blah blah concatenate all data
    total = [(x[0], [j for i in x[1] for j in i]) for x in data]
    fig = plt.figure()
    ax = fig.add_subplot(111)
    ax.boxplot([x[1] for x in total], meanline=True, labels = [x[0] for x in total])
    ax.set_ylabel("nSPF")
    plt.show()

def build_box(data : list[tuple[str, list[list[int]]]], index : int):
    if index == -1:
        build_box_total(data)
        return
    fig = plt.figure()
    ax = fig.add_subplot(111)
    ax.boxplot(data[index][1], meanline=True)
    ax.set_ylabel("nSPF")
    plt.show()

def build_scatter_cv(data : list[tuple[str, list[list[int]]]]):
    fig = plt.figure()
    ax = fig.add_subplot(111)
    indices = []
    for i in range(len(data)):
        indices += [data[i][0]] * len(data[i][1])
    devs = [[np.std(x) / np.mean(x) for x in y[1]] for y in data]
    ax.scatter(indices, [j for i in devs for j in i])
    ax.set_ylabel("Coefficient of Variation")
    plt.show()

def build_scatter(data : list[tuple[str, list[list[int]]]], index : int):
    if index == -1:
        build_scatter_cv(data)
        return
    fig = plt.figure()
    ax = fig.add_subplot(111)
    indices = []
    for i in range(len(data[index][1])):
        indices += [i] * len(data[index][1][i])
    ax.scatter(indices, [j for i in data[index][1] for j in i])
    ax.set_ylabel("nSPF")
    ax.set_xlabel("replicate of " + data[index][0])
    plt.show()

def build_histogram_total(data : list[tuple[str, list[list[int]]]]):
    means = [[np.mean(x) for x in y[1]] for y in data]
    plt.figure()
    plots = [plt.subplot(len(data)*100+11)]
    for i in range(1,len(data)):
        plots.append(plt.subplot(len(data)*100+11+i, sharex=plots[i-1]))
    binwidth = int(1e4)
    for i in range(len(data)):
        plots[i].hist(means[i], bins = range(int(min(min(means))), int(max(max(means))) + binwidth, binwidth))
        plots[i].set_ylabel(data[i][0], rotation=0)
        plots[i].spines['top'].set_visible(False)
    plots[-1].set_xlabel("nSPF")
    plt.show()

def build_histogram(data : list[tuple[str, list[list[int]]]], index : int):
    if index == -1:
        build_histogram_total(data)
        return
    fig = plt.figure()
    ax = fig.add_subplot(111)
    binwidth = int(5e6)
    ax.hist(data[index][1], bins = range(int(min(min(data[index][1]))), int(max(max(data[index][1]))) + binwidth, binwidth))
    plt.show()

def build_line(data : list[tuple[str, list[list[int]]]], index : int):
    fig = plt.figure()
    ax = fig.add_subplot(111)
    ax.plot(data[index][1][0])
    plt.show()

def main():
    if len(sys.argv) < 2:
        help()
        return
    data = collect_all(sys.argv[1])
    plot_type = "histogram"
    if len(sys.argv) > 2:
        plot_type = sys.argv[2]
    index = -1
    if len(sys.argv) > 3:
        index = int(sys.argv[3])

    if plot_type == "box":
        build_box(data, index)
    elif plot_type == "scatter":
        build_scatter(data, index)
    elif plot_type in {"hist", "histogram"}:
        build_histogram(data, index)
    elif plot_type == "line":
        build_line(data, index)
    else:
        print("For the 2nd argument, select box, scatter, line, or hist")

if __name__=="__main__":
    main()