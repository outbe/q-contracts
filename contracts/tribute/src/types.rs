use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, HexBinary, Timestamp, Uint128};
use cw20::Denom;
use outbe_nft::state::NftInfo;
use outbe_nft::traits::Cw721CollectionConfig;

/// ConsumptionUnit contract config
#[cw_serde]
pub struct TributeConfig {
    pub symbolic_rate: Decimal,
    pub native_token: Denom,
    pub price_oracle: Addr,
}

impl Cw721CollectionConfig for TributeConfig {}

/// ConsumptionUnit public data
#[cw_serde]
pub struct TributeData {
    /// Value of the Tribute in Settlement Tokens
    pub settlement_value: Uint128,
    /// Tribute settlement token
    pub settlement_token: Denom,
    /// Value of the Tribute in Native Coins
    pub nominal_qty: Uint128,
    /// Price in Native coins with a rate on the moment of the transaction
    pub nominal_price: Decimal,
    /// Signals an eligible interest to the network Gratis qty
    pub symbolic_load: Uint128,
    /// Hashes identifying consumption records batch. Each hash should be a valid unique
    /// sha256 hash in hex format
    pub hashes: Vec<HexBinary>,
    pub tribute_date: Timestamp,
    pub fidelity_index: i32,
    /// Time when the Tribute NFT was created on the network
    pub created_at: Timestamp,
    /// Last updated time
    pub updated_at: Timestamp,
}

pub type TributeNft = NftInfo<TributeData>;

impl outbe_nft::traits::Cw721State for TributeData {}
impl outbe_nft::traits::Cw721CustomMsg for TributeData {}
