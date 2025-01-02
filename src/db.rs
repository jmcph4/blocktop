use std::{path::PathBuf, sync::Arc, time::Duration};

use alloy::{
    primitives::{BlockNumber, B256},
    rpc::types::eth::Header,
};
use eyre::eyre;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Params};

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
        let mut this = Self {
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
        };
        this.initialise()?;
        Ok(this)
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

    pub fn latest_block_number(&self) -> eyre::Result<Option<BlockNumber>> {
        match self.conn_pool.get()?.query_row(
            "SELECT number FROM block_headers ORDER BY inserted_at DESC LIMIT 1",
            [],
            |row| row.get::<usize, u64>(0),
        ) {
            Ok(t) => Ok(Some(t)),
            Err(e) => match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                _ => Err(eyre!(e)),
            },
        }
    }

    pub fn add_block_header(&self, header: &Header) -> eyre::Result<()> {
        self.transact(
            "INSERT INTO block_headers (
                    inserted_at,
                    hash,
                    number,
                    parent_hash,
                    ommer_hash,
                    beneficiary,
                    state_root,
                    transactions_root,
                    receipts_root,
                    logs_bloom,
                    difficulty,
                    gas_limit,
                    gas_used,
                    timestamp,
                    extra_data,
                    mix_hash,
                    nonce,
                    base_fee_per_gas,
                    withdrawals_root,
                    blob_gas_used,
                    excess_blob_gas,
                    parent_beacon_block_root,
                    requests_hash,
                    size
                ) VALUES (
                    TIME('now'),
                    ?1,
                    ?2,
                    ?3,
                    ?4,
                    ?5,
                    ?6,
                    ?7,
                    ?8,
                    ?9,
                    ?10,
                    ?11,
                    ?12,
                    ?13,
                    ?14,
                    ?15,
                    ?16,
                    ?17,
                    ?18,
                    ?19,
                    ?20,
                    ?21,
                    ?22,
                    ?23
                )"
            .to_string(),
            params![
                header.hash.to_string(),
                header.number.to_string(),
                header.parent_hash.to_string(),
                header.ommers_hash.to_string(),
                header.beneficiary.to_string(),
                header.state_root.to_string(),
                header.transactions_root.to_string(),
                header.receipts_root.to_string(),
                header.logs_bloom.to_string(),
                header.difficulty.to_string(),
                header.gas_limit.to_string(),
                header.gas_used.to_string(),
                header.timestamp.to_string(),
                header.extra_data.to_string(),
                header.mix_hash.to_string(),
                header.nonce.to_string(),
                header.base_fee_per_gas.unwrap_or_default(),
                header.withdrawals_root.unwrap_or_default().to_string(),
                header.blob_gas_used.unwrap_or_default().to_string(),
                header.excess_blob_gas.unwrap_or_default().to_string(),
                header
                    .parent_beacon_block_root
                    .unwrap_or_default()
                    .to_string(),
                header.requests_hash.unwrap_or_default().to_string(),
                header.size.unwrap_or_default().to_string(),
            ],
        )
    }

    fn transact<P>(&self, sql: String, params: P) -> eyre::Result<()>
    where
        P: Params,
    {
        let mut conn = self.conn_pool.get()?;
        let tx = conn.transaction()?;
        {
            let mut statement = tx.prepare(&sql)?;
            statement.execute(params)?;
        }
        tx.commit()?;
        Ok(())
    }

    fn initialise(&mut self) -> eyre::Result<()> {
        self.transact(
            "CREATE TABLE IF NOT EXISTS block_headers (
            inserted_at TIMESTAMP,
            hash STRING,
            number INTEGER,
            parent_hash STRING,
            ommer_hash STRING,
            beneficiary STRING,
            state_root STRING,
            transactions_root STRING,
            receipts_root STRING,
            logs_bloom STRING,
            difficulty INTEGER,
            gas_limit INTEGER,
            gas_used INTEGER,
            timestamp TIMESTAMP,
            extra_data BLOB,
            mix_hash STRING,
            nonce INTEGER,
            base_fee_per_gas INTEGER,
            withdrawals_root STRING,
            blob_gas_used INTEGER,
            excess_blob_gas INTEGER,
            parent_beacon_block_root STRING,
            requests_hash INTEGER,
            size INTEGER
        )"
            .to_string(),
            (),
        )
    }
}
