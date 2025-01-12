//! SQLite database interaction for storing indexed blockchain data
use std::{iter::zip, path::PathBuf, sync::Arc, time::Duration};

use alloy::{
    consensus::{
        Signed, Transaction as TraitTransaction, TxEip1559, TxEip2930,
        TxEip4844, TxEip4844Variant, TxEnvelope, TxLegacy,
    },
    hex::FromHex,
    primitives::{
        Address, BlockHash, Bytes, PrimitiveSignature, TxHash, TxKind, U256,
    },
    rpc::types::{eth::Header, Block, Transaction},
};
use eyre::{eyre, ErrReport};
use log::{debug, error};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Error, Params, Row};

const CONN_GET_TIMEOUT_MILLIS: u64 = 1_000; /* 1 second */
const CONN_IDLE_TIMEOUT_MILLIS: u64 = 1_000; /* 1 second */

/// Represents where to store a [`Database`]
#[derive(Clone, Debug)]
pub enum Location {
    /// On-disk at the given filepath
    Disk(PathBuf),
    /// In-memory (the default)
    Memory,
}

impl Default for Location {
    fn default() -> Self {
        Self::Memory
    }
}

/// Handle to the SQLite database storing indexed chain data
#[derive(Clone, Debug)]
pub struct Database {
    /// Connection pool
    pub conn_pool: Arc<Pool<SqliteConnectionManager>>,
}

impl Database {
    /// Creates a new [`Database`] instance at the given [`Location`]
    ///
    /// This will initialise the database with the necessary schema in an
    /// idempotent fashion as well as handle any (unlikely to occur) connection
    /// timeouts.
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

    /// Retrieve the block [`Header`] with the highest timestamp (if it exists)
    pub fn latest_block_header(&self) -> eyre::Result<Option<Header>> {
        match self.conn_pool.get()?.query_row(
            "SELECT * FROM block_headers ORDER BY inserted_at DESC",
            [],
            |row| Ok(Self::row_to_header(row)),
        ) {
            Ok(t) => Ok(Some(t?)),
            Err(e) => match e {
                Error::QueryReturnedNoRows => Ok(None),
                _ => Err(e.into()),
            },
        }
    }

    /// Retrieves the block [`Header`] with the given [`BlockHash`] (if it
    /// exists)
    pub fn header(&self, hash: BlockHash) -> eyre::Result<Option<Header>> {
        debug!("Block header {} requested from database...", hash);
        match self.conn_pool.get()?.query_row(
            "SELECT * FROM block_headers WHERE hash = ?",
            [hash.to_string()],
            |row| Ok(Self::row_to_header(row)),
        ) {
            Ok(t) => Ok(Some(t?)),
            Err(e) => match e {
                Error::QueryReturnedNoRows => Ok(None),
                _ => Err(e.into()),
            },
        }
    }

    /// Retrieves the block with the associated identifier (if it exists)
    pub fn block(&self, hash: BlockHash) -> eyre::Result<Option<Block>> {
        debug!("Block {} requested from database...", hash);

        match self.header(hash).inspect_err(|e| {
            error!("Failed to retrieve block header from the database: {e:?}")
        })? {
            Some(header) => Ok(Some(Block::new(header, alloy::rpc::types::BlockTransactions::Full(
                self.transactions_by_block(hash).inspect_err(|e| error!("Failed to retrieve associated transactions from the database: {e:?}"))?
            )))),
            None => Ok(None),
        }
    }

    /// Retrieves the transaction with the associated hash (if it exists)
    pub fn transaction(
        &self,
        hash: TxHash,
    ) -> eyre::Result<Option<Transaction>> {
        debug!("Transaction {} requested from database...", hash);
        match self.conn_pool.get()?.query_row(
            "SELECT * FROM transactions WHERE hash = ?",
            [hash.to_string()],
            |row| Ok(Self::row_to_transaction(row)),
        ) {
            Ok(t) => Ok(Some(t?)),
            Err(e) => match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                _ => Err(e.into()),
            },
        }
    }

    /// Retrieves all of the [`Transaction`]s associated with the [`Block`]
    /// with the given [`BlockHash`]
    ///
    /// If there are no such transactions in the database, the returned vector
    /// is guaranteed to have a length of zero.
    pub fn transactions_by_block(
        &self,
        hash: BlockHash,
    ) -> eyre::Result<Vec<Transaction>> {
        let conn = self.conn_pool.get()?;
        let mut stmt =
            conn.prepare("SELECT * FROM transactions WHERE block_hash = ?")?;
        let txs = stmt
            .query_and_then([hash.to_string()], |row| {
                Self::row_to_transaction(row)
            })?
            .collect();
        txs
    }

    /// Write a [`Transaction`] to the database
    pub fn add_transaction(
        &self,
        transaction: &Transaction,
    ) -> eyre::Result<()> {
        let tx_info = transaction.info();

        let to = match &transaction.inner {
            TxEnvelope::Legacy(t) => match t.tx().to {
                TxKind::Create => Address::ZERO,
                TxKind::Call(a) => a,
            },
            TxEnvelope::Eip2930(t) => match t.tx().to {
                TxKind::Create => Address::ZERO,
                TxKind::Call(a) => a,
            },
            TxEnvelope::Eip1559(t) => match t.tx().to {
                TxKind::Create => Address::ZERO,
                TxKind::Call(a) => a,
            },
            TxEnvelope::Eip4844(t) => match t.tx() {
                TxEip4844Variant::TxEip4844(tx_eip4844) => tx_eip4844.to,
                TxEip4844Variant::TxEip4844WithSidecar(
                    tx_eip4844_with_sidecar,
                ) => tx_eip4844_with_sidecar.tx.to,
            },
            TxEnvelope::Eip7702(t) => t.tx().to,
        };
        let tx_type: u8 = transaction.inner.tx_type().into();

        if tx_info.hash.is_none()
            || tx_info.block_hash.is_none()
            || tx_info.block_number.is_none()
            || tx_info.index.is_none()
        {
            Err(eyre!("Invalid transaction information for database"))
        } else {
            self.transact(
                "INSERT INTO transactions (
                        hash,
                        block_hash,
                        block_number,
                        position,
                        from_address,
                        type,
                        chain_id,
                        nonce,
                        gas_price,
                        gas_limit,
                        to_address,
                        value,
                        input,
                        max_fee_per_gas,
                        max_priority_fee_per_gas
                    ) VALUES(
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
                        ?15
                    )"
                .to_string(),
                params![
                    tx_info.hash.unwrap().to_string(),
                    tx_info.block_hash.unwrap().to_string(),
                    tx_info.block_number.unwrap().to_string(),
                    tx_info.index.unwrap().to_string(),
                    transaction.from.to_string(),
                    tx_type.to_string(),
                    transaction.chain_id().unwrap_or(1),
                    transaction.nonce(),
                    transaction.gas_price().unwrap_or_default() as u64,
                    transaction.gas_limit(),
                    to.to_string(),
                    transaction.value().to_string(),
                    transaction.input().to_string(),
                    transaction.max_fee_per_gas() as u64,
                    transaction.max_priority_fee_per_gas().map(|x| x as u64),
                ],
            )
        }
    }

    /// Write each transaction to the database
    pub fn add_transactions(
        &self,
        transactions: Vec<Transaction>,
    ) -> eyre::Result<()> {
        transactions
            .iter()
            .try_for_each(|tx| self.add_transaction(tx))
    }

    /// Write a [`Block`] to the database
    pub fn add_block(&self, block: &Block) -> eyre::Result<()> {
        self.add_block_header(&block.header)?;
        self.add_transactions(
            block.transactions.clone().into_transactions().collect(),
        )?;
        Ok(())
    }

    /// Write a block [`Header`] to the database
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

    fn transact_many<P>(
        &self,
        sqls: Vec<String>,
        params: Vec<P>,
    ) -> eyre::Result<()>
    where
        P: Params,
    {
        let mut conn = self.conn_pool.get()?;
        let tx = conn.transaction()?;
        {
            zip(sqls, params).try_for_each(|(st, px)| {
                let mut statement = tx.prepare(&st)?;
                statement.execute(px)?;
                Ok::<(), ErrReport>(())
            })?;
        }
        tx.commit()?;
        Ok(())
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
        self.transact_many(
            vec![
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
                "CREATE TABLE IF NOT EXISTS transactions (
                hash TEXT,
                block_hash TEXT,
                block_number INTEGER NOT NULL,
                position INTEGER NOT NULL,
                from_address TEXT,
                type INTEGER NOT NULL,

                -- Legacy
                chain_id INTEGER,
                nonce INTEGER,
                gas_price INTEGER,
                gas_limit INTEGER,
                to_address TEXT,
                value TEXT,
                input BLOB,

                -- EIP-1559
                max_fee_per_gas INTEGER,
                max_priority_fee_per_gas INTEGER
            )"
                .to_string(),
            ],
            vec![(), ()],
        )
    }

    fn row_to_transaction(row: &Row) -> eyre::Result<Transaction> {
        let hash = row.get::<&str, String>("hash")?.parse()?;
        let chain_id = row.get::<&str, u64>("chain_id")?;
        let nonce = row.get::<&str, u64>("nonce")?;
        let gas_price = row.get::<&str, u64>("gas_price")?;
        let gas_limit = row.get::<&str, u64>("gas_limit")?;
        let to: Address = row.get::<&str, String>("to_address")?.parse()?;
        let value: U256 = row.get::<&str, String>("value")?.parse()?;
        let input: Bytes = Bytes::from_hex(row.get::<&str, String>("input")?)?;

        let max_fee_per_gas = row.get::<&str, u64>("max_fee_per_gas")?;
        let max_priority_fee_per_gas =
            row.get::<&str, Option<u64>>("max_priority_fee_per_gas")?;

        let tx_type = row.get::<&str, u64>("type")?;

        let inner: TxEnvelope = match tx_type {
            0 => TxEnvelope::Legacy(Signed::new_unchecked(
                TxLegacy {
                    chain_id: Some(chain_id),
                    nonce,
                    gas_price: gas_price.into(),
                    gas_limit,
                    to: match to {
                        Address::ZERO => TxKind::Create,
                        t => TxKind::Call(t),
                    },
                    value,
                    input: input,
                },
                PrimitiveSignature::test_signature(),
                hash,
            )),
            1 => TxEnvelope::Eip2930(Signed::new_unchecked(
                TxEip2930 {
                    chain_id,
                    nonce,
                    gas_price: gas_price.into(),
                    gas_limit,
                    to: match to {
                        Address::ZERO => TxKind::Create,
                        t => TxKind::Call(t),
                    },
                    value,
                    access_list: vec![].into(), /* TODO(jmcph4): support access lists */
                    input: input,
                },
                PrimitiveSignature::test_signature(),
                hash,
            )),
            2 => TxEnvelope::Eip1559(Signed::new_unchecked(
                TxEip1559 {
                    chain_id,
                    nonce,
                    gas_limit,
                    max_fee_per_gas: max_fee_per_gas.into(),
                    max_priority_fee_per_gas: max_priority_fee_per_gas
                        .unwrap()
                        .into(),
                    to: match to {
                        Address::ZERO => TxKind::Create,
                        t => TxKind::Call(t),
                    },
                    value,
                    access_list: vec![].into(), /* TODO(jmcph4): support access lists */
                    input: input,
                },
                PrimitiveSignature::test_signature(),
                hash,
            )),
            3 => TxEnvelope::Eip4844(Signed::new_unchecked(
                TxEip4844Variant::TxEip4844(TxEip4844 {
                    chain_id,
                    nonce,
                    gas_limit,
                    max_fee_per_gas: max_fee_per_gas.into(),
                    max_priority_fee_per_gas: max_priority_fee_per_gas
                        .unwrap()
                        .into(),
                    to,
                    value,
                    access_list: vec![].into(), /* TODO(jmcph4): support access lists */
                    blob_versioned_hashes: vec![],
                    max_fee_per_blob_gas: 0,
                    input: input,
                }),
                PrimitiveSignature::test_signature(),
                hash,
            )),
            _ => return Err(eyre!("Unsupported EIP-2718 transaction type")),
        };

        Ok(Transaction {
            inner,
            block_hash: Some(row.get::<&str, String>("block_hash")?.parse()?),
            block_number: Some(row.get::<&str, u64>("block_number")?),
            transaction_index: Some(row.get::<&str, u64>("position")?),
            effective_gas_price: None, /* deprecated */
            from: row.get::<&str, String>("from_address")?.parse()?,
        })
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
