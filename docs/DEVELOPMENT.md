# Development Guide #

## Workflow ##

Use the `Makefile` if you want to.

## Dependencies ##

```
$ cargo install cargo-machete cargo-audit
```

## Libraries ##

 - TUI is [Ratatui](https://docs.rs/ratatui)
 - Database is [rusqlite](https://docs.rs/rusqlite) with [r2d2-sqlite](https://docs.rs/r2d2-sqlite) for concurrency
 - Chain interaction is [Alloy](https://docs.rs/alloy)

