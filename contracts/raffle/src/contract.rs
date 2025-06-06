use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{
    Config, RaffleHistory, RaffleRunData, CONFIG, CREATOR, DAILY_RAFFLE, HISTORY,
    TRIBUTES_DISTRIBUTION,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Decimal, DepsMut, Env, Event, MessageInfo, Response, SubMsg, Timestamp,
    Uint128, WasmMsg,
};
use std::collections::HashSet;
use std::str::FromStr;

const CONTRACT_NAME: &str = "outbe.net:raffle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // use info.sender if None is passed
    let creator: &str = match msg.creator.as_deref() {
        Some(creator) => creator,
        None => info.sender.as_str(),
    };

    CREATOR.initialize_owner(deps.storage, deps.api, Some(creator))?;

    CONFIG.save(
        deps.storage,
        &Config {
            vector: msg.vector,
            tribute: msg.tribute,
            nod: msg.nod,
            token_allocator: msg.token_allocator,
            price_oracle: msg.price_oracle,
        },
    )?;

    Ok(Response::default()
        .add_attribute("action", "raffle::instantiate")
        .add_event(Event::new("raffle::instantiate").add_attribute("creator", creator)))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Raffle { raffle_date } => execute_raffle(deps, env, info, raffle_date),
        ExecuteMsg::BurnAll {} => execute_burn_all(deps, &env, &info),
    }
}

fn execute_raffle(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    raffle_date: Option<Timestamp>,
) -> Result<Response, ContractError> {
    let date_time = raffle_date.unwrap_or(env.block.time);
    let date = normalize_to_date(date_time).seconds();

    let raffle_run_today = DAILY_RAFFLE.may_load(deps.storage, date)?;
    let raffle_run_today = raffle_run_today.unwrap_or_default();
    let raffle_run_today = raffle_run_today + 1;

    let config = CONFIG.load(deps.storage)?;
    let tribute_address = config.tribute.ok_or(ContractError::NotInitialized {})?;
    let token_allocator_address = config
        .token_allocator
        .ok_or(ContractError::NotInitialized {})?;
    let vector_address = config.vector.ok_or(ContractError::NotInitialized {})?;
    let price_oracle_address = config
        .price_oracle
        .ok_or(ContractError::NotInitialized {})?;

    println!("Raffle dates = {} {} ", date_time, date);
    let nod_address = config.nod.ok_or(ContractError::NotInitialized {})?;

    let tributes_in_current_raffle: Vec<String> = if raffle_run_today == 1 {
        // distribute tokens
        let tributes: tribute::query::DailyTributesResponse = deps.querier.query_wasm_smart(
            &tribute_address,
            &tribute::query::QueryMsg::DailyTributes {
                date: date_time,
                status: Some(tribute::types::Status::Accepted),
            },
        )?;
        println!("Raffle tributes = {}", tributes.tributes.len());

        // query total to distribute
        let allocation_per_block: token_allocator::types::TokenAllocatorData =
            deps.querier.query_wasm_smart(
                &token_allocator_address,
                &token_allocator::query::QueryMsg::GetData {},
            )?;

        let total_allocation =
            Uint128::from(allocation_per_block.amount) * Uint128::new(24 * 60 * 12);
        let pool_allocation = total_allocation / Uint128::new(24);

        let total_interest = tributes
            .tributes
            .iter()
            .fold(Uint128::zero(), |acc, t| acc + t.data.symbolic_load);

        let mut total_deficit = (Decimal::from_str("0.08").unwrap()
            * Decimal::from_atomics(total_allocation, 0).unwrap())
        .to_uint_floor();

        if total_interest > total_allocation && total_interest - total_allocation > total_deficit {
            total_deficit = total_interest - total_allocation;
        }

        // TODO distribute deficit
        let pool_deficit = total_deficit / Uint128::new(24);

        // Pool Capacity = Pool Allocation + Pool Deficit.
        let pool_capacity = pool_allocation + pool_deficit;

        // + 8%
        println!("total_allocation = {}", total_allocation);
        println!("total_interest = {}", total_interest);
        println!("allocation_per_pool = {}", pool_allocation);
        println!("total_deficit = {}", total_deficit);
        println!("pool_deficit = {}", pool_deficit);
        println!("pool_capacity = {}", pool_capacity);

        let mut raffle_history: Vec<RaffleRunData> = vec![];
        let mut distributed_tributes: HashSet<String> = HashSet::new();
        let mut pools: Vec<Vec<String>> = Vec::with_capacity(24);
        let mut pool_index: u16 = 0;
        while pool_index < 24 {
            let mut pool_tributes: Vec<String> = vec![];
            let mut allocated_in_pool = Uint128::zero();
            for tribute in tributes.tributes.clone() {
                if allocated_in_pool >= pool_capacity {
                    break;
                }
                if !distributed_tributes.contains(&tribute.token_id) {
                    if allocated_in_pool + tribute.data.symbolic_load > pool_capacity {
                        continue;
                    }
                    allocated_in_pool += tribute.data.symbolic_load;
                    pool_tributes.push(tribute.token_id.clone());
                    distributed_tributes.insert(tribute.token_id.clone());
                }
            }
            println!(
                "Distributed in pool {:?}: {:?} tributes",
                pool_index,
                pool_tributes.len()
            );

            raffle_history.push(RaffleRunData {
                raffle_date: Timestamp::from_seconds(date),
                raffle_date_time: date_time,
                pool_index,
                total_allocation,
                pool_allocation,
                total_deficit,
                pool_deficit,
                pool_capacity,
                assigned_tributes: pool_tributes.len(),
                assigned_tributes_sum: allocated_in_pool,
            });

            pools.push(pool_tributes);
            pool_index += 1;
        }

        for (i, pool) in pools.iter().enumerate() {
            for (j, tribute_id) in pool.iter().enumerate() {
                // todo define map key for such struct
                // NB: i starts from 1 because first vector starts from 1
                let key = format!("{}_{}_{}", date, i + 1, j);
                TRIBUTES_DISTRIBUTION.save(deps.storage, &key, tribute_id)?;
                println!("added tribute {} in pool {}", tribute_id, key);
            }
        }

        // save history of the last distribution
        HISTORY.save(
            deps.storage,
            &RaffleHistory {
                data: raffle_history,
            },
        )?;

        pools.first().unwrap_or(&vec![]).clone()
    } else {
        // use already distributed tokens
        let mut result: Vec<String> = vec![];
        let mut j: usize = 0;
        loop {
            let key = format!("{}_{}_{}", date, raffle_run_today, j);
            let tribute_id = TRIBUTES_DISTRIBUTION.may_load(deps.storage, &key)?;
            match tribute_id {
                None => {
                    break;
                }
                Some(id) => result.push(id),
            }
            j += 1;
        }
        result
    };

    // mint nod
    let mut messages: Vec<SubMsg> = vec![];
    let tributes_count = tributes_in_current_raffle.len();
    println!(
        "Tributes in current run {}: {}",
        raffle_run_today, tributes_count
    );

    let tribute_info: tribute::query::TributeContractInfoResponse = deps
        .querier
        .query_wasm_smart(&tribute_address, &tribute::query::QueryMsg::ContractInfo {})?;
    let tribute_info = tribute_info.collection_config;

    let vector_info: vector::query::AllVectorsResponse = deps
        .querier
        .query_wasm_smart(&vector_address, &vector::query::QueryMsg::Vectors {})?;
    let vector = vector_info
        .vectors
        .iter()
        .find(|v| v.vector_id == raffle_run_today)
        .ok_or(ContractError::BadRunConfiguration {})?;

    let exchange_rate: price_oracle::types::TokenPairPrice = deps.querier.query_wasm_smart(
        &price_oracle_address,
        &price_oracle::query::QueryMsg::GetPrice {},
    )?;

    for tribute_id in tributes_in_current_raffle {
        let tribute: tribute::query::TributeInfoResponse = deps.querier.query_wasm_smart(
            &tribute_address,
            &tribute::query::QueryMsg::NftInfo {
                token_id: tribute_id.clone(),
            },
        )?;
        let nod_id = format!("{}_{}", tribute_id, raffle_run_today);
        let floor_price = exchange_rate.price
            * (Decimal::one() + Decimal::from_atomics(vector.vector_rate, 3).unwrap());
        println!("Nod id creation = {}", nod_id);
        let nod_mint = WasmMsg::Execute {
            contract_addr: nod_address.to_string(),
            msg: to_json_binary(&nod::msg::ExecuteMsg::Submit {
                token_id: nod_id.clone(),
                owner: tribute.owner.to_string(),
                extension: Box::new(nod::msg::SubmitExtension {
                    entity: nod::msg::NodEntity {
                        nod_id,
                        settlement_token: tribute.extension.settlement_token.clone(),
                        symbolic_rate: tribute_info.symbolic_rate,
                        nominal_minor_rate: tribute.extension.nominal_minor_qty,
                        symbolic_minor_load: tribute.extension.symbolic_load,
                        vector_minor_rate: vector.vector_rate,
                        issuance_minor_rate: exchange_rate.price,
                        floor_minor_price: floor_price,
                        state: nod::types::State::Issued,
                        address: tribute.owner.to_string(),
                    },
                    created_at: None,
                }),
            })?,
            funds: vec![],
        };
        messages.push(SubMsg::new(nod_mint));
    }

    DAILY_RAFFLE.save(deps.storage, date, &raffle_run_today)?;

    Ok(Response::new()
        .add_attribute("action", "raffle::raffle")
        .add_event(
            Event::new("raffle::raffle")
                .add_attribute("run", raffle_run_today.to_string())
                .add_attribute("tributes_count", format!("{}", tributes_count)),
        )
        .add_submessages(messages))
}

/// Normalize any timestamp to midnight UTC of that day.
fn normalize_to_date(timestamp: Timestamp) -> Timestamp {
    // 86400 seconds in a day
    let seconds = timestamp.seconds();
    let days = seconds / 86400;
    Timestamp::from_seconds(days * 86400)
}

fn execute_burn_all(
    deps: DepsMut,
    _env: &Env,
    info: &MessageInfo,
) -> Result<Response, ContractError> {
    // TODO verify ownership
    // let token = config.nft_info.load(deps.storage, &token_id)?;
    // check_can_send(deps.as_ref(), env, info.sender.as_str(), &token)?;

    HISTORY.remove(deps.storage);
    DAILY_RAFFLE.clear(deps.storage);
    TRIBUTES_DISTRIBUTION.clear(deps.storage);

    Ok(Response::new()
        .add_attribute("action", "raffle::burn_all")
        .add_event(Event::new("raffle::burn_all").add_attribute("sender", info.sender.to_string())))
}
