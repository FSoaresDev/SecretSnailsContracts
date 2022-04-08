use cosmwasm_std::{
    from_binary, to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse,
    Querier, StdError, StdResult, Storage, Uint128,
};
use cosmwasm_storage::PrefixedStorage;
use rand::prelude::SliceRandom;
use rand::{Rng, RngCore};
use secret_toolkit::crypto::Prng;
use secret_toolkit::snip20::{send_msg, transfer_msg};

use crate::msg::{
    Authentication, Extension, HandleAnswer, HandleReceiveMsg, HiddenAttribute, MediaFile,
    Metadata, Mint, NftsHandleMsg, PreLoad, QueryAnswer, ResponseStatus, Trait,
};
use crate::state::{load, may_load, save, SecretContract, BLOCK_SIZE};
use crate::{
    msg::{HandleMsg, InitMsg, QueryMsg},
    state::Config,
};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::{ChaCha20Rng, ChaChaRng};
use secret_toolkit::storage::{TypedStore, TypedStoreMut};
use secret_toolkit::utils::{HandleCallback, InitCallback};
use secret_toolkit::{crypto::sha_256, snip20::register_receive_msg};
use sha2::{Digest, Sha256};

pub const COUNT_KEY: &[u8] = b"count";
pub const CONFIG_KEY: &[u8] = b"config";
pub const PREFIX_WHITELIST: &[u8] = b"whitelistprefix";
pub const PRNG_SEED_KEY: &[u8] = b"prngseed";

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let mut config_store = TypedStoreMut::attach(&mut deps.storage);
    let admin = msg.admin.unwrap_or(env.message.sender);

    config_store.store(
        CONFIG_KEY,
        &Config {
            admin,
            token_contract: msg.token_contract.clone(),
            nft_contract: None,
            mint_price: msg.mint_price.clone(),
            max_mint_per_tx: msg.max_mint_per_tx.clone(),
            whitelist_mint_enabled: false,
            standard_mint_enabled: false,
            revenue_split: msg.revenue_split.clone(),
            change_metadata_permited_addresses: vec![],
        },
    )?;

    let prng_seed: Vec<u8> = sha_256(base64::encode(msg.entropy).as_bytes()).to_vec();
    save(&mut deps.storage, PRNG_SEED_KEY, &prng_seed)?;
    save(&mut deps.storage, COUNT_KEY, &0)?;
    let mut white_store = PrefixedStorage::new(PREFIX_WHITELIST, &mut deps.storage);
    for hum_addr in msg.whitelist.iter() {
        save(&mut white_store, &hum_addr.0.as_bytes(), &false)?;
    }

    Ok(InitResponse {
        messages: vec![register_receive_msg(
            env.contract_code_hash.clone(),
            None,
            1,
            msg.token_contract.token_code_hash.clone(),
            msg.token_contract.contract_addr.clone(),
        )?],
        log: vec![],
    })
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::UpdateMint {
            whitelist_mint_enabled,
            standard_mint_enabled,
            mint_price,
            max_mint_per_tx,
        } => update_mint(
            deps,
            env,
            whitelist_mint_enabled,
            standard_mint_enabled,
            mint_price,
            max_mint_per_tx,
        ),
        HandleMsg::Receive {
            sender,
            from,
            amount,
            msg,
        } => try_receive(deps, env, sender, from, amount, msg),
        HandleMsg::AddNftContract { contract } => add_nft_contract(deps, env, contract),
        HandleMsg::LoadMetadata { new_data } => load_metadata(deps, env, new_data),
        HandleMsg::ChangeAdmin { admin } => change_admin(deps, env, admin),
        HandleMsg::UpdateChangeMetadataPermitedAdresses {
            change_metadata_permited_addresses,
        } => {
            update_change_metadata_permited_addresses(deps, env, change_metadata_permited_addresses)
        }
    }
}

pub fn update_mint<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    whitelist_mint_enabled: bool,
    standard_mint_enabled: bool,
    mint_price: Option<Uint128>,
    max_mint_per_tx: Option<u16>,
) -> StdResult<HandleResponse> {
    let mut config_store = TypedStoreMut::attach(&mut deps.storage);
    let mut config: Config = config_store.load(CONFIG_KEY)?;

    if env.message.sender != config.admin {
        return Err(StdError::generic_err(format!(
            "Only admin can execute this action!"
        )));
    }

    config.whitelist_mint_enabled = whitelist_mint_enabled;
    config.standard_mint_enabled = standard_mint_enabled;

    if let Some(mint_price) = mint_price {
        config.mint_price = mint_price
    }

    if let Some(max_mint_per_tx) = max_mint_per_tx {
        config.max_mint_per_tx = max_mint_per_tx
    }

    config_store.store(CONFIG_KEY, &config)?;

    return Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::UpdateMint {
            status: ResponseStatus::Success,
        })?),
    });
}

pub fn add_nft_contract<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    contract: SecretContract,
) -> StdResult<HandleResponse> {
    let mut config_store = TypedStoreMut::attach(&mut deps.storage);
    let mut config: Config = config_store.load(CONFIG_KEY)?;

    if env.message.sender != config.admin {
        return Err(StdError::generic_err(format!(
            "Only admin can execute this action!"
        )));
    }

    if config.standard_mint_enabled != false || config.whitelist_mint_enabled != false {
        return Err(StdError::generic_err(format!(
            "Mint should be stoped to perform this"
        )));
    }

    config.nft_contract = Some(contract);

    config_store.store(CONFIG_KEY, &config)?;

    return Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::AddNftContract {
            status: ResponseStatus::Success,
        })?),
    });
}

/// Lets Admin load metadata used in random minting
pub fn load_metadata<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    new_data: Vec<PreLoad>,
) -> StdResult<HandleResponse> {
    let config_store = TypedStore::attach(&deps.storage);
    let config: Config = config_store.load(CONFIG_KEY)?;

    if env.message.sender != config.admin {
        return Err(StdError::generic_err(format!(
            "Only admin can execute this action!"
        )));
    }

    let mut id: u16 = load(&deps.storage, COUNT_KEY)?;

    for data in new_data.iter() {
        id = id + 1;
        save(&mut deps.storage, &id.clone().to_le_bytes(), data)?;
    }

    save(&mut deps.storage, COUNT_KEY, &id)?;

    Ok(HandleResponse::default())
}

pub fn change_admin<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    admin: HumanAddr,
) -> StdResult<HandleResponse> {
    let mut config_store = TypedStoreMut::attach(&mut deps.storage);
    let mut config: Config = config_store.load(CONFIG_KEY)?;

    if env.message.sender != config.admin {
        return Err(StdError::generic_err(format!(
            "Only admin can execute this action!"
        )));
    }

    config.admin = admin;

    config_store.store(CONFIG_KEY, &config)?;

    return Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::ChangeAdmin {
            status: ResponseStatus::Success,
        })?),
    });
}

pub fn update_change_metadata_permited_addresses<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    change_metadata_permited_addresses: Vec<HumanAddr>,
) -> StdResult<HandleResponse> {
    let mut config_store = TypedStoreMut::attach(&mut deps.storage);
    let mut config: Config = config_store.load(CONFIG_KEY)?;

    if env.message.sender != config.admin {
        return Err(StdError::generic_err(format!(
            "Only admin can execute this action!"
        )));
    }

    config.change_metadata_permited_addresses = change_metadata_permited_addresses;

    config_store.store(CONFIG_KEY, &config)?;

    return Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(
            &HandleAnswer::UpdateChangeMetadataPermitedAdresses {
                status: ResponseStatus::Success,
            },
        )?),
    });
}

pub fn try_receive<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    _sender: HumanAddr,
    from: HumanAddr,
    amount: Uint128,
    msg: Binary,
) -> StdResult<HandleResponse> {
    let config = TypedStore::<Config, S>::attach(&deps.storage).load(CONFIG_KEY)?;
    let msg: HandleReceiveMsg = from_binary(&msg)?;
    if let HandleReceiveMsg::MintNfts { count } = msg.clone() {
        if env.message.sender != config.token_contract.contract_addr {
            return Err(StdError::generic_err(format!("Invalid token sent!")));
        } else {
            return mint_nfts(deps, env.clone(), amount, from, count);
        }
    } else {
        return Err(StdError::generic_err(format!("Receive handler not found!")));
    }
}

pub fn mint_nfts<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Uint128,
    from: HumanAddr,
    mint_count: u16,
) -> StdResult<HandleResponse> {
    let config = TypedStore::<Config, S>::attach(&deps.storage).load(CONFIG_KEY)?;

    let nft_contract = if let Some(nft_contract) = config.nft_contract.clone() {
        nft_contract
    } else {
        return Err(StdError::generic_err(format!("No NFT contract set")));
    };

    // Checks how many tokens are left
    let mut count: u16 = load(&deps.storage, COUNT_KEY)?;

    if count == 0 {
        return Err(StdError::generic_err("All tokens have been minted"));
    }

    if count.checked_sub(mint_count) == None {
        return Err(StdError::generic_err(
            "Not enought tokens to be minted by this request!",
        ));
    }

    if mint_count > config.max_mint_per_tx {
        return Err(StdError::generic_err(format!(
            "Requested mint count is too high, max is {}",
            config.max_mint_per_tx
        )));
    }

    if !config.standard_mint_enabled && !config.whitelist_mint_enabled {
        return Err(StdError::generic_err(format!("Mint is not enabled!")));
    }

    // Check if sent amount is correct
    let total_amount_expected = config.mint_price.u128() * (mint_count as u128);

    if total_amount_expected != amount.u128() {
        return Err(StdError::generic_err(format!(
            "Incorrect amount of snip20 tokens received {:?} != {:?}",
            amount.u128(),
            total_amount_expected
        )));
    }

    if config.whitelist_mint_enabled {
        //Whitelist management
        //Checks if minter has a whitelist reservation, and removes their reservation after minting
        let mut white_store = PrefixedStorage::new(PREFIX_WHITELIST, &mut deps.storage);

        let list_check: Option<bool> = may_load(&white_store, env.message.sender.0.as_bytes())?;

        // If addr is on list and hasn't minted
        if Some(false) == list_check {
            save(&mut white_store, env.message.sender.0.as_bytes(), &true)?;
        } else if Some(true) == list_check && !config.standard_mint_enabled {
            return Err(StdError::generic_err(format!(
                "Whitelist enabled only, sender address already minted on his token eligible thought the whitelist"
            )));
        } else if !config.standard_mint_enabled {
            return Err(StdError::generic_err(format!(
                "Whitelist enabled only, sender address not eligible for minting"
            )));
        }
    }

    let mut mints: Vec<Mint> = vec![];

    let prng_seed: Vec<u8> = load(&deps.storage, PRNG_SEED_KEY)?;

    for index in 1..=mint_count {
        let random_seed = new_entropy(
            &env,
            prng_seed.as_ref(),
            prng_seed.as_ref(),
            index.to_string().as_bytes(),
        );
        let mut rng = ChaChaRng::from_seed(random_seed);

        // Pull random token data for minting then remove from data pool
        let num = (rng.next_u32() % (count as u32)) as u16 + 1; // an id number between 1 and count

        let token_data: PreLoad = load(&deps.storage, &num.to_le_bytes())?;
        let swap_data: PreLoad = load(&deps.storage, &count.to_le_bytes())?;

        count = count - 1;

        save(&mut deps.storage, &num.to_le_bytes(), &swap_data)?;
        save(&mut deps.storage, COUNT_KEY, &count)?;

        mints.push(Mint {
            token_id: Some(token_data.id.clone()),
            owner: Some(from.clone()),
            public_metadata: Some(Metadata {
                extension: Some(Extension {
                    image: None,
                    image_data: None,
                    external_url: None,
                    description: None,
                    name: Some("Secret Snail #".to_string() + &token_data.id.to_string()),
                    attributes: Some(vec![
                        Trait {
                            display_type: None,
                            trait_type: Some("Wins".to_string()),
                            value: 0.to_string(),
                            max_value: None,
                        },
                        Trait {
                            display_type: None,
                            trait_type: Some("Loses".to_string()),
                            value: 0.to_string(),
                            max_value: None,
                        },
                    ]),
                    background_color: None,
                    animation_url: None,
                    youtube_url: None,
                    media: Some(vec![MediaFile {
                        file_type: Some("image".to_string()),
                        extension: Some("gif".to_string()),
                        authentication: Some(Authentication {
                            key: Some("".to_string()),
                            user: Some("".to_string()),
                        }),
                        url: token_data.img_url.clone(),
                    }]),
                    protected_attributes: None,
                    token_subtype: None,
                }),
                token_uri: None,
            }),
            private_metadata: Some(Metadata {
                extension: Some(Extension {
                    image: None,
                    image_data: None,
                    external_url: None,
                    description: None,
                    name: Some("Secret Snail #".to_string() + &token_data.id.to_string()),
                    attributes: Some(vec![Trait {
                        display_type: None,
                        trait_type: Some("Category".to_string()),
                        value: "Stephen Hawking".to_string(),
                        max_value: None,
                    }]),
                    background_color: None,
                    animation_url: None,
                    youtube_url: None,
                    media: Some(vec![MediaFile {
                        file_type: Some("image".to_string()),
                        extension: Some("gif".to_string()),
                        authentication: Some(Authentication {
                            key: Some("".to_string()),
                            user: Some("".to_string()),
                        }),
                        url: token_data.img_url.clone(),
                    }]),
                    protected_attributes: None,
                    token_subtype: None,
                }),
                token_uri: None,
            }),
            memo: None,
            serial_number: None,
            royalty_info: None,
            transferable: None,
            hidden_attributes: Some(vec![HiddenAttribute {
                name: "speed".to_string(),
                value: rng.gen_range(1, 101).to_string(),
            }]),
        });
    }

    let mints_msg = NftsHandleMsg::BatchMintNft {
        mints,
        padding: None,
    };

    let mints_cosmos_msg = mints_msg.to_cosmos_msg(
        nft_contract.token_code_hash,
        nft_contract.contract_addr,
        None,
    )?;

    let mut messages = vec![mints_cosmos_msg];

    // TODO: mint revenue

    return Ok(HandleResponse {
        messages,
        log: vec![],
        data: Some(to_binary(&HandleAnswer::MintNfts {
            status: ResponseStatus::Success,
        })?),
    });
}

pub fn new_entropy(env: &Env, seed: &[u8], entropy: &[u8], index: &[u8]) -> [u8; 32] {
    // 16 here represents the lengths in bytes of the block height and time.
    let entropy_len = 16 + env.message.sender.len() + entropy.len() + index.len();
    let mut rng_entropy = Vec::with_capacity(entropy_len);
    rng_entropy.extend_from_slice(&env.block.height.to_be_bytes());
    rng_entropy.extend_from_slice(&env.block.time.to_be_bytes());
    rng_entropy.extend_from_slice(&env.message.sender.0.as_bytes());
    rng_entropy.extend_from_slice(entropy);
    rng_entropy.extend_from_slice(index);

    let mut rng = Prng::new(seed, &rng_entropy);

    rng.rand_bytes()
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => query_info(deps),
    }
}

fn query_info<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<Binary> {
    let config_store = TypedStore::attach(&deps.storage);
    let config: Config = config_store.load(CONFIG_KEY)?;

    let id: u16 = load(&deps.storage, COUNT_KEY)?;

    let nft_contract = config.nft_contract.unwrap().clone();

    let nft_current_count_response = secret_toolkit::snip721::num_tokens_query(
        &deps.querier,
        None,
        BLOCK_SIZE,
        nft_contract.token_code_hash.clone(),
        nft_contract.contract_addr.clone(),
    )?;

    to_binary(&QueryAnswer::Info {
        admin: config.admin,
        token_contract: config.token_contract,
        nft_contract,
        mint_price: config.mint_price,
        max_mint_per_tx: config.max_mint_per_tx,
        whitelist_mint_enabled: config.whitelist_mint_enabled,
        standard_mint_enabled: config.standard_mint_enabled,
        mint_current_count: nft_current_count_response.count,
        mint_current_left: id,
    })
}
