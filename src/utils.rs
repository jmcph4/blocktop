//! Miscellaneous logic and types
use std::{
    fmt,
    str::FromStr,
    time::{Duration, SystemTime},
};

use alloy::{
    consensus::Transaction as AbstractTransaction,
    primitives::{Address, Bytes, TxHash, B256, U256},
    rpc::types::Transaction,
};
use url::Url;

use crate::ADDRESS_LABELS;

const HASH_TRUNCATION_LEN: usize = 8;
const ADDRESS_HEAD_TAIL_LEN: usize = 4;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum BuilderNetIdentity {
    Flashbots,
    Nethermind,
    Beaver,
}

impl fmt::Display for BuilderNetIdentity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Flashbots => write!(f, "Flashbots"),
            Self::Nethermind => write!(f, "Nethermind"),
            Self::Beaver => write!(f, "Beaver"),
        }
    }
}

impl FromStr for BuilderNetIdentity {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BuilderNet (Flashbots)" | "Illuminate Dmocrtz Dstrib Prtct" => {
                Ok(Self::Flashbots)
            }
            "BuilderNet (Nethermind)" => Ok(Self::Nethermind),
            "BuilderNet (Beaverbuild)" => Ok(Self::Beaver),
            _ => Err("Unknown BuilderNet operator"),
        }
    }
}

/// Represents the (public) identity of known block builders on Ethereum mainnet
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum BuilderIdentity {
    Beaver,
    Titan,
    Rsync,
    Penguin,
    Flashbots,
    Nethermind,
    Jet,
    Loki,
    SixtyNine,
    BuildAI,
    Beelder,
    Blocksmith,
    Bob,
    Boba,
    Manifold,
    Bitget,
    Btcs,
    Local,
    BuilderNet(BuilderNetIdentity),
}

impl fmt::Display for BuilderIdentity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Beaver => write!(f, "beaverbuild"),
            Self::Titan => write!(f, "Titan Builder"),
            Self::Rsync => write!(f, "rsync-builder"),
            Self::Penguin => write!(f, "penguinbuild.org"),
            Self::Flashbots => write!(f, "Flashbots"),
            Self::Nethermind => write!(f, "Nethermind"),
            Self::Jet => write!(f, "JetBuilder"),
            Self::Loki => write!(f, "Loki Builder"),
            Self::SixtyNine => write!(f, "Builder0x69"),
            Self::BuildAI => write!(f, "BuildAI"),
            Self::Beelder => write!(f, "beelder.eth"),
            Self::Blocksmith => write!(f, "Blocksmith"),
            Self::Bob => write!(f, "Bob The Builder"),
            Self::Boba => write!(f, "Boba Builder"),
            Self::Manifold => write!(f, "Manifold"),
            Self::Bitget => write!(f, "Bitget"),
            Self::Btcs => write!(f, "Builder+"),
            Self::Local => write!(f, "<local>"),
            Self::BuilderNet(t) => write!(f, "BuilderNet - {}", t),
        }
    }
}

impl From<Vec<u8>> for BuilderIdentity {
    fn from(value: Vec<u8>) -> Self {
        if let Ok(s) = String::from_utf8(value) {
            match s.as_str() {
                "beaverbuild.org" => Self::Beaver,
                "Titan (titanbuilder.xyz)" => Self::Titan,
                "@rsyncbuilder" | "rsync-builder.xyz" => Self::Rsync,
                "Illuminate Dmocratize Dstribute"
                | "Illuminate Dmocrtz Dstrib Prtct" => Self::Flashbots,
                "penguinbuild.org" | "@penguinbuild.org"
                | "@@penguinbuild.org" => Self::Penguin,
                "Nethermind" => Self::Nethermind,
                "jetbldr.xyz" => Self::Jet,
                "lokibuilder.xyz" => Self::Loki,
                "builder0x69" | "by builder0x69" | "by @builder0x69" => {
                    Self::SixtyNine
                }
                "BuildAI (https://buildai.net)" => Self::BuildAI,
                "https://blockbeelder.com 🐝" => Self::Beelder,
                "blocksmith.org" => Self::Blocksmith,
                "bobTheBuilder.xyz" => Self::Bob,
                "boba-builder.com" => Self::Boba,
                "Manifold: coinbase" => Self::Manifold,
                "Bitget(https://www.bitget.com/)" => Self::Bitget,
                "Builder+ www.btcs.com/builder" => Self::Btcs,
                s => {
                    if let Ok(op) = BuilderNetIdentity::from_str(s) {
                        Self::BuilderNet(op)
                    } else {
                        Self::Local
                    }
                }
            }
        } else {
            Self::Local
        }
    }
}

impl From<Bytes> for BuilderIdentity {
    fn from(value: Bytes) -> Self {
        value.to_vec().into()
    }
}

/// Given a block number, produce the Etherscan [`Url`] for the corresponding
/// block
pub fn etherscan_block_url(block_number: u64) -> Url {
    format!("https://etherscan.io/block/{block_number}")
        .parse()
        .expect("invariant violated: constructed invalid block URL")
}

/// Given a [`TxHash`], produce the Etherscan [`Url`] for the corresponding
/// transaction
pub fn etherscan_transaction_url(transaction_hash: TxHash) -> Url {
    format!("https://etherscan.io/tx/{transaction_hash}")
        .parse()
        .expect("invariant violated: constructed invalid transaction URL")
}

pub fn shorten_hash(hash: &B256) -> String {
    format!("{}...", &hash.to_string()[0..HASH_TRUNCATION_LEN])
}

pub fn shorten_address(address: &Address) -> String {
    let s = address.to_string();
    format!(
        "{}...{}",
        &s[0..ADDRESS_HEAD_TAIL_LEN + 2],
        &s[s.len().saturating_sub(ADDRESS_HEAD_TAIL_LEN)..]
    )
}

pub fn duration_since_timestamp(timestamp: u64) -> Duration {
    let now = SystemTime::now();
    let unix_epoch = SystemTime::UNIX_EPOCH;
    let timestamp_time = unix_epoch + Duration::from_secs(timestamp);
    now.duration_since(timestamp_time).unwrap()
}

pub fn human_readable_tx_data(data: Bytes) -> String {
    let buflen = data.len();

    if buflen == 0 {
        "∅".to_string()
    } else if buflen > 10 {
        format!("{}...", &data.to_string()[0..10])
    } else {
        data.to_string().to_string()
    }
}

#[inline]
pub fn to_gwei(x: f64) -> f64 {
    x / f64::powi(10.0, 9)
}

#[inline]
pub fn to_ether(x: U256) -> f64 {
    if x > U256::from(u128::MAX) {
        todo!()
    } else {
        u128::from_be_bytes(
            x.to_be_bytes_vec()[0..((u128::BITS / 8) as usize)]
                .try_into()
                .expect(
                    "invariant violated: U256 must have enough bytes for u128",
                ),
        ) as f64
    }
}

#[inline]
pub fn useful_gas_price(tx: &Transaction) -> u128 {
    tx.max_fee_per_gas()
}

pub fn grab_range(xs: &Bytes, a: usize, b: usize) -> Bytes {
    if a >= xs.len() {
        Bytes::from(vec![])
    } else if b > xs.len() {
        Bytes::from(xs[a..xs.len()].to_vec())
    } else {
        Bytes::from(xs[a..b].to_vec())
    }
}

const MAX_ADDR_LEN: usize = 32;

pub fn label_address(address: &Address) -> String {
    if let Some(label) = ADDRESS_LABELS.get(address) {
        if label.len() > MAX_ADDR_LEN {
            (label[0..MAX_ADDR_LEN]).to_string()
        } else {
            label.clone()
        }
    } else {
        shorten_address(address)
    }
}
