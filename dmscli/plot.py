#!/usr/bin/env python3

import argparse

parser = argparse.ArgumentParser(description="Plot dmscli experiment results from JSON files.")
parser.add_argument("filename")
parser.add_argument('-n', '--naming', dest="naming",
                    help="Names of benchmarks: default or opt")
parser.add_argument('-p', '--plot', dest="plot",
                    help="Plot: memory or time")
parser.add_argument('-b', '--bus', dest="bus_count",
                    type=int,
                    help="Number of buses")

args = parser.parse_args()

import json
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.ticker import MaxNLocator

plt.rc('font', size=14)

def plot_setup(l):
    fig, ax1 = plt.subplots(figsize=(12, l*0.6))
    fig.subplots_adjust(left=0.135, bottom=0.2)
    # fig.subplots_adjust(left=0.115, right=0.88)
    # fig.canvas.set_window_title('Eldorado K-8 Fitness Chart')
    return fig, ax1

def plot(benchmark_data, options):
    fig, ax1 = plot_setup(len(benchmark_data))

    benchmark_names = [b["name"] for b in benchmark_data]
    datas = options["fields"]
    errors = [b["error"] if "error" in b else None for b in benchmark_data]
    minmaxavg = (min(datas[0]) + max(datas[0]))/2

    pos = np.arange(len(benchmark_names))

    rect_height = 0.75

    data_rects = [
            ax1.barh(pos, data,
                     align='center',
                     height=rect_height,
                     tick_label=benchmark_names)
            for data in datas
            ]

    if options["title"]:
        ax1.set_title(options["title"], fontweight="bold")

    ax1.xaxis.set_major_locator(MaxNLocator(11))
    ax1.xaxis.grid(True, linestyle='--', which='major',
                   color='grey', alpha=.25)
    if "xlim" in options:
        ax1.set_xlim(*options["xlim"])
        minmaxavg = options["xlim"][0] * 0.25 + options["xlim"][1] * 0.75

    ax1.yaxis.set_ticks([i*0.999 for i in range(len(benchmark_names)) if i % 2 == 1], minor=True)
    ax1.yaxis.grid(True,
                   # linestyle='--',
                   fillstyle='full',
                   linewidth=29,
                   which='minor',
                   color='grey',
                   alpha=.5,
                   )
    ax1.yaxis.set_zorder(-1)

    if options["xlabel"]:
        ax1.set_xlabel(options["xlabel"])

    if "side_field" in options:
        field = options["side_field"]
        # Set the right-hand Y-axis ticks and labels
        ax2 = ax1.twinx()
        right_labels = [b[field] if field in b else "-" for b in benchmark_data]
        # set the tick locations
        ax2.set_yticks(pos)
        # make sure that the limits are set equally on both yaxis so the
        # ticks line up
        ax2.set_ylim(ax1.get_ylim())
        # set the tick labels
        ax2.set_yticklabels(right_labels)
        if options["side_label"]:
            ax2.set_ylabel(options["side_label"])

    field_format = options["field_format"] if "field_format" in options else "%.2f"

    for datum, rect, error in zip(datas[0], data_rects[0], errors):
        # Rectangle widths are already integer-valued but are floating
        # type, so it helps to remove the trailing decimal point and 0 by
        # converting width to int type
        width = rect.get_width()

        if datum < minmaxavg or error:
            # Shift the text to the right side of the right edge
            xloc = 5
            clr = 'black'
            align = 'left'
        else:
            # Shift the text to the left side of the right edge
            xloc = -5
            clr = 'white'
            align = 'right'

        label = error if error else field_format % (datum,)

        # Center the text vertically in the bar
        yloc = rect.get_y() + rect.get_height() / 2
        ax1.annotate(label, xy=(width, yloc), xytext=(xloc, 0),
                            textcoords="offset points",
                            ha=align, va='center',
                            color=clr, weight='bold', clip_on=True)

    for data, rects in zip(datas[1:], data_rects[1:]):
        for datum, rect, error in zip(data, rects, errors):
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
            label = ax1.annotate(field_format % (datum,), xy=(width, yloc), xytext=(xloc, 0),
                                textcoords="offset points",
                                ha=align, va='center',
                                color=clr, weight='bold', clip_on=True)

    return data_rects


def plot_time(benchmark_data, options={}):
    rects = plot(benchmark_data, {
        **options,
        "title": "Execution Time",
        "fields": [
            [b["totalTime"] if "totalTime" in b else 0 for b in benchmark_data],
            [b["generationTime"] if "generationTime" in b else 0 for b in benchmark_data],
        ],
        "xlabel": "Time (seconds)",
        "side_field": "states",
        "side_label": "Number of States",
        })
    plt.legend((rects[0][0], rects[1][0]), ('Total Time', 'Generation Time'), loc="upper left")

def plot_memory(benchmark_data, options={}):
    plot(benchmark_data, {
        **options,
        "title": "Max Memory Usage",
        "fields": [
            [b["maxMemory"] / 1024 / 1024 if "maxMemory" in b else 0 for b in benchmark_data],
        ],
        "xlabel": "Maximum Memory Usage (MB)",
        "side_field": "states",
        "side_label": "Number of States",
        })

def plot_ep(benchmark_data, options={}):
    plot(benchmark_data, {
        **options,
        "title": "Average Energization Probability",
        "fields": [
            [b["energizationP"] if "energizationP" in b else 0 for b in benchmark_data],
        ],
        "xlabel": "Energization Probability",
        "xlim": (0, 1),
        "side_field": "avgTime",
        "side_label": "Avg. Time",
        })

def plot_value(benchmark_data, options={}):
    plot(benchmark_data, {
        **options,
        "title": "Value Function",
        "fields": [
            [b["value"] if "value" in b else 0 for b in benchmark_data],
        ],
        "xlabel": "Minimum Value",
        "side_field": "states",
        "side_label": "Number of States",
        })

def plot_avg(benchmark_data, options={}):
    plot(benchmark_data, {
        **options,
        "title": "Average Time Until Energization",
        "fields": [
            [b["avgTime"] if "avgTime" in b else 0 for b in benchmark_data],
        ],
        "xlabel": "Average Time Until Energization",
        "side_field": "energizationP",
        "side_label": "Energization Probability",
        })

def plot_states(benchmark_data, options={}):
    plot(benchmark_data, {
        **options,
        "title": "Number of States",
        "fields": [
            [b["states"] if "states" in b else 0 for b in benchmark_data],
        ],
        "xlabel": "Number of States",
        "field_format": "%d",
        })

def plot_st(benchmark_data, options={}):
    plot(benchmark_data, {
        **options,
        "title": "Number of Transitions/States",
        "fields": [
            [b["transitions"] if "transitions" in b else 0 for b in benchmark_data],
            [b["states"] if "states" in b else 0 for b in benchmark_data],
        ],
        "xlabel": "Number of States/Transitions",
        "field_format": "%d",
        })


def get_optimization_name(d):
    def indexer(s):
        return ["S"] if s.startswith("Sorted") else []
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
    opts = indexer(d["indexer"]) + actions[d["actions"]] + transitions[d["transitions"]]
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
    if "simulation" in d:
        o.update(d["simulation"])
    return o


with open(args.filename) as f:
    data = json.load(f)

filename = args.filename[:args.filename.rindex('.')]

if args.naming == "opt":
    data = [ process_datum(d, get_optimization_name(d["optimizations"])) for d in data ]
else:
    data = [ process_datum(d, d["name"]) for d in data ]

plot_type = args.plot if args.plot else "t"
if plot_type.startswith("t"):
    plot_time(data[::-1], {})
    filename += ".exec"
elif plot_type.startswith("m"):
    plot_memory(data[::-1], {})
    filename += ".mem"
elif plot_type.startswith("v"):
    plot_value(data[::-1], {})
    filename += ".val"
elif plot_type.startswith("e"):
    plot_ep(data[::-1], {})
    filename += ".ep"
elif plot_type.startswith("a"):
    plot_avg(data[::-1], {})
    filename += ".avg"
elif plot_type.startswith("st"):
    plot_st(data[::-1], {})
    filename += ".st"
elif plot_type.startswith("s"):
    plot_states(data[::-1], {})
    filename += ".states"
else:
    print("Unknown plot type:", plot_type)

plt.savefig(filename + ".png", bbox_inches='tight')
# plt.show()
