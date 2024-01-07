# PowerRAFT: Power Restoration Application with Field Teams

PowerRAFT is a program for computing the optimal power restoration strategy in a post-disaster scenario.
It features a web-based interface as well as a command line interface for running the experiments.

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

## Compiling

If you have already installed Rust, Cargo will automatically download and compile almost all of the dependencies.
However, installing **Tch-rs** (Rust bindings for PyTorch's C++ API) requires some manual intervention.
You can follow the instructions provided in [this blog post](https://necrashter.github.io/tch-rs-install-from-pytorch) to install Tch-rs, or you can see the [Getting Started section in Tch-rs README](https://github.com/LaurentMazare/tch-rs#getting-started) for more information.
Note that you won't need CUDA support in Tch-rs.


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
cargo run --release -- dqn --help
```

Short aliases for some subcommands are also accepted for convenience, e.g., `r` instead of `run`.


## Experiments

This section explains how to run the DRL experiments.
If you want to run the experiments conducted in the paper "Field Teams Coordination for Earthquake-Damaged Distribution System Energization", see [this document](./FieldTeamExperiments.md).

### Exact Solutions

```sh
# Exact solutions on WSCC-9 with 2 and 3 teams.
cargo run --release -- r --no-save ../experiments/monte-carlo/wscc.exact.json 
# Exact solution on IEEE-37 with 1 team.
cargo run --release -- r --no-save ../experiments/ieee37.full.json
```

### Monte Carlo

These experiments are provided in `experiments/monte-carlo`, and they can be run as follows:
```sh
# Run Greedy Monte Carlo on WSCC-9 with 2 and 3 teams.
cargo run --release -- r --no-save ../experiments/monte-carlo/wscc.greedy.json 
# Run Greedy Monte Carlo with Action Elimination on IEEE-37 with 1 team.
cargo run --release -- r --no-save ../experiments/monte-carlo/ieee37.greedy.ae.json 
```
See the aforementioned folder for more Monte Carlo experiments.

### DQN Models

```sh
cd dmscli
# DQN on WSCC-9 with 2 teams.
# Trains for 100 checkpoints (100x500 iterations), can be interrupted with CTRL+C.
cargo run --release -- dqn train -c 100 --seed 30 --cpu ../dqn-models/wscc2.dqn.yaml 
# Dueling DQN on the same problem.
cargo run --release -- dqn train -c 100 --seed 30 --cpu ../dqn-models/wscc2.ddqn.yaml 
# Dueling DQN with Action Elimination on the same problem.
cargo run --release -- dqn train -c 100 --seed 30 --cpu ../dqn-models/wscc2.ddqn.ae.yaml 
```

Running on other problems is similar. Just replace `wscc2` with the desired problem name.
The available problems are `wscc2`, `wscc3`,`ieee37`, and `simple` (the 8 bus system on which the qualitative analysis was performed).
These model configuration files are human-readable `.yaml` files.

**NOTE:** The training will continue from the latest checkpoint when available.
You need to remove `dqn-models/{MODEL_NAME}.d` folder if you want to start over, where `{MODEL_NAME}` is the name of the configuration file without `.yaml` suffix.

In order to load the solutions into the UI, do the following:
```sh
# Run and convert the exact solution.
cargo run --release -- r ../experiments/simple.json 
cargo run --release -- convert results/simple.d/001.bin "Exact AE.json"

# Train DQN models on this problem.
cargo run --release -- dqn train -c 100 --seed 30 --cpu ../dqn-models/simple.dqn.yaml 
cargo run --release -- dqn train -c 100 --seed 30 --cpu ../dqn-models/simple.ddqn.yaml 
cargo run --release -- dqn train -c 100 --seed 30 --cpu ../dqn-models/simple.ddqn.ae.yaml 
# Convert the solutions from DQN models.
# You may need to change the checkpoint numbers.
cargo run --release -- convert ../dqn-models/simple.dqn.d/solution-88.bin "DQN.json"
cargo run --release -- convert ../dqn-models/simple.ddqn.d/solution-45.bin "DDQN.json"
cargo run --release -- convert ../dqn-models/simple.ddqn.ae.d/solution-88.bin "DDQN with AE.json"
cargo run --release -- convert ../dqn-models/simple.dqn.d/solution-1.bin "DQN - First Checkpoint.json"
```
Afterwards, move the resulting `.json` files in this directory to `graphs/simple.soln.d`, creating the directory if necessary.


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
