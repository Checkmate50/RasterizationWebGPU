"""
Written by Dietrich
Plots a given set of .result FPS measurement files
"""

import matplotlib.pyplot as plt
import sys
import numpy as np

def help():
    print("usage: python graph.py number_of_data_points [opt: plot_type] [opt: index_to_plot]")

def collect_data(filename : str) -> list[int]:
    to_return = []
    with open(filename, "r") as f:
        for x in f:
            x = x.strip()
            if len(x) > 0:
                to_return.append(int(x))
    return to_return

def collect_all(file_start : str, count : int) -> list[list[int]]:
    to_return = []
    for i in range(count):
        to_return.append(collect_data(f"results/{file_start}_{i}.result"))
    return to_return

def build_box_total(data : list[tuple[str, list[list[int]]]]):
    # blah blah concatenate all data
    total = [(x[0], [j for i in x[1] for j in i]) for x in data]
    fig = plt.figure()
    ax = fig.add_subplot(111)
    ax.boxplot([x[1] for x in total], meanline=True, labels = [x[0] for x in total])
    ax.set_ylabel("FPS")
    plt.show()

def build_box(data : list[tuple[str, list[list[int]]]], index : int):
    if index == -1:
        build_box_total(data)
        return
    fig = plt.figure()
    ax = fig.add_subplot(111)
    ax.boxplot(data[index][1], meanline=True)
    ax.set_ylabel("FPS")
    plt.show()

def build_histogram_total(data : list[tuple[str, list[list[int]]]]):
    means = [[np.mean(x) for x in y[1]] for y in data]
    plt.figure()
    plots = [plt.subplot(len(data)*100+11)]
    for i in range(1,len(data)):
        plots.append(plt.subplot(len(data)*100+11+i, sharex=plots[i-1]))
    binwidth = 2
    for i in range(len(data)):
        plots[i].hist(means[i], bins = range(int(min(min(means))), int(max(max(means))) + binwidth, binwidth))
        plots[i].set_ylabel(data[i][0], rotation=0)
        plots[i].spines['top'].set_visible(False)
    plots[-1].set_xlabel("FPS")
    plt.show()

def build_histograms(data : list[tuple[str, list[list[int]]]], index : int):
    if index == -1:
        build_histogram_total(data)
        return

def main():
    if len(sys.argv) < 2 or not sys.argv[1].isdigit():
        help()
        return
    data = []
    with open("bins.txt", "r") as f:
        for file_start in f:
            data.append((file_start.strip(), collect_all(file_start.strip(), int(sys.argv[1]))))
    plot_type = "histogram"
    if len(sys.argv) > 2:
        plot_type = sys.argv[2]
    index = -1
    if len(sys.argv) > 3:
        index = int(sys.argv[3])

    if plot_type == "box":
        build_box(data, index)
    elif plot_type in {"hist", "histogram"}:
        build_histograms(data, index)
    else:
        print("For the 2nd ")

if __name__=="__main__":
    main()