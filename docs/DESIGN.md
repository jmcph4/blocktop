# Design #

`blocktop`, being a block explorer, necessarily indexes the blockchain being explored. Blockchain data is retrieved directly from the specified RPC node via a supported transport (see [`FUNCTIONALITY`](docs/FUNCTIONALITY.md)). At a high level, this is simply the blocks themselves. This data is then inserted into a local SQLite database (either in-memory or on-disk). The UI then retrieves this data directly from the database, caching only what's necessarily to ensure a sufficiently low latency user experience.

```mermaid
flowchart LR
    Node --> Database
    Database --> TUI
```

`blocktop` is mulithreaded: RPC node communication, database connection, and the UI each have their own threads.

