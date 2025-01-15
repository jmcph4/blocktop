# Security #

## Security Contact ##

Submit all vulnerabilities to:

![https://jmcph4.dev/assets/images/email.png]

## Security Considerations ##

`blocktop` is a lightweight development tool and, as such, does not really have many serious security implications. There are a few things to consider though, for the sake of completeness.

### Node Trust ###

The RPC node used to retrieve blockchain data from must be entirely trusted not to:

 - Submit malformed data that could be used to perform either
    - [Log injection](https://notes.ethereum.org/Wg2pH0o3Q1-K2BMowW5vuA) or,
    - SQL injection (although `blocktop` uses the prepared statements API provided by `rusqlite`).
 - Submit incorrect blockchain data that is otherwise valid
    - Stale data will mean that the index lags behind the rest of the network
    - Reorgs (see below)

### Blockchain Reorganisations ###

[Blockchain reorganisations](https://www.alchemy.com/overviews/what-is-a-reorg) (or *reorgs*) are a possibility in modern blockchain networks.

Currently, `blocktop` does **not** handle reorgs at all and will happily continue to write data to the index and display this data to the user (unless in headless mode, obviously). It is advisable to understand how your RPC node (be it your own or that of a third-party provider) handles reorgs and how this impacts your use case for `blocktop`.

