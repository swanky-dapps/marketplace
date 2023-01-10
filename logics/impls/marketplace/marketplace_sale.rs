// Copyright (c) 2022 Astar Network
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the"Software"),
// to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
// LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
// WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use super::types::RegisteredCollection;
use crate::{
    ensure,
    impls::marketplace::types::{
        Data,
        Item,
        MarketplaceError,
    },
    traits::marketplace::MarketplaceSale,
};
use ink_env::{
    hash::Blake2x256,
    Hash,
};
use ink_lang::ToAccountId;
use openbrush::{
    contracts::{
        ownable::*,
        psp34::*,
        reentrancy_guard::*,
    },
    modifiers,
    traits::{
        AccountId,
        Balance,
        Storage,
        String,
    },
};
use shiden34::shiden34::Shiden34ContractRef;

pub trait Internal {
    /// Checks if contract caller is an token owner
    fn check_token_owner(
        &self,
        contract_address: AccountId,
        token_id: Id,
    ) -> Result<(), MarketplaceError>;

    /// Checks token price.
    fn check_price(
        &self,
        transfered_value: Balance,
        price: Balance,
    ) -> Result<(), MarketplaceError>;

    /// Checks fee
    fn check_fee(&self, fee: u16, max_fee: u16) -> Result<(), MarketplaceError>;

    /// Checks if token is listed for sale on the marketplace.
    fn is_token_listed(&self, contract_address: AccountId, token_id: Id) -> bool;

    /// Transfers token.
    fn transfer_token(
        &self,
        contract_address: AccountId,
        token_id: Id,
        token_owner: AccountId,
        buyer: AccountId,
        seller_fee: Balance,
        marketplace_fee: Balance,
        royalty_receiver: AccountId,
        author_royalty: Balance,
    ) -> Result<(), MarketplaceError>;
}

impl<T> MarketplaceSale for T
where
    T: Storage<Data> + Storage<ownable::Data> + Storage<reentrancy_guard::Data>,
{
    /// Adds a NFT contract to the marketplace.
    default fn factory(
        &mut self,
        marketplace_ipfs: String,
        royalty_receiver: AccountId,
        royalty: u16,
        nft_name: String,
        nft_symbol: String,
        nft_base_uri: String,
        nft_max_supply: u64,
        nft_price_per_mint: Balance,
    ) -> Result<AccountId, MarketplaceError> {
        let contract_hash = self.data::<Data>().nft_contract_hash;
        if contract_hash == Hash::default() {
            return Err(MarketplaceError::NftContractHashNotSet)
        }

        // Generate salt
        let nonce = self.data::<Data>().nonce.saturating_add(1);
        let caller = Self::env().caller();
        let salt = Self::env().hash_encoded::<Blake2x256, _>(&(caller, nonce));

        let nft = Shiden34ContractRef::new(
            nft_name,
            nft_symbol,
            nft_base_uri,
            nft_max_supply,
            nft_price_per_mint,
        )
        .endowment(0)
        .code_hash(contract_hash)
        .salt_bytes(&salt[..4])
        .instantiate()
        .map_err(|_| MarketplaceError::PSP34InstantiationFailed)?;

        let contract_address = nft.to_account_id();
        self.data::<Data>().registered_collections.insert(
            &contract_address,
            &RegisteredCollection {
                royalty_receiver,
                royalty,
                marketplace_ipfs,
            },
        );

        self.data::<Data>().nonce = nonce;

        Ok(contract_address)
    }

    /// Sets a hash of a Shiden34 contract to be instantiated by factory call.
    #[modifiers(only_owner)]
    default fn set_nft_contract_hash(
        &mut self,
        contract_hash: Hash,
    ) -> Result<(), MarketplaceError> {
        self.data::<Data>().nft_contract_hash = contract_hash;

        Ok(())
    }

    /// Gets Shiden34 contract hash.
    default fn nft_contract_hash(&self) -> Hash {
        self.data::<Data>().nft_contract_hash
    }

    /// Creates a NFT item sale on the marketplace.
    default fn list(
        &mut self,
        contract_address: AccountId,
        token_id: Id,
        price: Balance,
    ) -> Result<(), MarketplaceError> {
        ensure!(
            !self.is_token_listed(contract_address, token_id.clone()),
            MarketplaceError::ItemAlreadyListedForSale
        );
        self.check_token_owner(contract_address, token_id.clone())?;
        self.data::<Data>().items.insert(
            &(contract_address, token_id),
            &Item {
                owner: Self::env().caller(),
                price,
            },
        );
        Ok(())
    }

    /// Removes a NFT from the marketplace sale.
    default fn unlist(
        &mut self,
        contract_address: AccountId,
        token_id: Id,
    ) -> Result<(), MarketplaceError> {
        ensure!(
            self.is_token_listed(contract_address, token_id.clone()),
            MarketplaceError::ItemNotListedForSale
        );
        self.check_token_owner(contract_address, token_id.clone())?;

        self.data::<Data>()
            .items
            .remove(&(contract_address, token_id));
        Ok(())
    }

    /// Buys NFT item from the marketplace.
    #[modifiers(non_reentrant)]
    default fn buy(
        &mut self,
        contract_address: AccountId,
        token_id: Id,
    ) -> Result<(), MarketplaceError> {
        let item = self
            .data::<Data>()
            .items
            .get(&(contract_address, token_id.clone()))
            .ok_or(MarketplaceError::ItemNotListedForSale)?;

        let token_owner = PSP34Ref::owner_of(&contract_address, token_id.clone())
            .ok_or(MarketplaceError::TokenDoesNotExist)?;
        let caller = Self::env().caller();
        ensure!(token_owner != caller, MarketplaceError::AlreadyOwner);

        let value = Self::env().transferred_value();
        self.check_price(value, item.price)?;

        let collection = self
            .data::<Data>()
            .registered_collections
            .get(&contract_address)
            .ok_or(MarketplaceError::NotRegisteredContract)?;

        let marketplace_fee = value
            .checked_mul(self.data::<Data>().fee as u128)
            .unwrap_or_default()
            / 10_000;
        let author_royalty = value
            .checked_mul(collection.royalty as u128)
            .unwrap_or_default()
            / 10_000;
        let seller_fee = value
            .checked_sub(marketplace_fee)
            .unwrap_or_default()
            .checked_sub(author_royalty)
            .unwrap_or_default();

        self.transfer_token(
            contract_address,
            token_id,
            token_owner,
            caller,
            seller_fee,
            marketplace_fee,
            collection.royalty_receiver,
            author_royalty,
        )
    }

    /// Registers NFT collection to the marketplace.
    default fn register(
        &mut self,
        contract_address: AccountId,
        royalty_receiver: AccountId,
        royalty: u16,
        marketplace_ipfs: String,
    ) -> Result<(), MarketplaceError> {
        let max_fee = self.data::<Data>().max_fee;
        self.check_fee(royalty, max_fee)?;

        let caller = Self::env().caller();

        // Check if caller is Marketplace owner of NFT owner.
        if self.data::<ownable::Data>().owner != caller
            && OwnableRef::owner(&contract_address) != caller
        {
            return Err(MarketplaceError::NotOwner)
        }

        if self
            .data::<Data>()
            .registered_collections
            .get(&contract_address)
            .is_some()
        {
            Err(MarketplaceError::ContractAlreadyRegistered)
        } else {
            self.data::<Data>().registered_collections.insert(
                &contract_address,
                &RegisteredCollection {
                    royalty_receiver,
                    royalty,
                    marketplace_ipfs,
                },
            );

            Ok(())
        }
    }

    /// Gets registered collection.
    default fn get_registered_collection(
        &self,
        contract_address: AccountId,
    ) -> Option<RegisteredCollection> {
        self.data::<Data>()
            .registered_collections
            .get(&contract_address)
    }

    /// Sets the marketplace fee.
    #[modifiers(only_owner)]
    default fn set_marketplace_fee(&mut self, fee: u16) -> Result<(), MarketplaceError> {
        let max_fee = self.data::<Data>().max_fee;
        self.check_fee(fee, max_fee)?;
        self.data::<Data>().fee = fee;

        Ok(())
    }

    /// Gets the marketplace fee.
    default fn get_marketplace_fee(&self) -> u16 {
        self.data::<Data>().fee
    }

    /// Gets max fee that can be applied to an item price.
    default fn get_max_fee(&self) -> u16 {
        self.data::<Data>().max_fee
    }

    /// Checks if NFT token is listed on the marketplace and returns token price.
    default fn get_price(&self, contract_address: AccountId, token_id: Id) -> Option<Balance> {
        match self.data::<Data>().items.get(&(contract_address, token_id)) {
            Some(item) => Some(item.price),
            _ => None,
        }
    }

    /// Sets contract metadata (ipfs url)
    #[modifiers(only_owner)]
    default fn set_contract_metadata(
        &mut self,
        contract_address: AccountId,
        ipfs: String,
    ) -> Result<(), MarketplaceError> {
        let collection = self
            .data::<Data>()
            .registered_collections
            .get(&contract_address)
            .ok_or(MarketplaceError::NotRegisteredContract)?;

        self.data::<Data>().registered_collections.insert(
            &contract_address,
            &RegisteredCollection {
                royalty_receiver: collection.royalty_receiver,
                marketplace_ipfs: ipfs,
                royalty: collection.royalty,
            },
        );

        Ok(())
    }

    /// Gets the marketplace fee recipient.
    default fn get_fee_recipient(&self) -> AccountId {
        self.data::<Data>().market_fee_recipient
    }

    /// Sets the marketplace fee recipient.
    #[modifiers(only_owner)]
    default fn set_fee_recipient(
        &mut self,
        fee_recipient: AccountId,
    ) -> Result<(), MarketplaceError> {
        self.data::<Data>().market_fee_recipient = fee_recipient;

        Ok(())
    }
}

impl<T> Internal for T
where
    T: Storage<Data>,
{
    default fn check_token_owner(
        &self,
        contract_address: AccountId,
        token_id: Id,
    ) -> Result<(), MarketplaceError> {
        if !self
            .data::<Data>()
            .registered_collections
            .contains(&contract_address)
        {
            return Err(MarketplaceError::NotRegisteredContract)
        }

        let caller = Self::env().caller();
        match PSP34Ref::owner_of(&contract_address, token_id) {
            Some(token_owner) => {
                ensure!(caller == token_owner, MarketplaceError::NotOwner);
                Ok(())
            }
            None => Err(MarketplaceError::TokenDoesNotExist),
        }
    }

    default fn check_price(
        &self,
        transfered_value: Balance,
        price: Balance,
    ) -> Result<(), MarketplaceError> {
        ensure!(transfered_value >= price, MarketplaceError::BadBuyValue);

        Ok(())
    }

    default fn check_fee(&self, fee: u16, max_fee: u16) -> Result<(), MarketplaceError> {
        ensure!(fee <= max_fee, MarketplaceError::FeeTooHigh);

        Ok(())
    }

    default fn is_token_listed(&self, contract_address: AccountId, token_id: Id) -> bool {
        self.data::<Data>()
            .items
            .get(&(contract_address, token_id))
            .is_some()
    }

    fn transfer_token(
        &self,
        contract_address: AccountId,
        token_id: Id,
        token_owner: AccountId,
        buyer: AccountId,
        seller_fee: Balance,
        marketplace_fee: Balance,
        royalty_receiver: AccountId,
        author_royalty: Balance,
    ) -> Result<(), MarketplaceError> {
        match PSP34Ref::transfer(
            &contract_address,
            buyer,
            token_id,
            ink_prelude::vec::Vec::new(),
        ) {
            Ok(()) => {
                Self::env()
                    .transfer(token_owner, seller_fee)
                    .map_err(|_| MarketplaceError::TransferToOwnerFailed)?;
                Self::env()
                    .transfer(self.data::<Data>().market_fee_recipient, marketplace_fee)
                    .map_err(|_| MarketplaceError::TransferToMarketplaceFailed)?;
                Self::env()
                    .transfer(royalty_receiver, author_royalty)
                    .map_err(|_| MarketplaceError::TransferToAuthorFailed)?;
                Ok(())
            }
            Err(_) => Err(MarketplaceError::UnableToTransferToken),
        }
    }
}
