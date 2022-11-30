#!/usr/bin/env python3

import argparse

parser = argparse.ArgumentParser(description="Plot dmscli experiment results from JSON files.")
parser.add_argument("filename")
parser.add_argument('-n', '--naming', dest="naming",
                    help="Names of benchmarks: default or opt")
parser.add_argument('-p', '--plot', dest="plot",
                    help="Plot: memory or time")

args = parser.parse_args()

import json
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.ticker import MaxNLocator


def plot(benchmark_data, options):
    fig, ax1 = plt.subplots(figsize=(12, 6))
    fig.subplots_adjust(left=0.125)
    # fig.subplots_adjust(left=0.115, right=0.88)
    # fig.canvas.set_window_title('Eldorado K-8 Fitness Chart')

    benchmark_names = [b["name"] for b in benchmark_data]
    total_times = [b["total_time"] if "total_time" in b else 0
                   for b in benchmark_data]
    generation_times = [b["generation_time"]  if "generation_time" in b else 0
                        for b in benchmark_data]
    errors = [b["error"] if "error" in b else None for b in benchmark_data]
    minmaxavg = (min(total_times) + max(total_times))/2

    pos = np.arange(len(benchmark_names))

    rect_height = 0.75
    total_rects = ax1.barh(pos, total_times,
                     align='center',
                     height=rect_height,
                     tick_label=benchmark_names)
    generation_rects = ax1.barh(pos, generation_times,
                     align='center',
                     height=rect_height,
                     tick_label=benchmark_names)

    ax1.set_title("Benchmark Results", fontweight="bold")

    ax1.xaxis.set_major_locator(MaxNLocator(11))
    ax1.xaxis.grid(True, linestyle='--', which='major',
                   color='grey', alpha=.25)

    ax1.set_xlabel("Time (seconds)")

    if not "no-states" in options:
        # Set the right-hand Y-axis ticks and labels
        ax2 = ax1.twinx()
        right_labels = [b["states"] if "states" in b else "-" for b in benchmark_data]
        # set the tick locations
        ax2.set_yticks(pos)
        # make sure that the limits are set equally on both yaxis so the
        # ticks line up
        ax2.set_ylim(ax1.get_ylim())
        # set the tick labels
        ax2.set_yticklabels(right_labels)
        ax2.set_ylabel('Number of States')

    for time, rect, error in zip(total_times, total_rects, errors):
        # Rectangle widths are already integer-valued but are floating
        # type, so it helps to remove the trailing decimal point and 0 by
        # converting width to int type
        width = rect.get_width()

        if time < minmaxavg or error:
            # Shift the text to the right side of the right edge
            xloc = 5
            clr = 'black'
            align = 'left'
        else:
            # Shift the text to the left side of the right edge
            xloc = -5
            clr = 'white'
            align = 'right'

        label = error if error else "%.2f" % (time,)

        # Center the text vertically in the bar
        yloc = rect.get_y() + rect.get_height() / 2
        ax1.annotate(label, xy=(width, yloc), xytext=(xloc, 0),
                            textcoords="offset points",
                            ha=align, va='center',
                            color=clr, weight='bold', clip_on=True)

    for time, rect in zip(generation_times, generation_rects):
        # Rectangle widths are already integer-valued but are floating
        # type, so it helps to remove the trailing decimal point and 0 by
        # converting width to int type
        width = rect.get_width()

        # Shift the text to the left side of the right edge
        xloc = -5
        clr = 'black'
        align = 'right'

        # Center the text vertically in the bar
        yloc = rect.get_y() + rect.get_height() / 2
        label = ax1.annotate("%.2f" % (time,), xy=(width, yloc), xytext=(xloc, 0),
                            textcoords="offset points",
                            ha=align, va='center',
                            color=clr, weight='bold', clip_on=True)

    plt.legend((total_rects[0], generation_rects[0]), ('Total Time', 'Generation Time'), loc="upper left")
    plt.show()


def plot_memory(benchmark_data, options):
    fig, ax1 = plt.subplots(figsize=(12, 6))
    fig.subplots_adjust(left=0.125)
    # fig.subplots_adjust(left=0.115, right=0.88)
    # fig.canvas.set_window_title('Eldorado K-8 Fitness Chart')

    benchmark_names = [b["name"] for b in benchmark_data]
    mems = [b["max_memory"] / 1024 / 1024 for b in benchmark_data]
    errors = [b["error"] if "error" in b else None for b in benchmark_data]
    minmaxavg = (min(mems) + max(mems))/2

    pos = np.arange(len(benchmark_names))

    rect_height = 0.75
    total_rects = ax1.barh(pos, mems,
                     align='center',
                     height=rect_height,
                     tick_label=benchmark_names)

    ax1.set_title("Benchmark Results", fontweight="bold")

    ax1.xaxis.set_major_locator(MaxNLocator(11))
    ax1.xaxis.grid(True, linestyle='--', which='major',
                   color='grey', alpha=.25)

    ax1.set_xlabel("Maximum Memory Usage (MB)")

    if not "no-states" in options:
        # Set the right-hand Y-axis ticks and labels
        ax2 = ax1.twinx()
        right_labels = [b["states"] if "states" in b else "TIME OUT"
                        for b in benchmark_data]
        # set the tick locations
        ax2.set_yticks(pos)
        # make sure that the limits are set equally on both yaxis so the
        # ticks line up
        ax2.set_ylim(ax1.get_ylim())
        # set the tick labels
        ax2.set_yticklabels(right_labels)
        ax2.set_ylabel('Number of States')

    for mem, rect, error in zip(mems, total_rects, errors):
        # Rectangle widths are already integer-valued but are floating
        # type, so it helps to remove the trailing decimal point and 0 by
        # converting width to int type
        width = rect.get_width()

        if mem < minmaxavg or error:
            # Shift the text to the right side of the right edge
            xloc = 5
            clr = 'black'
            align = 'left'
        else:
            # Shift the text to the left side of the right edge
            xloc = -5
            clr = 'white'
            align = 'right'

        label = error if error else ("%.2f" % (mem,))

        # Center the text vertically in the bar
        yloc = rect.get_y() + rect.get_height() / 2
        ax1.annotate(label, xy=(width, yloc), xytext=(xloc, 0),
                            textcoords="offset points",
                            ha=align, va='center',
                            color=clr, weight='bold', clip_on=True)

    # plt.legend((total_rects[0], ), ('Total Time', ), loc="upper left")
    plt.show()


def get_optimization_name(d):
    indexer = {
            "NaiveStateIndexer": [],
            "SortedStateIndexer": ["S"],
    }
    actions = {
            "NaiveActions": [],
            "PermutationalActions": ["P"],
            "FilterOnWay<NaiveActions>": ["O"],
            "FilterOnWay<PermutationalActions>": ["P", "O"],
            "FilterEnergizedOnWay<NaiveActions>": ["O"],
            "FilterEnergizedOnWay<PermutationalActions>": ["P", "O"],
    }
    transitions = {
            "NaiveActionApplier": [],
            "TimedActionApplier<TimeUntilArrival>": ["V"],
            "TimedActionApplier<TimeUntilEnergization>": ["W"],
    }
    opts = indexer[d["indexer"]] + actions[d["actions"]] + transitions[d["transitions"]]
    if opts:
        return " + ".join(opts)
    else:
        return "-"

def process_datum(d, name):
    o = { "name": name }
    if "error" in d["result"] and "description" in d["result"]:
        o["error"] = d["result"]["description"]
    if "success" in d["result"]:
        o.update(d["result"]["success"])
    return o


with open(args.filename) as f:
    data = json.load(f)

if args.naming == "opt":
    data = [ process_datum(d, get_optimization_name(d["optimizations"])) for d in data ]
else:
    data = [ process_datum(d, d["name"]) for d in data ]

if args.plot:
    if args.plot.startswith("t"):
        plot(data[::-1], {})
    elif args.plot.startswith("m"):
        plot_memory(data[::-1], {})
    else:
        print("Unknown plot type:", args.plot)
else:
    plot(data[::-1], {})
