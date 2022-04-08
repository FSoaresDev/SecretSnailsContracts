use cosmwasm_std::{Binary, Coin, HumanAddr, Uint128};
use schemars::JsonSchema;
use secret_toolkit::utils::{HandleCallback, InitCallback};
use serde::{Deserialize, Serialize};

use crate::state::{SecretContract, BLOCK_SIZE};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub admin: Option<HumanAddr>,
    pub token_contract: SecretContract,
    pub entropy: String,
    pub mint_price: Uint128,
    pub max_mint_per_tx: u16,
    pub whitelist: Vec<HumanAddr>,
    pub revenue_split: Vec<RevenueSplit>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RevenueSplit {
    pub address: HumanAddr,
    pub percentage: u32, // 4 decimal places
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Receive {
        sender: HumanAddr,
        from: HumanAddr,
        amount: Uint128,
        msg: Binary,
    },
    UpdateMint {
        whitelist_mint_enabled: bool,
        standard_mint_enabled: bool,
        mint_price: Option<Uint128>,
        max_mint_per_tx: Option<u16>,
    },
    UpdateChangeMetadataPermitedAdresses {
        change_metadata_permited_addresses: Vec<HumanAddr>,
    },
    ChangeAdmin {
        admin: HumanAddr,
    },
    AddNftContract {
        contract: SecretContract,
    },
    LoadMetadata {
        new_data: Vec<PreLoad>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleReceiveMsg {
    MintNfts { count: u16 },
}

/// Preloaded data storage for the tokens during init
#[derive(Serialize, Deserialize, Clone, JsonSchema, PartialEq, Debug)]
pub struct PreLoad {
    pub id: String,
    pub img_url: String,
    pub attributes: Option<Vec<Trait>>,
    pub priv_attributes: Option<Vec<Trait>>,
    pub hidden_attributes: Option<Vec<Trait>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    AddNftContract { status: ResponseStatus },
    MintNfts { status: ResponseStatus },
    ChangeAdmin { status: ResponseStatus },
    UpdateChangeMetadataPermitedAdresses { status: ResponseStatus },
    UpdateMint { status: ResponseStatus },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Success,
    Failure,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Info {},
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    Info {
        admin: HumanAddr,
        token_contract: SecretContract,
        nft_contract: SecretContract,
        mint_price: Uint128,
        whitelist_mint_enabled: bool,
        standard_mint_enabled: bool,
        max_mint_per_tx: u16,
        mint_current_count: u32,
        mint_current_left: u16,
    },
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum NftsHandleMsg {
    MintNft {
        /// optional token id. if omitted, use current token index
        token_id: Option<String>,
        /// optional owner address. if omitted, owned by the message sender
        owner: Option<HumanAddr>,
        /// optional public metadata that can be seen by everyone
        public_metadata: Option<Metadata>,
        /// optional private metadata that can only be seen by the owner and whitelist
        private_metadata: Option<Metadata>,
        /// optional serial number for this token
        serial_number: Option<SerialNumber>,
        /// optional royalty information for this token.  This will be ignored if the token is
        /// non-transferable
        royalty_info: Option<RoyaltyInfo>,
        /// optionally true if the token is transferable.  Defaults to true if omitted
        transferable: Option<bool>,
        /// optional memo for the tx
        memo: Option<String>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// Mint multiple tokens
    BatchMintNft {
        /// list of mint operations to perform
        mints: Vec<Mint>,
        /// optional message length padding
        padding: Option<String>,
    },
}

impl HandleCallback for NftsHandleMsg {
    const BLOCK_SIZE: usize = BLOCK_SIZE;
}

/// token mint info used when doing a BatchMint
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct Mint {
    /// optional token id, if omitted, use current token index
    pub token_id: Option<String>,
    /// optional owner address, owned by the minter otherwise
    pub owner: Option<HumanAddr>,
    /// optional public metadata that can be seen by everyone
    pub public_metadata: Option<Metadata>,
    /// optional private metadata that can only be seen by owner and whitelist
    pub private_metadata: Option<Metadata>,
    /// optional serial number for this token
    pub serial_number: Option<SerialNumber>,
    /// optional royalty information for this token.  This will be ignored if the token is
    /// non-transferable
    pub royalty_info: Option<RoyaltyInfo>,
    /// optionally true if the token is transferable.  Defaults to true if omitted
    pub transferable: Option<bool>,
    /// optional memo for the tx
    pub memo: Option<String>,
    pub hidden_attributes: Option<Vec<HiddenAttribute>>,
}

/// token metadata
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug, Default)]
pub struct Metadata {
    /// optional uri for off-chain metadata.  This should be prefixed with `http://`, `https://`, `ipfs://`, or
    /// `ar://`.  Only use this if you are not using `extension`
    pub token_uri: Option<String>,
    /// optional on-chain metadata.  Only use this if you are not using `token_uri`
    pub extension: Option<Extension>,
}

/// metadata extension
/// You can add any metadata fields you need here.  These fields are based on
/// https://docs.opensea.io/docs/metadata-standards and are the metadata fields that
/// Stashh uses for robust NFT display.  Urls should be prefixed with `http://`, `https://`, `ipfs://`, or
/// `ar://`
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug, Default)]
pub struct Extension {
    /// url to the image
    pub image: Option<String>,
    /// raw SVG image data (not recommended). Only use this if you're not including the image parameter
    pub image_data: Option<String>,
    /// url to allow users to view the item on your site
    pub external_url: Option<String>,
    /// item description
    pub description: Option<String>,
    /// name of the item
    pub name: Option<String>,
    /// item attributes
    pub attributes: Option<Vec<Trait>>,
    /// background color represented as a six-character hexadecimal without a pre-pended #
    pub background_color: Option<String>,
    /// url to a multimedia attachment
    pub animation_url: Option<String>,
    /// url to a YouTube video
    pub youtube_url: Option<String>,
    /// media files as specified on Stashh that allows for basic authenticatiion and decryption keys.
    /// Most of the above is used for bridging public eth NFT metadata easily, whereas `media` will be used
    /// when minting NFTs on Stashh
    pub media: Option<Vec<MediaFile>>,
    /// a select list of trait_types that are in the private metadata.  This will only ever be used
    /// in public metadata
    pub protected_attributes: Option<Vec<String>>,
    /// token subtypes used by Stashh for display groupings (primarily used for badges)
    pub token_subtype: Option<String>,
}

/// attribute trait
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug, Default)]
pub struct Trait {
    /// indicates how a trait should be displayed
    pub display_type: Option<String>,
    /// name of the trait
    pub trait_type: Option<String>,
    /// trait value
    pub value: String,
    /// optional max value for numerical traits
    pub max_value: Option<String>,
}

/// media file
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug, Default)]
pub struct MediaFile {
    /// file type
    /// Stashh currently uses: "image", "video", "audio", "text", "font", "application"
    pub file_type: Option<String>,
    /// file extension
    pub extension: Option<String>,
    /// authentication information
    pub authentication: Option<Authentication>,
    /// url to the file.  Urls should be prefixed with `http://`, `https://`, `ipfs://`, or `ar://`
    pub url: String,
}

/// media file authentication
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug, Default)]
pub struct Authentication {
    /// either a decryption key for encrypted files or a password for basic authentication
    pub key: Option<String>,
    /// username used in basic authentication
    pub user: Option<String>,
}

/// Serial number to give an NFT when minting
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct SerialNumber {
    /// optional number of the mint run this token will be minted in.  A mint run represents a
    /// batch of NFTs released at the same time.  So if a creator decided to make 100 copies
    /// of an NFT, they would all be part of mint run number 1.  If they sold quickly, and
    /// the creator wanted to rerelease that NFT, he could make 100 more copies which would all
    /// be part of mint run number 2.
    pub mint_run: Option<u32>,
    /// serial number (in this mint run).  This is used to serialize
    /// identical NFTs
    pub serial_number: u32,
    /// optional total number of NFTs minted on this run.  This is used to
    /// represent that this token is number m of n
    pub quantity_minted_this_run: Option<u32>,
}

/// all royalty information
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct RoyaltyInfo {
    /// decimal places in royalty rates
    pub decimal_places_in_rates: u8,
    /// list of royalties
    pub royalties: Vec<Royalty>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Royalty {
    /// address to send royalties to
    pub recipient: HumanAddr,
    /// royalty rate
    pub rate: u16,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct HiddenAttribute {
    pub name: String,
    pub value: String,
}
