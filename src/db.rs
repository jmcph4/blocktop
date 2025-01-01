use std::{path::PathBuf, sync::Arc, time::Duration};

use alloy::{primitives::B256, rpc::types::eth::Header};
use eyre::eyre;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

const CONN_GET_TIMEOUT_MILLIS: u64 = 1_000; /* 1 second */
const CONN_IDLE_TIMEOUT_MILLIS: u64 = 1_000; /* 1 second */

#[derive(Clone, Debug)]
pub enum Location {
    Disk(PathBuf),
    Memory,
}

impl Default for Location {
    fn default() -> Self {
        Self::Memory
    }
}

#[derive(Clone, Debug)]
pub struct Database {
    pub conn_pool: Arc<Pool<SqliteConnectionManager>>,
}

impl Database {
    pub fn new(location: Location) -> eyre::Result<Self> {
        Ok(Self {
            conn_pool: Arc::new(
                Pool::builder()
                    .connection_timeout(Duration::from_millis(
                        CONN_GET_TIMEOUT_MILLIS,
                    ))
                    .idle_timeout(Some(Duration::from_millis(
                        CONN_IDLE_TIMEOUT_MILLIS,
                    )))
                    .build(match location {
                        Location::Memory => SqliteConnectionManager::memory(),
                        Location::Disk(path) => {
                            SqliteConnectionManager::file(path)
                        }
                    })?,
            ),
        })
    }

    pub fn latest_block_hash(&self) -> eyre::Result<Option<B256>> {
        match self.conn_pool.get()?.query_row(
            "SELECT hash FROM block_headers ORDER BY inserted_at DESC LIMIT 1",
            [],
            |row| row.get::<usize, String>(0),
        ) {
            Ok(t) => Ok(Some(t.parse()?)),
            Err(e) => match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                _ => Err(eyre!(e)),
            },
        }
    }

    pub fn add_block_header(&self, header: &Header) -> eyre::Result<usize> {
        Ok(self.conn_pool.get()?.execute(
            "INSERT INTO block_headers (inserted_at, hash) VALUES (TIME('now'), ?1)",
            [header.hash.to_string()],
        )?)
    }
}
