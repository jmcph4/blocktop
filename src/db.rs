//! SQLite database interaction for storing indexed blockchain data
use std::{iter::zip, path::PathBuf, str::FromStr, sync::Arc, time::Duration};

use alloy::{
    consensus::{
        Signed, Transaction as TraitTransaction, TxEip1559, TxEip2930,
        TxEip4844, TxEip4844Variant, TxEnvelope, TxLegacy,
    },
    eips::{BlockId, BlockNumberOrTag},
    hex::{FromHex, FromHexError},
    primitives::{
        Address, BlockHash, BlockNumber, Bytes, PrimitiveSignature, TxHash,
        TxKind, U256,
    },
    rpc::types::{eth::Header, Block, Transaction},
};
use eyre::{eyre, ErrReport};
use log::{debug, error, info};
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
            "SELECT * FROM block_headers ORDER BY number DESC",
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

    /// Retrieve the [`Block`] with the highest timestamp (if it exists)
    pub fn latest_block(&self) -> eyre::Result<Option<Block>> {
        match self.latest_block_header()? {
            Some(latest_header) => self.block_by_hash(latest_header.hash),
            None => Ok(None),
        }
    }

    /// Retrieves the block [`Header`] with the given [`BlockHash`] (if it
    /// exists)
    pub fn header_by_hash(
        &self,
        hash: BlockHash,
    ) -> eyre::Result<Option<Header>> {
        debug!(
            "Block header {} requested from database...",
            hash.to_string()
        );
        match self.conn_pool.get()?.query_row(
            format!("SELECT * FROM block_headers WHERE hash = '{}'", hash)
                .as_str(),
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

    /// Retrieves the block [`Header`] with the given [`BlockNumber`] (if it
    /// exists)
    pub fn header_by_number(
        &self,
        number: BlockNumber,
    ) -> eyre::Result<Option<Header>> {
        debug!("Block header #{} requested from database...", number,);
        match self.conn_pool.get()?.query_row(
            format!("SELECT * FROM block_headers WHERE number = '{}'", number)
                .as_str(),
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

    /// Retrieves the block with the associated hash (if it exists)
    pub fn block_by_hash(
        &self,
        hash: BlockHash,
    ) -> eyre::Result<Option<Block>> {
        debug!("Block {} requested from database...", hash);

        match self.header_by_hash(hash).inspect_err(|e| {
            error!("Failed to retrieve block header from the database: {e:?}")
        })? {
            Some(header) => Ok(Some(Block::new(header, alloy::rpc::types::BlockTransactions::Full(
                self.transactions_by_block_hash(hash).inspect_err(|e| error!("Failed to retrieve associated transactions from the database: {e:?}"))?
            )))),
            None => Ok(None),
        }
    }

    /// Retrieves the block with the associated number (if it exists)
    pub fn block_by_number(
        &self,
        number: BlockNumber,
    ) -> eyre::Result<Option<Block>> {
        debug!("Block #{} requested from database...", number);

        match self.header_by_number(number).inspect_err(|e| {
            error!("Failed to retrieve block header from the database: {e:?}")
        })? {
            Some(header) => Ok(Some(Block::new(header, alloy::rpc::types::BlockTransactions::Full(
                self.transactions_by_block_number(number).inspect_err(|e| error!("Failed to retrieve associated transactions from the database: {e:?}"))?
            )))),
            None => Ok(None),
        }
    }

    /// Retrieves the [`Block`] matching the given [`BlockId`] (if it exists)
    pub fn block(&self, id: BlockId) -> eyre::Result<Option<Block>> {
        match id {
            BlockId::Hash(h) => self.block_by_hash(h.into()),
            BlockId::Number(t) => match t {
                BlockNumberOrTag::Number(n) => self.block_by_number(n),
                BlockNumberOrTag::Latest => self.latest_block(),
                _ => unimplemented!(),
            },
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

    pub fn all_block_hashes(&self) -> eyre::Result<Vec<BlockHash>> {
        let conn = self.conn_pool.get()?;
        let mut stmt = conn.prepare("SELECT hash FROM block_headers")?;
        let hash_strings: Vec<String> = stmt
            .query_and_then([], |row| row.get::<&str, String>("hash"))?
            .collect::<Result<Vec<String>, rusqlite::Error>>()?;
        let hashes: Vec<BlockHash> = hash_strings
            .iter()
            .map(|s| s.parse())
            .collect::<Result<Vec<BlockHash>, FromHexError>>(
        )?;
        Ok(hashes)
    }

    /// Retrieves all of the [`Transaction`]s associated with the [`Block`]
    /// with the given [`BlockHash`]
    ///
    /// If there are no such transactions in the database, the returned vector
    /// is guaranteed to have a length of zero.
    #[allow(clippy::let_and_return)] /* clippy gets this wrong */
    pub fn transactions_by_block_hash(
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

    /// Retrieves all of the [`Transaction`]s associated with the [`Block`]
    /// with the given [`BlockNumber`]
    ///
    /// If there are no such transactions in the database, the returned vector
    /// is guaranteed to have a length of zero.
    #[allow(clippy::let_and_return)] /* clippy gets this wrong */
    pub fn transactions_by_block_number(
        &self,
        number: BlockNumber,
    ) -> eyre::Result<Vec<Transaction>> {
        let conn = self.conn_pool.get()?;
        let mut get_hash_stmt =
            conn.prepare("SELECT hash FROM block_headers WHERE number = ?")?;
        let hash: BlockHash = get_hash_stmt
            .query_and_then([number], |row| {
                Ok::<BlockHash, ErrReport>(BlockHash::from_str(
                    row.get::<usize, String>(0)?.as_str(),
                )?)
            })?
            .next()
            .unwrap()?;
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
        info!("Wrote block {} to the database", block.header.hash);
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
                    ommers_hash,
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
        )?;
        debug!("Wrote block header {} to the database", header.hash);
        Ok(())
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
            ommers_hash STRING,
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
                    input,
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
                    input,
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
                    input,
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
                    input,
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
        let mut header = Header::new(alloy::consensus::Header {
            parent_hash: row.get::<&str, String>("parent_hash")?.parse()?,
            ommers_hash: row.get::<&str, String>("ommers_hash")?.parse()?,
            beneficiary: row.get::<&str, String>("beneficiary")?.parse()?,
            state_root: row.get::<&str, String>("state_root")?.parse()?,
            transactions_root: row
                .get::<&str, String>("transactions_root")?
                .parse()?,
            receipts_root: row.get::<&str, String>("receipts_root")?.parse()?,
            logs_bloom: row.get::<&str, String>("logs_bloom")?.parse()?,
            difficulty: U256::from(row.get::<&str, u64>("difficulty")?),
            number: row.get::<&str, u64>("number")?,
            gas_limit: row.get::<&str, u64>("gas_limit")?,
            gas_used: row.get::<&str, u64>("gas_used")?,
            timestamp: row.get::<&str, u64>("timestamp")?,
            extra_data: row.get::<&str, Vec<u8>>("extra_data")?.into(),
            mix_hash: row.get::<&str, String>("mix_hash")?.parse()?,
            nonce: row.get::<&str, String>("nonce")?.parse()?,
            base_fee_per_gas: match row.get::<&str, u64>("base_fee_per_gas")? {
                0 => None,
                x => Some(x),
            },
            withdrawals_root: match row
                .get::<&str, String>("withdrawals_root")?
                .as_str()
            {
                "" => None,
                x => Some(x.parse()?),
            },
            blob_gas_used: match row.get::<&str, u64>("blob_gas_used")? {
                0 => None,
                x => Some(x),
            },
            excess_blob_gas: match row.get::<&str, u64>("excess_blob_gas")? {
                0 => None,
                x => Some(x),
            },
            parent_beacon_block_root: match row
                .get::<&str, String>("parent_beacon_block_root")?
                .as_str()
            {
                "" => None,
                x => Some(x.parse()?),
            },
            requests_hash: match row
                .get::<&str, String>("requests_hash")?
                .as_str()
            {
                "" => None,
                x => Some(x.parse()?),
            },
        });
        header.hash = row.get::<&str, String>("hash")?.parse()?;
        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latest_block() {
        let block = Block::default();
        let creation_result = Database::new(Location::Memory);
        assert!(creation_result.is_ok());
        let db = creation_result.unwrap();
        let insertion_result = db.add_block(&block);
        assert!(insertion_result.is_ok());
        let retrieval_result = db.latest_block();
        assert!(retrieval_result.is_ok());
        let perhaps_latest_block = retrieval_result.unwrap();
        assert!(perhaps_latest_block.is_some());
    }

    #[test]
    fn test_latest_block_header() {
        let header = Header::default();
        let creation_result = Database::new(Location::Memory);
        assert!(creation_result.is_ok());
        let db = creation_result.unwrap();
        let insertion_result = db.add_block_header(&header);
        assert!(insertion_result.is_ok());
        let retrieval_result = db.latest_block_header();
        assert!(retrieval_result.is_ok());
        let perhaps_latest_header = retrieval_result.unwrap();
        assert!(perhaps_latest_header.is_some());
    }
}
