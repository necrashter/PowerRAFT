# PowerRAFT: Power Restoration Application with Field Teams

PowerRAFT is a program for computing the optimal power restoration strategy in a post-disaster scenario.
It features a web-based interface as well as a command line interface for running the experiments.

This software accompanies the paper ["Field teams coordination for earthquake-damaged distribution system energization"](https://doi.org/10.1016/j.ress.2024.110050) ([arxiv](https://arxiv.org/abs/2404.04087)) which is published in the journal ["Reliability Engineering & System Safety"](https://www.sciencedirect.com/journal/reliability-engineering-and-system-safety). Please [cite](#citation) this paper if you find our work valuable.

## Demonstration Video

Click [this link](https://youtu.be/rr4daz0UgrY) or the image below to watch:

[![Watch the video](https://img.youtube.com/vi/rr4daz0UgrY/maxresdefault.jpg)](https://youtu.be/rr4daz0UgrY)


## Cloning the repository

This repository contains a submodule. Thus, it needs to be cloned with `--recurse-submodules` option:
```sh
git clone --recurse-submodules https://github.com/necrashter/PowerRAFT
```

If you have already cloned the repository normally, run the following commands in the cloned directory to initialize the submodules:
```sh
git submodule update --init --recursive
```


## Running the server

Rust must be installed in your system.

```sh
cd server
# Debug mode
cargo run
# Release mode, much faster
cargo run --release
```

If no errors occur, you should be able to access the web interface at `http://127.0.0.1:8000`.


## Command line interface

The command line interface of PowerRAFT is named `dmscli` which stands for Disaster Management System Command Line Interface.
It can be run as follows:
```sh
cd dmscli
cargo run --release -- <arguments to dmscli>
```

The best way to learn about the available command line arguments is to run the help command.
There are multiple subcommands, each with their own arguments.
```sh
cd dmscli
# Get help on available subcommands:
cargo run --release -- help
# Get more information about the run subcommand:
cargo run --release -- run --help
# Similarly for other subcommands:
cargo run --release -- solve --help
```

Short aliases for subcommands are also accepted for convenience, e.g., `r` instead of `run`.


## Running the experiments

In this section, the commands for running the experiments conducted in the paper are provided.
The section numbers are given in each subheading.

All experiments must be run in `dmscli` directory:
```sh
cd dmscli
```

### 5.2. Performance Evaluation for the Optimizations
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

### 5.3.1. The Number of Teams
```sh
cargo run --release -- r --no-save ../experiments/team.wscc.json 
```

### 5.3.2. The Number of Buses
```sh
cargo run --release -- r --no-save ../experiments/bus.midsize.base.json
cargo run --release -- r --no-save ../experiments/bus.midsize.d1.json
cargo run --release -- r --no-save ../experiments/bus.midsize.d2.json
```

Note that `bus.midsize.d1.json` contains additional starting configurations that were omitted in the paper for brevity.
To reproduce the exact graphs from the paper, the additional results from `bus.midsize.d1.json` must be removed and the remaining ones must be merged with the results from `bus.midsize.base.json` and `bus.midsize.d2.json`.

### 5.3.3. The Number of Branches
```sh
cargo run --release -- r --no-save ../experiments/branch.midsize.json 
```

### 5.3.4. The Number of Transmission Grid Connections
```sh
cargo run --release -- r --no-save ../experiments/tg.12-bus.json 
# The following is an omitted experiment where the failure probabilities of
# all buses are set to 0.25 in the previous experiment.
cargo run --release -- r --no-save ../experiments/tg.12-bus.pf25.json 
```

### 5.4. Evaluation of Scalability with Partitioning

Please bear in mind that you will need more than 16 GB of RAM (approximately 24 GB) for IEEE-37 Full and IEEE-123 Part B.

```sh
cargo run --release -- r --no-save ../experiments/ieee37.full.json 
cargo run --release -- r --no-save ../experiments/ieee37.parted.json 
cargo run --release -- r --no-save ../experiments/ieee123.parted.json
```


## Plotting the experiment results

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


## Running the unit tests

Run smaller tests that evaluate the core functionality:
```sh
cargo test
```

Run the integration tests that take longer to execute:
```sh
cargo test -- --ignored
```

Run both kinds of tests:
```sh
cargo test -- --include-ignored
```

For more information, please see [cargo-test documentation](https://doc.rust-lang.org/cargo/commands/cargo-test.html).


## Citation

Please cite our paper if you found this work valuable:
```
@article{ISIK2024110050,
title = {Field teams coordination for earthquake-damaged distribution system energization},
journal = {Reliability Engineering & System Safety},
volume = {245},
pages = {110050},
year = {2024},
issn = {0951-8320},
doi = {https://doi.org/10.1016/j.ress.2024.110050},
url = {https://www.sciencedirect.com/science/article/pii/S095183202400125X},
author = {İlker Işık and Ebru {Aydin Gol}},
keywords = {Decision support, Markov decision process, Stochastic systems, Power networks, Disaster management},
abstract = {The re-energization of electrical distribution systems in a post-disaster scenario is of grave importance as most modern infrastructure systems rely heavily on the presence of electricity. This paper introduces a method to coordinate the field teams for the optimal energization of an electrical distribution system after an earthquake-induced blackout. The proposed method utilizes a Markov Decision Process (MDP) to create an optimal energization strategy, which aims to minimize the expected time to energize each distribution system component. The travel duration of each team and the possible outcomes of the energization attempts are considered in the state transitions. The failure probabilities of the system components are computed using the fragility curves of structures and the Peak Ground Acceleration (PGA) values which are encoded to the MDP model via transition probabilities. Furthermore, the proposed solution offers several methods to determine the non-optimal actions during the construction of the MDP and eliminate them in order to improve the run-time performance without sacrificing the optimality of the solution.}
}
```
