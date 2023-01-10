use ink_env::Hash;
use ink_storage::traits::{
    PackedLayout,
    SpreadLayout,
};
use openbrush::{
    contracts::{
        ownable::OwnableError,
        psp34::Id,
        reentrancy_guard::ReentrancyGuardError,
    },
    storage::Mapping,
    traits::{
        AccountId,
        Balance,
        String,
    },
};
use scale::{
    Decode,
    Encode,
};

pub const STORAGE_KEY: u32 = openbrush::storage_unique_key!(Data);

#[derive(Default, Debug)]
#[openbrush::upgradeable_storage(STORAGE_KEY)]
pub struct Data {
    pub registered_collections: Mapping<AccountId, RegisteredCollection>,
    pub items: Mapping<(AccountId, Id), Item>,
    pub fee: u16,
    pub max_fee: u16,
    pub market_fee_recipient: AccountId,
    pub nft_contract_hash: Hash,
    pub nonce: u64,
}

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum MarketplaceError {
    /// Caller is not a marketplace owner.
    OwnableError(OwnableError),
    /// Caller is tryin to make second call while 1st one is still executing.
    ReentrancyError(ReentrancyGuardError),
    /// Caller is not an NFT owner.
    NotOwner,
    /// A NFT item is not found in a contract.
    ItemNotFound,
    /// A NFT item is not listed for sale
    ItemNotListedForSale,
    /// NFT contract is not registered to the marketplace.
    NotRegisteredContract,
    /// Value send to buy method is invalid
    BadBuyValue,
    /// Fee transfer to the marketplace failed.
    TransferToMarketplaceFailed,
    /// Fee transfer to the marketplace failed.
    TransferToOwnerFailed,
    /// Royalty transfer failed.
    TransferToAuthorFailed,
    /// Contract has been alredy registered to the marketplace
    ContractAlreadyRegistered,
    /// Fee required is too high.
    FeeTooHigh,
    /// Unable to transfer token to a new owner.
    UnableToTransferToken,
    /// PSP23 contract hash was not set
    NftContractHashNotSet,
    /// Factory method was unable to initiate PSP34 contract.
    PSP34InstantiationFailed,
    /// Buyer already owns token.
    AlreadyOwner,
    /// Token does not exist.
    TokenDoesNotExist,
    /// Marketplace item is already listed for sale.
    ItemAlreadyListedForSale,
}

#[derive(Encode, Decode, SpreadLayout, PackedLayout, Default, Debug)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
)]
pub struct RegisteredCollection {
    pub royalty_receiver: AccountId,
    pub marketplace_ipfs: String,
    pub royalty: u16,
}

#[derive(Encode, Decode, SpreadLayout, PackedLayout, Default, Debug)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
)]
pub struct Item {
    pub owner: AccountId,
    pub price: Balance,
}

impl From<OwnableError> for MarketplaceError {
    fn from(error: OwnableError) -> Self {
        MarketplaceError::OwnableError(error)
    }
}

impl From<ReentrancyGuardError> for MarketplaceError {
    fn from(error: ReentrancyGuardError) -> Self {
        MarketplaceError::ReentrancyError(error)
    }
}
