#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]

#[openbrush::contract]
pub mod marketplace {
    use ink_storage::traits::SpreadAllocate;
    use openbrush::{
        contracts::{
            ownable::*,
            reentrancy_guard::*,
        },
        traits::Storage,
    };
    use pallet_marketplace::{
        impls::marketplace::*,
        traits::marketplace::*,
    };

    // MarketplaceContract contract storage
    #[ink(storage)]
    #[derive(Default, SpreadAllocate, Storage)]
    pub struct MarketplaceContract {
        #[storage_field]
        ownable: ownable::Data,
        #[storage_field]
        guard: reentrancy_guard::Data,
        #[storage_field]
        marketplace: types::Data,
    }

    impl MarketplaceContract {
        #[ink(constructor)]
        pub fn new(market_fee_recipient: AccountId) -> Self {
            ink_lang::codegen::initialize_contract(|instance: &mut MarketplaceContract| {
                instance.marketplace.fee = 100; // 1%
                instance.marketplace.max_fee = 1000; // 10%
                instance.marketplace.market_fee_recipient = market_fee_recipient;

                let caller = instance.env().caller();
                instance._init_with_owner(caller);
            })
        }
    }

    impl MarketplaceSale for MarketplaceContract {}

    // ***************************** Tests *******************************
    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::marketplace::MarketplaceContract;
        use ink_env::test;
        use ink_lang as ink;
        use openbrush::{
            contracts::psp34::Id,
            traits::String,
        };
        use pallet_marketplace::impls::marketplace::types::MarketplaceError;

        #[ink::test]
        fn new_works() {
            let marketplace = init_contract();
            assert_eq!(marketplace.get_marketplace_fee(), 100);
            assert_eq!(marketplace.get_max_fee(), 1000);
            assert_eq!(marketplace.get_fee_recipient(), fee_recipient());
        }

        #[ink::test]
        fn set_marketplace_fee_works() {
            let mut marketplace = init_contract();

            assert!(marketplace.set_marketplace_fee(120).is_ok());
            assert_eq!(marketplace.get_marketplace_fee(), 120);
        }

        #[ink::test]
        fn set_marketplace_fee_fails_if_not_owner() {
            let mut marketplace = init_contract();

            let accounts = default_accounts();
            set_sender(accounts.bob);
            assert_eq!(
                marketplace.set_marketplace_fee(120),
                Err(MarketplaceError::OwnableError(
                    OwnableError::CallerIsNotOwner
                ))
            );
        }

        #[ink::test]
        fn set_marketplace_fee_fails_if_fee_too_high() {
            let mut marketplace = init_contract();

            assert_eq!(
                marketplace.set_marketplace_fee(1001),
                Err(MarketplaceError::FeeTooHigh)
            );
            assert!(marketplace.set_marketplace_fee(1000).is_ok());
        }

        #[ink::test]
        fn set_fee_recipient_works() {
            let mut marketplace = init_contract();
            let accounts = default_accounts();

            assert!(marketplace.set_fee_recipient(accounts.bob).is_ok());
            assert_eq!(marketplace.get_fee_recipient(), accounts.bob);
        }

        #[ink::test]
        fn set_fee_recipient_fails_if_not_owner() {
            let mut marketplace = init_contract();
            let accounts = default_accounts();
            set_sender(accounts.bob);

            assert_eq!(
                marketplace.set_fee_recipient(accounts.bob),
                Err(MarketplaceError::OwnableError(
                    OwnableError::CallerIsNotOwner
                ))
            );
        }

        #[ink::test]
        fn buy_fails_if_unlisted_token() {
            let mut marketplace = init_contract();

            assert_eq!(
                marketplace.buy(contract_address(), Id::U128(1)),
                Err(MarketplaceError::ItemNotListedForSale)
            );
        }

        #[ink::test]
        fn register_contract_works() {
            let mut marketplace = init_contract();
            let ipfs = String::from("ipfs");

            assert!(marketplace
                .register(contract_address(), fee_recipient(), 999, ipfs.clone())
                .is_ok());
            let contract = marketplace
                .get_registered_collection(contract_address())
                .unwrap();
            assert_eq!(contract.royalty_receiver, fee_recipient());
            assert_eq!(contract.royalty, 999);
            assert_eq!(contract.marketplace_ipfs, ipfs);
        }

        #[ink::test]
        fn register_fails_if_fee_too_high() {
            let mut marketplace = init_contract();
            let ipfs = String::from("ipfs");

            assert_eq!(
                marketplace.register(contract_address(), fee_recipient(), 1001, ipfs.clone()),
                Err(MarketplaceError::FeeTooHigh)
            );
            assert!(marketplace
                .register(contract_address(), fee_recipient(), 999, ipfs)
                .is_ok());
        }

        #[ink::test]
        fn register_fails_if_contract_already_registered() {
            let mut marketplace = init_contract();
            let ipfs = String::from("ipfs");

            assert!(marketplace
                .register(contract_address(), fee_recipient(), 999, ipfs.clone())
                .is_ok());
            assert_eq!(
                marketplace.register(contract_address(), fee_recipient(), 999, ipfs),
                Err(MarketplaceError::ContractAlreadyRegistered)
            );
        }

        #[ink::test]
        fn set_nft_contract_hash_works() {
            let mut marketplace = init_contract();
            let hash = Hash::try_from([1; 32]).unwrap();

            assert!(marketplace.set_nft_contract_hash(hash).is_ok());
            assert_eq!(marketplace.nft_contract_hash(), hash);
        }

        #[ink::test]
        fn set_nft_contract_fails_if_not_owner() {
            let mut marketplace = init_contract();
            let hash = Hash::try_from([1; 32]).unwrap();
            let accounts = default_accounts();
            set_sender(accounts.bob);

            assert_eq!(
                marketplace.set_nft_contract_hash(hash),
                Err(MarketplaceError::OwnableError(
                    OwnableError::CallerIsNotOwner
                ))
            );
        }

        #[ink::test]
        fn factory_fails_if_no_hash() {
            let mut marketplace = init_contract();
            let accounts = default_accounts();

            assert_eq!(
                marketplace.factory(
                    String::from("ipfs"),
                    accounts.alice,
                    100,
                    String::from("name"),
                    String::from("symbol"),
                    String::from("base_uri"),
                    0,
                    0,
                ),
                Err(MarketplaceError::NftContractHashNotSet)
            );
        }

        fn init_contract() -> MarketplaceContract {
            MarketplaceContract::new(fee_recipient())
        }

        fn default_accounts() -> test::DefaultAccounts<ink_env::DefaultEnvironment> {
            test::default_accounts::<Environment>()
        }

        fn set_sender(sender: AccountId) {
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(sender);
        }

        fn fee_recipient() -> AccountId {
            AccountId::from([0x1; 32])
        }

        fn contract_address() -> AccountId {
            AccountId::from([0x2; 32])
        }
    }
}
