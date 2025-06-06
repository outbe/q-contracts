use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct Vector {
    /// Vector identifier
    pub vector_id: u16,
    /// Name or label of the vector tier
    pub name: String,
    /// TPT+%
    pub vector_rate: Uint128,
}
