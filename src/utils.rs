//! Miscellaneous logic and types
use std::fmt;

use alloy::primitives::{Bytes, TxHash};
use url::Url;

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
                _ => Self::Local,
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
