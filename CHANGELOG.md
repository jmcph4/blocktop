# Changelog #

## 0.2.0 ##

### Features ###

 - Blocktop is now ready for the [Pectra hard fork](https://eips.ethereum.org/EIPS/eip-7600)
 - Addresses are now labelled by using [the dataset derived from the public Etherscan labels](https://github.com/dawsbot/eth-labels)
 - Arbitrary blocks and transactions can now be loaded via the `--block` and `--tx` CLI flags, respectively (`r` toggles this in the UI)
 - Transaction input data is now rendered in a hex editor widget
 - In block view, `l` opens the selected block in [Cryptic Woods Research](https://crypticwoods.com)'s [libMEV](https://libmev.com) tool
 - The UI now gracefully handles `Ctrl+C` by exiting
 - Blocktop now (optionally) exposes Prometheus metrics via the new `--metrics` CLI flag
 - Blocktop is now containerised via Docker

### Bugfixes ###

 - When selecting an arbitrary transaction (i.e., via the `--tx` CLI flag), the transaction's timestamp was akways displayed as the timestamp of the most recent known block

### PRs ###

 - [Pectra support](https://github.com/jmcph4/blocktop/pull/30) by [@jmcph4](https://github.com/jmcph4)
 - [Add libMEV support for blocks](https://github.com/jmcph4/blocktop/pull/25)
 - [Fix timestamp bug for arbitrary transactions](https://github.com/jmcph4/blocktop/pull/21) by [@jmcph4](https://github.com/jmcph4)
 - [Specific block and transaction retrieval](https://github.com/jmcph4/blocktop/pull/19) by [@jmcph4](https://github.com/jmcph4)
 - [Add ^C handler](https://github.com/jmcph4/blocktop/pull/15) by [@alecdwm](https://github.com/alecdwm)
 - [Implement basic address labelling](https://github.com/jmcph4/blocktop/pull/12) by [@jmcph4](https://github.com/jmcph4)
 - [Add hex view for transaction data](https://github.com/jmcph4/blocktop/pull/10)

### New Contributors ###

 - [@alecdwm](https://github.com/alecdwm)

