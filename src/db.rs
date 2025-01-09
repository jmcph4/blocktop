use std::{path::PathBuf, sync::Arc, time::Duration};

use alloy::{
    eips::BlockNumberOrTag,
    primitives::{BlockNumber, B256, U256},
    rpc::types::{eth::Header, Block, Transaction},
};
use eyre::eyre;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Params, Row};

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

    pub fn latest_block_header(&self) -> eyre::Result<Option<Header>> {
        match self.conn_pool.get()?.query_row(
            "SELECT * FROM block_headers ORDER BY inserted_at DESC LIMIT 1",
            [],
            |row| Ok(Self::row_to_header(row)),
        ) {
            Ok(t) => Ok(Some(t?)),
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

    pub fn block(&self, _tag: BlockNumberOrTag) -> Option<Block> {
        self.latest_block() /* TODO(jmcph4): placeholder */
    }

    pub fn latest_block(&self) -> Option<Block> {
        self.latest_block_header().ok().flatten().map(|header| {
            Block::new(
                header,
                alloy::rpc::types::BlockTransactions::Full(vec![]),
            )
        }) /* TODO(jmcph4): placeholder */
    }

    pub fn add_transaction(
        &self,
        transaction: &Transaction,
    ) -> eyre::Result<()> {
        todo!()
    }

    pub fn add_transactions(
        &self,
        transactions: Vec<Transaction>,
    ) -> eyre::Result<()> {
        transactions
            .iter()
            .try_for_each(|tx| self.add_transaction(tx))
    }

    pub fn add_block(&self, block: &Block) -> eyre::Result<()> {
        self.add_block_header(&block.header)?;
        self.add_transactions(
            block.transactions.clone().into_transactions().collect(),
        )?;
        Ok(())
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
                    requests_hash
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
                    ?22
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
                header.extra_data.to_vec(),
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
            requests_hash INTEGER
        )"
            .to_string(),
            (),
        )
    }

    fn row_to_header(row: &Row) -> eyre::Result<Header> {
        Ok(Header::new(alloy::consensus::Header {
            parent_hash: row.get::<usize, String>(3)?.parse()?,
            ommers_hash: row.get::<usize, String>(4)?.parse()?,
            beneficiary: row.get::<usize, String>(5)?.parse()?,
            state_root: row.get::<usize, String>(6)?.parse()?,
            transactions_root: row.get::<usize, String>(7)?.parse()?,
            receipts_root: row.get::<usize, String>(8)?.parse()?,
            logs_bloom: row.get::<usize, String>(9)?.parse()?,
            difficulty: U256::from(row.get::<usize, u64>(10)?),
            number: row.get::<usize, u64>(2)?,
            gas_limit: row.get::<usize, u64>(11)?,
            gas_used: row.get::<usize, u64>(12)?,
            timestamp: row.get::<usize, u64>(13)?,
            extra_data: row.get::<usize, Vec<u8>>(14)?.into(),
            mix_hash: row.get::<usize, String>(15)?.parse()?,
            nonce: row.get::<usize, String>(16)?.parse()?,
            base_fee_per_gas: match row.get::<usize, u64>(17)? {
                0 => None,
                x => Some(x),
            },
            withdrawals_root: match row.get::<usize, String>(18)?.as_str() {
                "" => None,
                x => Some(x.parse()?),
            },
            blob_gas_used: match row.get::<usize, u64>(19)? {
                0 => None,
                x => Some(x),
            },
            excess_blob_gas: match row.get::<usize, u64>(20)? {
                0 => None,
                x => Some(x),
            },
            parent_beacon_block_root: match row
                .get::<usize, String>(21)?
                .as_str()
            {
                "" => None,
                x => Some(x.parse()?),
            },
            requests_hash: match row.get::<usize, String>(22)?.as_str() {
                "" => None,
                x => Some(x.parse()?),
            },
        }))
    }
}
