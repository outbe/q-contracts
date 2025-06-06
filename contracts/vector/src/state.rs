use crate::types::Vector;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw_ownable::{OwnershipStore, OWNERSHIP_KEY};
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub vectors: Vec<Vector>,
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const CREATOR: OwnershipStore = OwnershipStore::new(OWNERSHIP_KEY);

/// Defined a list of vector tiers
pub fn default_vector_tiers() -> Vec<Vector> {
    vec![
        Vector {
            vector_id: 1,
            name: "Growth Vector 8%".to_string(),
            vector_rate: Uint128::new(8u128),
        },
        Vector {
            vector_id: 2,
            name: "Growth Vector 12%".to_string(),
            vector_rate: Uint128::new(12u128),
        },
        Vector {
            vector_id: 3,
            name: "Growth Vector 16%".to_string(),
            vector_rate: Uint128::new(16u128),
        },
        Vector {
            vector_id: 4,
            name: "Growth Vector 20%".to_string(),
            vector_rate: Uint128::new(20u128),
        },
        Vector {
            vector_id: 5,
            name: "Growth Vector 24%".to_string(),
            vector_rate: Uint128::new(24u128),
        },
        Vector {
            vector_id: 6,
            name: "Growth Vector 28%".to_string(),
            vector_rate: Uint128::new(28u128),
        },
        Vector {
            vector_id: 7,
            name: "Growth Vector 32%".to_string(),
            vector_rate: Uint128::new(32u128),
        },
        Vector {
            vector_id: 8,
            name: "Growth Vector 36%".to_string(),
            vector_rate: Uint128::new(36u128),
        },
        Vector {
            vector_id: 9,
            name: "Growth Vector 40%".to_string(),
            vector_rate: Uint128::new(40u128),
        },
        Vector {
            vector_id: 10,
            name: "Growth Vector 44%".to_string(),
            vector_rate: Uint128::new(44u128),
        },
        Vector {
            vector_id: 11,
            name: "Growth Vector 48%".to_string(),
            vector_rate: Uint128::new(48u128),
        },
        Vector {
            vector_id: 12,
            name: "Growth Vector 52%".to_string(),
            vector_rate: Uint128::new(52u128),
        },
        Vector {
            vector_id: 13,
            name: "Growth Vector 56%".to_string(),
            vector_rate: Uint128::new(56u128),
        },
        Vector {
            vector_id: 14,
            name: "Growth Vector 60%".to_string(),
            vector_rate: Uint128::new(60u128),
        },
        Vector {
            vector_id: 15,
            name: "Growth Vector 64%".to_string(),
            vector_rate: Uint128::new(64u128),
        },
        Vector {
            vector_id: 16,
            name: "Growth Vector 68%".to_string(),
            vector_rate: Uint128::new(68u128),
        },
        Vector {
            vector_id: 17,
            name: "Growth Vector 72%".to_string(),
            vector_rate: Uint128::new(72u128),
        },
        Vector {
            vector_id: 18,
            name: "Growth Vector 76%".to_string(),
            vector_rate: Uint128::new(76u128),
        },
        Vector {
            vector_id: 19,
            name: "Growth Vector 80%".to_string(),
            vector_rate: Uint128::new(80u128),
        },
        Vector {
            vector_id: 20,
            name: "Growth Vector 84%".to_string(),
            vector_rate: Uint128::new(84u128),
        },
        Vector {
            vector_id: 21,
            name: "Growth Vector 88%".to_string(),
            vector_rate: Uint128::new(88u128),
        },
        Vector {
            vector_id: 22,
            name: "Growth Vector 92%".to_string(),
            vector_rate: Uint128::new(92u128),
        },
        Vector {
            vector_id: 23,
            name: "Growth Vector 96%".to_string(),
            vector_rate: Uint128::new(96u128),
        },
        // + touch execution
    ]
}
