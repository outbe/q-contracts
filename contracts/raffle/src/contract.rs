use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{
    Config, RaffleHistory, RaffleRunData, CONFIG, CREATOR, DAILY_RAFFLE, DAILY_TOUCH, HISTORY,
    TRIBUTES_DISTRIBUTION,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Decimal, Deps, DepsMut, Env, Event, MessageInfo, Response, SubMsg,
    Timestamp, Uint128, WasmMsg,
};
use outbe_utils::time_utils;
use price_oracle::types::DayType;
use std::collections::HashSet;
use tribute::query::FullTributeData;

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
            deficit: msg.deficit,
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
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    raffle_date: Option<Timestamp>,
) -> Result<Response, ContractError> {
    let execution_date_time = raffle_date.unwrap_or(env.block.time);
    println!("Raffle execution date = {}", execution_date_time);

    let config = CONFIG.load(deps.storage)?;
    let tribute_address = config.tribute.ok_or(ContractError::NotInitialized {})?;
    let token_allocator_address = config
        .token_allocator
        .ok_or(ContractError::NotInitialized {})?;
    let vector_address = config.vector.ok_or(ContractError::NotInitialized {})?;
    let price_oracle_address = config
        .price_oracle
        .ok_or(ContractError::NotInitialized {})?;
    let nod_address = config.nod.ok_or(ContractError::NotInitialized {})?;

    let exchange_rate: price_oracle::types::TokenPairPrice = deps.querier.query_wasm_smart(
        &price_oracle_address,
        &price_oracle::query::QueryMsg::GetPrice {},
    )?;

    let (total_allocation, allocation_per_tier) =
        calc_allocation(deps.as_ref(), token_allocator_address)?;
    println!(
        "total_allocation = {}, allocation_per_pool = {}, ",
        total_allocation, allocation_per_tier
    );

    match exchange_rate.day_type {
        DayType::GREEN => execute_raffle_tier(
            deps.branch(),
            total_allocation,
            allocation_per_tier,
            execution_date_time,
            tribute_address,
            nod_address,
            vector_address,
            exchange_rate.price,
            config.deficit,
        ),
        DayType::RED => {
            // We have only touch if day is red
            let all_tributes: tribute::query::DailyTributesResponse =
                deps.querier.query_wasm_smart(
                    &tribute_address,
                    &tribute::query::QueryMsg::DailyTributes {
                        date: execution_date_time,
                    },
                )?;
            let all_tributes = all_tributes.tributes;
            println!("Raffle tributes = {}", all_tributes.len());

            execute_touch(
                deps.branch(),
                all_tributes.clone(),
                allocation_per_tier,
                execution_date_time,
                nod_address,
                exchange_rate.price,
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_raffle_tier(
    mut deps: DepsMut,
    total_allocation: Uint128,
    allocation_per_tier: Uint128,
    execution_date_time: Timestamp,
    tribute_address: Addr,
    nod_address: Addr,
    vector_address: Addr,
    exchange_rate: Decimal,
    deficit: Decimal,
) -> Result<Response, ContractError> {
    let date = time_utils::normalize_to_date(execution_date_time).seconds();

    let raffle_run_today = DAILY_RAFFLE.may_load(deps.storage, date)?;
    let raffle_run_today = raffle_run_today.unwrap_or_default();
    let raffle_run_today = raffle_run_today + 1;

    match raffle_run_today {
        1 => {
            let tributes_in_first_tier = distribute_tributes_for_tiers(
                deps.branch(),
                total_allocation,
                allocation_per_tier,
                execution_date_time,
                date,
                deficit,
                tribute_address.clone(),
            )?;
            do_raffle_in_tier(
                deps,
                tributes_in_first_tier,
                1,
                date,
                tribute_address,
                vector_address,
                nod_address,
                exchange_rate,
            )
        }
        2..=23 => {
            // use already distributed tokens
            let mut tributes_in_first_tier: Vec<String> = vec![];
            let mut j: usize = 0;
            loop {
                let key = format!("{}_{}_{}", date, raffle_run_today, j);
                let tribute_id = TRIBUTES_DISTRIBUTION.may_load(deps.storage, &key)?;
                match tribute_id {
                    None => {
                        break;
                    }
                    Some(id) => tributes_in_first_tier.push(id),
                }
                j += 1;
            }

            do_raffle_in_tier(
                deps,
                tributes_in_first_tier,
                raffle_run_today,
                date,
                tribute_address,
                vector_address,
                nod_address,
                exchange_rate,
            )
        }
        24 => {
            let tributes_for_raffle: Vec<FullTributeData> = vec![];
            // TODO add tributes query that were not selected for raffle
            execute_touch(
                deps.branch(),
                tributes_for_raffle,
                allocation_per_tier,
                execution_date_time,
                nod_address,
                exchange_rate,
            )
        }
        _ => Err(ContractError::BadRunConfiguration {}),
    }
}

#[allow(clippy::too_many_arguments)]
fn do_raffle_in_tier(
    deps: DepsMut,
    tributes_in_current_raffle: Vec<String>,
    raffle_run_today: u16,
    date: u64,
    tribute_address: Addr,
    vector_address: Addr,
    nod_address: Addr,
    exchange_rate: Decimal,
) -> Result<Response, ContractError> {
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

    for tribute_id in tributes_in_current_raffle {
        let tribute: tribute::query::TributeInfoResponse = deps.querier.query_wasm_smart(
            &tribute_address,
            &tribute::query::QueryMsg::NftInfo {
                token_id: tribute_id.clone(),
            },
        )?;
        let nod_id = format!("{}_{}", tribute_id, raffle_run_today);
        let floor_price = exchange_rate
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
                        nominal_minor_rate: tribute.extension.nominal_qty,
                        symbolic_minor_load: tribute.extension.symbolic_load,
                        vector_minor_rate: vector.vector_rate,
                        issuance_minor_rate: exchange_rate,
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

fn distribute_tributes_for_tiers(
    deps: DepsMut,
    total_allocation: Uint128,
    allocation_per_tier: Uint128,
    execution_date_time: Timestamp,
    execution_date: u64,
    deficit: Decimal,
    tribute_address: Addr,
) -> Result<Vec<String>, ContractError> {
    // distribute tokens
    let all_tributes: tribute::query::DailyTributesResponse = deps.querier.query_wasm_smart(
        &tribute_address,
        &tribute::query::QueryMsg::DailyTributes {
            date: execution_date_time,
        },
    )?;
    let all_tributes = all_tributes.tributes;
    println!(
        "Raffle {} tributes distribution for date ",
        all_tributes.len()
    );

    // TODO do sort by fidelity index
    //  such as fidelity_index = 0 for all tributes we avoid sorting as redundant operation

    let total_interest = all_tributes
        .iter()
        .fold(Uint128::zero(), |acc, t| acc + t.data.symbolic_load);

    let total_deficit = calc_total_deficit(total_allocation, total_interest, deficit);

    // TODO calc deficit per pool
    let pool_deficit = total_deficit / Uint128::new(24);

    // Pool Capacity = Pool Allocation + Pool Deficit.
    let pool_capacity = allocation_per_tier + pool_deficit;

    // + 8%
    println!("total_allocation = {}", total_allocation);
    println!("total_interest = {}", total_interest);
    println!("allocation_per_pool = {}", allocation_per_tier);
    println!("total_deficit = {}", total_deficit);
    println!("pool_deficit = {}", pool_deficit);
    println!("pool_capacity = {}", pool_capacity);

    let mut raffle_history: Vec<RaffleRunData> = vec![];
    let mut distributed_tributes: HashSet<String> = HashSet::new();
    let mut pools: Vec<Vec<String>> = Vec::with_capacity(23);
    let mut pool_index: u16 = 0;
    while pool_index < 23 {
        let mut pool_tributes: Vec<String> = vec![];
        let mut allocated_in_pool = Uint128::zero();
        for tribute in all_tributes.clone() {
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
            raffle_date: Timestamp::from_seconds(execution_date),
            raffle_date_time: execution_date_time,
            pool_index,
            total_allocation,
            pool_allocation: allocation_per_tier,
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
            let key = format!("{}_{}_{}", execution_date, i + 1, j);
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

    Ok(pools.first().unwrap_or(&vec![]).clone())
}

fn calc_total_deficit(
    total_allocation: Uint128,
    total_interest: Uint128,
    deficit_percent: Decimal,
) -> Uint128 {
    let mut total_deficit =
        (deficit_percent * Decimal::from_atomics(total_allocation, 0).unwrap()).to_uint_floor();

    if total_interest > total_allocation && total_interest - total_allocation > total_deficit {
        total_deficit = total_interest - total_allocation;
    }
    total_deficit
}

fn execute_touch(
    deps: DepsMut,
    tributes: Vec<FullTributeData>,
    allocation: Uint128,
    execution_date_time: Timestamp,
    nod_address: Addr,
    exchange_rate: Decimal,
) -> Result<Response, ContractError> {
    let date = time_utils::normalize_to_date(execution_date_time).seconds();

    let touch_run_today = DAILY_TOUCH
        .may_load(deps.storage, date)?
        .unwrap_or_default();
    if touch_run_today >= 1 {
        return Err(ContractError::BadRunConfiguration {});
    }
    let touch_run_today = touch_run_today + 1;

    DAILY_TOUCH.save(deps.storage, date, &touch_run_today)?;

    if tributes.is_empty() {
        return Ok(Response::new()
            .add_attribute("action", "raffle::raffle")
            .add_event(Event::new("raffle::raffle").add_attribute("touch", "no-data")));
    }

    // todo implement random. Now first tribute will win.
    let winner = tributes.first().unwrap();

    let mut messages: Vec<SubMsg> = vec![];
    let nod_id = format!("{}_{}", winner.token_id, touch_run_today);
    println!("Nod id creation = {}", nod_id);
    let nod_mint = WasmMsg::Execute {
        contract_addr: nod_address.to_string(),
        msg: to_json_binary(&nod::msg::ExecuteMsg::Submit {
            token_id: nod_id.clone(),
            owner: winner.owner.to_string(),
            extension: Box::new(nod::msg::SubmitExtension {
                entity: nod::msg::NodEntity {
                    nod_id,
                    settlement_token: winner.data.settlement_token.clone(),
                    symbolic_rate: winner.data.nominal_price,
                    nominal_minor_rate: winner.data.nominal_qty,
                    symbolic_minor_load: allocation,
                    vector_minor_rate: Uint128::zero(),
                    issuance_minor_rate: exchange_rate,
                    floor_minor_price: exchange_rate,
                    state: nod::types::State::Issued,
                    address: winner.owner.to_string(),
                },
                created_at: None,
            }),
        })?,
        funds: vec![],
    };
    messages.push(SubMsg::new(nod_mint));

    Ok(Response::new()
        .add_attribute("action", "raffle::raffle")
        .add_event(Event::new("raffle::raffle").add_attribute("touch", touch_run_today.to_string()))
        .add_submessages(messages))
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

fn calc_allocation(
    deps: Deps,
    token_allocator_address: Addr,
) -> Result<(Uint128, Uint128), ContractError> {
    let allocation_per_block: token_allocator::types::TokenAllocatorData =
        deps.querier.query_wasm_smart(
            &token_allocator_address,
            &token_allocator::query::QueryMsg::GetData {},
        )?;

    // todo calc total_allocation based on blocks with exact values
    let total_allocation = Uint128::from(allocation_per_block.amount) * Uint128::new(24 * 60 * 12);
    let allocation_per_tier = total_allocation / Uint128::new(24);

    Ok((total_allocation, allocation_per_tier))
}
