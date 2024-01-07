# Field Teams Experiments

In this section, the commands for running the experiments conducted in the paper are provided.
The section numbers are given in each subheading.

All experiments must be run in `dmscli` directory:
```sh
cd dmscli
```

## 5.2. Performance Evaluation for the Optimizations
```sh
# WSCC 9-bus with starting teams (9, 9, 9)
cargo run --release -- r --no-save ../experiments/opt.wscc.t-9-9-9.json
# WSCC 9-bus with starting teams (9, 9)
cargo run --release -- r --no-save ../experiments/opt.wscc.t-9-9.json

# 12-bus with only bus 1 connected to TG and starting teams (1, 1, 1)
cargo run --release -- r --no-save ../experiments/opt.12-bus.a.json
# 12-bus with only buses 1 and 10 connected to TG and starting teams (1, 1)
cargo run --release -- r --no-save ../experiments/opt.12-bus.b.json

# 17-bus system with teams starting at (1, 1)
cargo run --release -- r --no-save ../experiments/opt.midsize00.sov.json
cargo run --release -- r --no-save ../experiments/opt.midsize00.sow.json
cargo run --release -- r --no-save ../experiments/opt.midsize00.spov.json
cargo run --release -- r --no-save ../experiments/opt.midsize00.spow.json
```

Note that the last experiment (the one with the 17-bus system) is composed of 4 separate experiment files.
The resulting `.json` files must be merged manually in order to reproduce Figure 6.
To merge `.json` files, concatenate them in a text editor and replace `] [` parts between them with `,` so that it becomes one JSON array.

## 5.3.1. The Number of Teams
```sh
cargo run --release -- r --no-save ../experiments/team.wscc.json 
```

## 5.3.2. The Number of Buses
```sh
cargo run --release -- r --no-save ../experiments/bus.midsize.base.json
cargo run --release -- r --no-save ../experiments/bus.midsize.d1.json
cargo run --release -- r --no-save ../experiments/bus.midsize.d2.json
```

Note that `bus.midsize.d1.json` contains additional starting configurations that were omitted in the paper for brevity.
To reproduce the exact graphs from the paper, the additional results from `bus.midsize.d1.json` must be removed and the remaining ones must be merged with the results from `bus.midsize.base.json` and `bus.midsize.d2.json`.

## 5.3.3. The Number of Branches
```sh
cargo run --release -- r --no-save ../experiments/branch.midsize.json 
```

## 5.3.4. The Number of Transmission Grid Connections
```sh
cargo run --release -- r --no-save ../experiments/tg.12-bus.json 
# The following is an omitted experiment where the failure probabilities of
# all buses are set to 0.25 in the previous experiment.
cargo run --release -- r --no-save ../experiments/tg.12-bus.pf25.json 
```

## 5.4. Evaluation of Scalability with Partitioning

Please bear in mind that you will need more than 16 GB of RAM (approximately 24 GB) for IEEE-37 Full and IEEE-123 Part B.

```sh
cargo run --release -- r --no-save ../experiments/ieee37.full.json 
cargo run --release -- r --no-save ../experiments/ieee37.parted.json 
cargo run --release -- r --no-save ../experiments/ieee123.parted.json
```


# Plotting the experiment results

If an experiment is executed successfully, the corresponding `.json` file containing the results will be created in `dmscli/results` directory.
Since JSON files are human readable, these files can be inspected manually.
Besides, the Python script `dmscli/plot.py` is provided for creating plots from these JSON files.

`numpy` and `matplotlib` libraries must be installed in order to use this script.

`plot.py` receives one positional argument containing the result JSON file.
In addition to this, `-p` argument must be provided to specify the plot type. Available plot types:
- `t`: Execution time
- `m`: Memory
- `v`: Value function
- `ep`: Average energization probability
- `ac`: Average expected cost per bus
- `a`: Average time until energization
- `st`: Number of transitions and states
- `s`: Number of states

The optional `-n` argument determines the name of each run in experiment.
By default, the name given in the experiment file is used, but for optimization experiments, it's more convenient to use the optimization name instead.
When `-n opt` is provided, each run is named according to the optimizations used.
The naming convention for optimizations is explained in the paper.

Example usage:
```sh
python3 plot.py -p t -n opt results/opt.wscc.t-9-9-9.json
# Figure 13
python3 plot.py -p ac results/tg.12-bus.json 
# Figure 14
python3 plot.py -p st results/tg.12-bus.json 
```

The created plots are saved in the results directory.
The script does not display the plots after running.
