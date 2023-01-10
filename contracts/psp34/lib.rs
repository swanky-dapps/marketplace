#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]

#[openbrush::contract]
pub mod test_psp34 {
    // imports from ink!
    use ink_prelude::string::String;
    use ink_storage::traits::SpreadAllocate;

    // imports from openbrush
    use openbrush::{
        contracts::psp34::extensions::{
            burnable::*,
            enumerable::*,
            metadata::*,
            mintable::*,
        },
        traits::Storage,
    };

    #[ink(storage)]
    #[derive(Default, SpreadAllocate, Storage)]
    pub struct Psp34Contract {
        #[storage_field]
        psp34: psp34::Data<Balances>,
        #[storage_field]
        metadata: metadata::Data,
    }

    // Section contains default implementation without any modifications
    impl PSP34 for Psp34Contract {}
    impl PSP34Burnable for Psp34Contract {}
    impl PSP34Mintable for Psp34Contract {}
    impl PSP34Enumerable for Psp34Contract {}
    impl PSP34Metadata for Psp34Contract {}

    impl Psp34Contract {
        #[ink(constructor)]
        pub fn new() -> Self {
            ink_lang::codegen::initialize_contract(|instance: &mut Psp34Contract| {
                instance
                    ._mint_to(instance.env().caller(), Id::U8(1))
                    .expect("Can't mint");
                let collection_id = instance.collection_id();
                instance._set_attribute(
                    collection_id.clone(),
                    String::from("name").into_bytes(),
                    String::from("TestPSP34").into_bytes(),
                );
                instance._set_attribute(
                    collection_id,
                    String::from("symbol").into_bytes(),
                    String::from("TETS").into_bytes(),
                );
            })
        }
    }
}
