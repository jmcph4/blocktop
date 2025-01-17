# blocktop #

[![asciicast](https://asciinema.org/a/698693.svg)](https://asciinema.org/a/698693)

Minimalist TUI block explorer for Ethereum networks.

 - Gain rapid insights into chain health by viewing new canonical blocks live in a visually clear, low-latency manner
 - Drill down into specific details of individual blocks and transactions
 - Store chain data to a relational schema using a simple, open format

See [FUNCTIONALITY](docs/FUNCTIONALITY.md) for the full feature list.

## Installation ##

```
$ cargo install blocktop
```

**Note**: `blocktop` is alpha software is is not stable yet.

## Usage ##

```
A minimalist TUI block explorer for Ethereum blockchains

Usage: blocktop [OPTIONS]

Options:
  -r, --rpc <RPC>          [default: wss://eth.merkle.io]
  -d, --db <DB>            
      --headless           
      --list-block-hashes  
  -h, --help               Print help
  -V, --version            Print version
```

At the moment, `blocktop` only supports Websockets or Unix domain sockets as transports for RPC communication. `blocktop` makes use of the [free Ethereum RPC service provided by Merkle](https://merkle.io/free-eth-rpc) by default.

### TUI Mode ###

The default invocation (i.e., `blocktop`) will open the TUI and start retrieving data from the default Ethereum RPC node using an in-memory SQLite database.

![Main page](https://pbs.twimg.com/media/GglTD6CbkAA1CpC?format=png&name=large)

#### Controls ####

| Key | Action |
| --- | --- |
| `j`, `k`, `Up`, `Down` | Scrolls lists | 
| `e` | In block or transaction view, opens the block or transaction in [Etherscan](https://etherscan.io), respectively |
| `q` | Exits the application |
| `Esc` | Returns to the previous page or exits the application if on the main page |

### Headless Mode ###

To invoke solely the indexer without the TUI frontend, specify the `--headless` flag. This mode is the most useful with the `RUST_LOG` environment variable configured to `info`:

```
$ RUST_LOG=info blocktop --headless
 2025-01-15T05:13:06.017Z INFO  blocktop::client > Websockets client initialised (endpoint: wss://eth.merkle.io/, chain: 1)
 2025-01-15T05:13:06.806Z INFO  blocktop::db     > Wrote block 0x2d21b100f838bb2656bcd0599cbdc30048d6d1a694581c6ec781e8f58961c729 to the database
 2025-01-15T05:13:08.077Z INFO  blocktop::client > Websockets client initialised (endpoint: wss://eth.merkle.io/, chain: 1)
 2025-01-15T05:13:19.203Z INFO  blocktop::db     > Wrote block 0x5850d0c1ba90da1cfe682ad29a727b841038ead07e198477869550cbb387f053 to the database
 2025-01-15T05:13:26.187Z INFO  blocktop::db     > Wrote block 0x52a43747e20465e7407ccba6915a027457220e06399ab992409b3ace66e40301 to the database
 2025-01-15T05:13:39.812Z INFO  blocktop::db     > Wrote block 0xf31df89a9277295916f714d78a3ccf708826951a7a6e0ac40563b18a51d14f76 to the database
```

As the warning-level log line at the start of the output indicates, headless operation also benefits from specifying an on-disk database to save chain state to:

```
$ RUST_LOG=info blocktop --headless --db foobar.db
```

