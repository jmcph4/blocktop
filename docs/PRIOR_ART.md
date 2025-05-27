# Prior Art #

## Competitors ##

| Product Name | Description | Open Source? | Source Code (if applicable) | Actively Maintained? |
| --- | --- | --- | --- | --- |
| blockrs | TUI block explorer in Rust (even uses Ratatui). More intended to watch live chain data. Very pretty UI. | ✅ | [sergerad/blockrs](https://github.com/sergerad/blockrs) | ✅ |
| lazy-etherscan | Another TUI block explorer in Rust. Allows searching on numerous fields. | ✅ | [woxjro/lazy-etherscan](https://github.com/woxjro/lazy-etherscan) | ✅ |
| ethscan | TUI block explorer in Go and inspired by `k9s`. | ✅ | [treethought/ethscan](https://github.com/treethought/ethscan) | ❌ (last commit was over three years ago) |

## Comparative Analysis ##

| Functionality                                                                            | blockrs | lazy-etherscan | Blocktop |
|------------------------------------------------------------------------------------------|:-------:|:--------------:|:--------:|
| Display the block numbers of the latest blocks                                           |    ✅   |        ✅      |    ✅    |
| Identify who built a block (including locally built blocks)                              |    ❌   |        ❌      |    ✅    |
| Display gas price of most recent blocks                                                  |    ❌   |        ✅      |    ✅    |
| Display gas limit of most recent blocks                                                  |    ❌   |        ✅      |    ✅    |
| Store block headers to a local SQLite database on disk                                   |    ❌   |        ❌      |    ✅    |
| Store block headers to a local SQLite database in memory                                 |    ❌   |        ❌      |    ✅    |
| Operate headlessly                                                                       |    ❌   |        ❌      |    ✅    |
| Plot gas usage of most recent blocks as a barchart                                       |    ❌   |        ❌      |    ✅    |
| Connect to an Ethereum EL node via Websockets                                            |    ❌   |        ❌      |    ✅    |
| Connect to an Ethereum EL node via IPC (i.e., Unix sockets)                              |    ❌   |        ❌      |    ✅    |
| Open a block on Etherscan in the default system web browser                              |    ❌   |        ❌      |    ✅    |
| Open a transaction on Etherscan in the default system web browser                        |    ❌   |        ❌      |    ✅    |
| Display details of a particular block                                                    |    ❌   |        ✅      |    ✅    |
| Display details of a particular transaction                                              |    ❌   |        ✅      |    ✅    |
| Write information about the state of the index to standard output                        |    ❌   |        ❌      |    ✅    |

