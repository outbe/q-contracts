use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp};

#[cw_serde]
pub struct InstantiateMsg {
    pub creator: Option<String>,
    pub vector: Option<Addr>,
    pub tribute: Option<Addr>,
    pub nod: Option<Addr>,
    pub token_allocator: Option<Addr>,
    pub price_oracle: Option<Addr>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Raffle { raffle_date: Option<Timestamp> },
    // todo remove after demo
    BurnAll {},
}
