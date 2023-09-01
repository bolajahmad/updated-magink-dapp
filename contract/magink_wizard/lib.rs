#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[openbrush::implementation(
    PSP34,
    PSP34Mintable,
    Ownable,
    PSP34Metadata,
    PSP34Enumerable
)]
#[openbrush::contract]
pub mod magink_wizard {
    use openbrush::traits::{Storage, String};

    #[ink(storage)]
    #[derive(Storage, Default)]
    pub struct MaginkWizardContract {
        #[storage_field]
        psp34: psp34::Data,
        #[storage_field]
        ownable: ownable::Data,
        #[storage_field]
        metadata: metadata::Data,
        #[storage_field]
        enumerable: enumerable::Data,
    }

    impl MaginkWizardContract {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new() -> Self {
            let mut instance = Self::default();
            let collection_id = psp34::PSP34Impl::collection_id(&instance);
            let caller = Self::env().caller();
            ownable::Internal::_init_with_owner(&mut instance, caller);
            metadata::Internal::_set_attribute(
                &mut instance,
                collection_id.clone(),
                String::from("name"),
                String::from("MaginkWizardContract"),
            );
            metadata::Internal::_set_attribute(
                &mut instance,
                collection_id.clone(),
                String::from("symbol"),
                String::from("SH34"),
            );
            // psp34::Internal::_mint_to(&mut instance, caller, Id::U8(1))
            //     .expect("Should mint");
            instance
        }

        #[ink(message)]
        pub fn mint(
            &mut self,
            account: AccountId,
            id: Vec<u8>,
        ) -> Result<(), PSP34Error> {
            psp34::InternalImpl::_mint_to(self, account, Id::Bytes(id.to_vec()))
        }
    }
}
