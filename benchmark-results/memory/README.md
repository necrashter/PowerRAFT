# Memory Benchmarks

Results of the benchmarks that measure the memory usage of the program.

## State Indexers

Benchmarking the memory usage of different state indexer implementations.

All indexers that are present in git commit `58e50ecaf353f79442426d3ee1875b12d4a7bf12`:
- `Naive`: Using `std::collections::HashMap` and storing new states in `Array2`s.
- `Stack HashMap`: Same as naive, but instead of storing new states in `Array2`s, only unexplored states are added to stack and removed. When MDP is built, `Array2`s are built from HashMap.
- `Stack BTreeMap`: Same as previous, but using `std::collections::BTreeMap`.
- `Stack hashbrown`: Same as previous, but using `hashbrown` library's `HashMap`.
- `Naive hashbrown`: `Naive` with `hashbrown`.
- `Queue hashbrown`: Same as `Stack hashbrown`, but using a queue instead of a stack.
- `minify 2^15`: Similar to `Stack hashbrown`. Removes unreachable states from the `HashMap` periodically (every `2^15` states) and adds removed states to `Array2`.
- `minify 2^20`: Same as previous with period = `2^20`.
- `tight stack`: Same as `Stack hashbrown`, but states are converted to `BitVec`s before indexing in HashMap. Uses minimum number of bits to store a state.
- `trie8`: Same as previous, but using a custom trie implementation. Each trie link holds 8 bits of information.
- `trie16`: Each trie link holds 16 bits of information.

Modifiers;
- `nobuild` in name denotes that the indexer didn't build the state `Array2` in `deconstruct` call, i.e., indexed states are left in the HashMap.

These were tested on `experiments/mem.json`.
Note that `ArrayStateIndexer` couldn't run the experiment due to horrible complexity (worst case `O(n)` for a single addition where `n` is the explored states).

![Memory Usage](./mem.mem.png)

![Execution Time](./mem.exec.png)

In `mem2.json`, I benchmarked the best 2 implementations again: `BitStackStateIndexer` (`tight stack`) and `TrieStateIndexer` (`trie8`).
There were no significant difference between their runtime performance and memory usage.
Note that this benchmark was performed without `nobuild` unlike the previous one, after Trie iterator was implemented (commit `d14f85e36fd9aabefed0250193aaccea181589c5`).
