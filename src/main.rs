use std::collections::HashMap;

use alloy::primitives::Address;
use clap::Parser;
use client::{AnyClient, Client};
use log::warn;
use serde::Deserialize;

use crate::{
    cli::Opts,
    db::{Database, Location},
    services::blockchain::BlockchainService,
    ui::run,
};

pub mod cli;
pub mod client;
pub mod db;
pub mod services;
pub mod ui;
pub mod utils;

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
struct LabelEntry {
    pub address: Address,
    #[serde(rename = "chainId")]
    chain_id: u64,
    label: String,
    #[serde(rename = "nameTag")]
    pub name_tag: Option<String>,
}

const LABELS_JSON_DATA: &str = include_str!("../assets/labels/mainnet.json");

lazy_static::lazy_static! {
    static ref ADDRESS_LABELS: HashMap<Address, String> = {
        let labels: Vec<LabelEntry> = serde_json::from_str(LABELS_JSON_DATA).expect("Invalid JSON data for address labels");
        labels.iter().filter(|label| label.name_tag.is_some()).map(|label| (label.address, label.name_tag.clone().unwrap())).collect()
    };
}

/// Retrieve an initial block from the endpoint so that upon UI startup there's data to render
#[allow(clippy::needless_question_mark)] /* clippy gets this wrong */
async fn populate_db(opts: &Opts, db: &mut Database) -> eyre::Result<()> {
    let rpc = opts.rpc.clone();
    let perhaps_block = opts.block;
    let perhaps_tx = opts.transaction;
    let client = AnyClient::new(rpc).await?;

    match (perhaps_block, perhaps_tx) {
        (Some(block), None) => {
            Ok(db.add_block(&client.block(block.into()).await?)?)
        }
        (None, Some(tx_hash)) => {
            let tx = client.transaction(tx_hash).await?;
            /* recall that we *must* have at least one *block* in the db at all times */
            db.add_block(&client.block(tx.block_hash.unwrap().into()).await?)?;
            Ok(())
        }
        _ => Ok(db.add_block(
            &client
                .block(alloy::eips::BlockNumberOrTag::Latest.into())
                .await?,
        )?),
    }
}

fn main() -> eyre::Result<()> {
    let opts: Opts = Opts::parse();

    if opts.headless {
        pretty_env_logger::init_timed();
    }

    if opts.headless && opts.db.is_none() {
        warn!("Headless mode without specifying an on-disk database. All data will be lost on exit.");
    }

    let mut db: Database = Database::new(match opts.db {
        Some(ref file) => Location::Disk(file.to_path_buf()),
        None => Location::Memory,
    })?;

    if opts.list_block_hashes {
        db.all_block_hashes()?
            .iter()
            .for_each(|hash| println!("{hash}"));
    }

    /* wet the database */
    tokio::task::block_in_place(|| {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { populate_db(&opts, &mut db).await })
    })?;

    let blockchain = BlockchainService::spawn(opts.rpc, db.clone());

    if !opts.headless {
        let terminal = ratatui::init();
        let result = run(terminal, &db, opts.block, opts.transaction);
        ratatui::restore();
        result
    } else {
        let _ = blockchain.join();
        Ok(())
    }
}
