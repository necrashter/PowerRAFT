# Experiments

Experiments are stored in this directory.
Distribution system graphs for these experiments are in `../graphs/`, and `./optimizations/` contains some configurations for optimizations (action elimination).

## Naming

An experiment filename is of form `"{Experiment type}.{System name}.{Additional info}.json"` where:
- Experiment type determines the independent variable in the experiment.
  - Most experiments are conducted with increasing `branch`, `bus` or `team` counts while keeping everything else constant.
  - `opt` denotes that different optimizations are tested on the same system with the same team configuration.
- System name is a short alias for the distribution system.
- Additional info might contain:
  - `pf0`: All failure probabilities are overridden to 0.
  - `h30`: Policy optimization horizon is fixed to 30.

## Experiment Content

All experiments are saved in human-readable JSON files. Most contain a short description.
