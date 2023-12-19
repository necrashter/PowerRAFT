#!/usr/bin/env python3
"""
Runs the Monte Carlo experiment given in EXP with different seeds
and reports the results (min, avg, max).
"""
EXP = "wscc3.random.ae"
RUNS = 25

import os
import json

def process_json_files(json_files):
    values = []
    states = []

    for json_file in json_files:
        with open(json_file, 'r') as f:
            data = json.load(f)
            result = data[0]["result"]["success"]
            values.append(result["value"])
            states.append(result["states"])

    avg_value = sum(values) / len(values)
    min_value = min(values)
    max_value = max(values)

    avg_states = sum(states) / len(states)
    min_states = min(states)
    max_states = max(states)

    print("Minimum Value:", min_value)
    print("Average Value:", avg_value)
    print("Maximum Value:", max_value)

    print("Minimum States:", min_states)
    print("Average States:", avg_states)
    print("Maximum States:", max_states)

filenames = []

os.system("mkdir temp")

for i in range(RUNS):
    seed = 42 + i
    os.system(f"cargo run --release -- r --no-save ../experiments/monte-carlo/{EXP}.json --seed {seed}")
    name = f"temp/{EXP}.{i}.json"
    os.system(f"mv results/{EXP}.json {name}")
    filenames.append(name)

process_json_files(filenames)
