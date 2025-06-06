use crate::state::{CONFIG, CREATOR};
use crate::types::Vector;
use cosmwasm_schema::{cw_serde, QueryResponses};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Addr, Binary, Deps, Env, StdResult, Storage};
use cw_ownable::Ownership;

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns all vectors
    #[returns(AllVectorsResponse)]
    Vectors {},

    #[returns(cw_ownable::Ownership<String>)]
    GetCreatorOwnership {},
}

#[cw_serde]
pub struct AllVectorsResponse {
    pub vectors: Vec<Vector>,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Vectors {} => to_json_binary(&query_vectors(deps.storage)?),
        QueryMsg::GetCreatorOwnership {} => to_json_binary(&query_creator_ownership(deps.storage)?),
    }
}

// Query
pub fn query_vectors(storage: &dyn Storage) -> StdResult<AllVectorsResponse> {
    let config = CONFIG.load(storage)?;
    Ok(AllVectorsResponse {
        vectors: config.vectors,
    })
}

pub fn query_creator_ownership(storage: &dyn Storage) -> StdResult<Ownership<Addr>> {
    CREATOR.get_ownership(storage)
}

#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate};
    use crate::msg::InstantiateMsg;
    use crate::query::{query, AllVectorsResponse, QueryMsg};
    use cw_multi_test::{App, ContractWrapper, Executor};

    #[test]
    fn test_query_tiers() {
        let mut app = App::default();
        let owner = app.api().addr_make("owner");

        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let init_msg = InstantiateMsg {
            vectors: None,
            creator: None,
        };

        let contract_addr = app
            .instantiate_contract(code_id, owner.clone(), &init_msg, &[], "tiers1", None)
            .unwrap();

        let response: AllVectorsResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::Vectors {})
            .unwrap();
        assert_eq!(response.vectors.len(), 23);
        assert_eq!(response.vectors.first().unwrap().vector_id, 1);
        assert_eq!(response.vectors.last().unwrap().vector_id, 23);
    }
}
