# blocktop #

Minimalist TUI block explorer for Ethereum networks.

 - Gain rapid insights into chain health by viewing new canonical blocks live in a visually clear, low-latency manner
 - Drill down into specific details of individual blocks and transactions
 - Store chain data to a relational schema using a simple, open format

See [FUNCTIONALITY](docs/FUNCTIONALITY.md) for the full feature list.

## Usage ##

```
$ blocktop -h
Usage: blocktop [OPTIONS]

Options:
  -r, --rpc <RPC>  [default: wss://eth.merkle.io]
  -d, --db <DB>    
      --headless   
  -h, --help       Print help
```

### Default Invocation ###

The default invocation (i.e., `blocktop`) will open the TUI and start retrieving data from the default Ethereum RPC node using an in-memory SQLite database.

### Headless Mode ###

To invoke solely the indexer without the TUI frontend, specify the `--headless` flag. This mode is the most useful with the `RUST_LOG` environment variable configured to either `info` or `debug`:

```
$ RUST_LOG=debug blocktop --headless
 2025-01-09T11:11:50.215Z WARN  blocktop > Headless mode without specifying an on-disk database. All data will be lost on exit.
 2025-01-09T11:11:52.661Z DEBUG tungstenite::handshake::client > Client handshake done.
 2025-01-09T11:11:55.159Z INFO  blocktop::client               > Websockets client initialised (endpoint: wss://eth.merkle.io/, chain: 1)
 2025-01-09T11:11:55.159Z DEBUG blocktop::client               > Subscribing to block header stream...
 2025-01-09T11:12:06.329Z DEBUG blocktop::services::blockchain > Saved header: 0xf951b93211d58182790f7d116643885c85a497411781361d9214ff0853473c93
 2025-01-09T11:12:06.329Z DEBUG blocktop::client               > Subscribing to block header stream...
 2025-01-09T11:12:17.804Z DEBUG blocktop::services::blockchain > Saved header: 0x47f76ce9be6b1985da8af06498f436bae59c8f9feb226f7ad6eabd22bc0585f6
 2025-01-09T11:12:17.804Z DEBUG blocktop::client               > Subscribing to block header stream...
```

As the warning-level log line at the start of the output indicates, headless operation also benefits from specifying an on-disk database to save chain state to:

```
$ RUST_LOG=debug blocktop --headless --db foobar.db
```

