#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod magink {
    use crate::ensure;
    use ink::env::call::{build_call, ExecutionInput, Selector};
    use ink::env::DefaultEnvironment;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[ink(event)]
    pub struct EraStarted {
        #[ink(topic)]
        account: AccountId,
        #[ink(topic)]
        era: u8,
        start_block: u32,
    }

    #[ink(event)]
    pub struct BadgeClaimed {
        #[ink(topic)]
        account: AccountId,
        claim_block: u32,
    }

    #[ink(event)]
    pub struct NFTClaimed {
        #[ink(topic)]
        account: AccountId,
        cid: Vec<u8>,
        mint_block: u32,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        TooEarlyToClaim,
        UserNotFound,
        LessonsNotCompleted,
        BadgeMintingFailed,
        AlreadyClaimedCompletionBadge,
    }

    #[ink(storage)]
    pub struct Magink {
        user: Mapping<AccountId, Profile>,
        wizard_contract: AccountId,
    }
    #[derive(
        Debug, PartialEq, Eq, PartialOrd, Ord, Clone, scale::Encode, scale::Decode,
    )]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Profile {
        // duration in blocks until next claim
        claim_era: u8,
        // block number of last claim
        start_block: u32,
        // number of badges claimed
        badges_claimed: u8,
    }

    impl Magink {
        /// Creates a new Magink smart contract.
        #[ink(constructor)]
        pub fn new(wizard_account: AccountId) -> Self {
            Self {
                user: Mapping::new(),
                wizard_contract: wizard_account,
            }
        }

        /// (Re)Start the Magink the claiming era for the caller.
        #[ink(message)]
        pub fn start(&mut self, era: u8) {
            let profile = Profile {
                claim_era: era,
                start_block: self.env().block_number(),
                badges_claimed: 0,
            };
            self.user.insert(self.env().caller(), &profile);
            self.env().emit_event(EraStarted {
                account: self.env().caller(),
                era,
                start_block: self.env().block_number(),
            })
        }

        /// Claim the badge after the era.
        #[ink(message)]
        pub fn claim(&mut self) -> Result<(), Error> {
            ensure!(self.get_remaining() == 0, Error::TooEarlyToClaim);
            let caller = self.env().caller();
            let block_number = self.env().block_number();

            // update profile
            let mut profile = self.get_profile().ok_or(Error::UserNotFound).unwrap();
            profile.badges_claimed += 1;
            profile.start_block = block_number;
            self.user.insert(caller, &profile);
            self.env().emit_event(BadgeClaimed {
                account: caller,
                claim_block: block_number,
            });
            Ok(())
        }

        /// Returns the remaining blocks in the era.
        #[ink(message)]
        pub fn get_remaining(&self) -> u8 {
            let current_block = self.env().block_number();
            let caller = self.env().caller();
            self.user.get(&caller).map_or(0, |profile| {
                if current_block - profile.start_block >= profile.claim_era as u32 {
                    return 0;
                }
                profile.claim_era - (current_block - profile.start_block) as u8
            })
        }

        /// Mints a wizard NFT for completed profile.
        #[ink(message)]
        pub fn mint_wizard(&mut self, metadata: Vec<u8>) -> Result<(), Error> {
            let caller = self.env().caller();
            let badges_for = self.get_badges_for(caller);
            ensure!(badges_for >= 9, Error::LessonsNotCompleted);
            let nfts_owned = self.get_nfts_for(caller).unwrap_or(0);
            ensure!(nfts_owned == 0, Error::AlreadyClaimedCompletionBadge);

            // claim a badge for the new user
            self.claim().unwrap();

            let mint_result = build_call::<DefaultEnvironment>()
                .call(self.wizard_contract)
                .gas_limit(0)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("mint")))
                        .push_arg(&caller)
                        .push_arg(&metadata),
                )
                .returns::<()>()
                .try_invoke();

            match mint_result {
                Ok(Ok(_)) => {
                    self.env().emit_event(NFTClaimed {
                        account: caller,
                        cid: metadata,
                        mint_block: self.env().block_number(),
                    });
                    Ok(())
                }
                _ => Err(Error::BadgeMintingFailed),
            }
        }

        /// Returns the remaining blocks in the era for the given account.
        #[ink(message)]
        pub fn get_remaining_for(&self, account: AccountId) -> u8 {
            let current_block = self.env().block_number();
            self.user.get(&account).map_or(0, |profile| {
                if current_block - profile.start_block >= profile.claim_era as u32 {
                    return 0;
                }
                profile.claim_era - (current_block - profile.start_block) as u8
            })
        }

        /// Returns the profile of the given account.
        #[ink(message)]
        pub fn get_account_profile(&self, account: AccountId) -> Option<Profile> {
            self.user.get(&account)
        }

        /// Returns the profile of the caller.
        #[ink(message)]
        pub fn get_profile(&self) -> Option<Profile> {
            let caller = self.env().caller();
            self.user.get(&caller)
        }

        /// Returns the badge of the caller.
        #[ink(message)]
        pub fn get_badges(&self) -> u8 {
            self.get_profile()
                .map_or(0, |profile| profile.badges_claimed)
        }

        /// Returns the badge count of the given account.
        #[ink(message)]
        pub fn get_badges_for(&self, account: AccountId) -> u8 {
            self.get_account_profile(account)
                .map_or(0, |profile| profile.badges_claimed)
        }

        // Returns the total supply of wizard NFTs that have been minted.
        #[ink(message)]
        pub fn get_total_wizard_supply(&self) -> u128 {
            let result = build_call::<DefaultEnvironment>()
                .call(self.wizard_contract)
                .gas_limit(0)
                .exec_input(ExecutionInput::new(Selector::new(ink::selector_bytes!(
                    "PSP34::total_supply"
                ))))
                .returns::<u128>()
                .try_invoke();

            match result {
                Ok(Ok(value)) => value,
                _ => 0,
            }
        }

        fn get_nfts_for(&self, user: AccountId) -> Result<u32, ()> {
            let result = build_call::<DefaultEnvironment>()
                .call(self.wizard_contract)
                .gas_limit(0)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!(
                        "PSP34::balance_of"
                    )))
                    .push_arg(&user),
                )
                .returns::<u32>()
                .try_invoke();

            match result {
                Ok(Ok(value)) => Ok(value),
                _ => Ok(0),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn start_works() {
            let mut magink = Magink::new(AccountId::from([0x01; 32]));
            println!("get {:?}", magink.get_remaining());
            magink.start(10);
            assert_eq!(10, magink.get_remaining());
            advance_block();
            assert_eq!(9, magink.get_remaining());
        }

        #[ink::test]
        fn claim_works() {
            const ERA: u32 = 10;
            let accounts = default_accounts();
            let mut magink = Magink::new(AccountId::from([0x01; 32]));
            magink.start(ERA as u8);
            advance_n_blocks(ERA - 1);
            assert_eq!(1, magink.get_remaining());

            // claim fails, too early
            assert_eq!(Err(Error::TooEarlyToClaim), magink.claim());

            // claim succeeds
            advance_block();
            assert_eq!(Ok(()), magink.claim());
            assert_eq!(1, magink.get_badges());
            assert_eq!(1, magink.get_badges_for(accounts.alice));
            assert_eq!(1, magink.get_badges());
            assert_eq!(10, magink.get_remaining());

            // claim fails, too early
            assert_eq!(Err(Error::TooEarlyToClaim), magink.claim());
            advance_block();
            assert_eq!(9, magink.get_remaining());
            assert_eq!(Err(Error::TooEarlyToClaim), magink.claim());
        }

        fn default_accounts(
        ) -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<Environment>()
        }

        // fn set_sender(sender: AccountId) {
        //     ink::env::test::set_caller::<Environment>(sender);
        // }
        fn advance_n_blocks(n: u32) {
            for _ in 0..n {
                advance_block();
            }
        }
        fn advance_block() {
            ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
        }
    }

    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        use super::*;
        use crate::magink::MaginkRef;
        use ink::primitives::AccountId;
        use ink_e2e::build_message;
        use ink_e2e::subxt::client;
        use ink_e2e::subxt::tx::Signer;
        use magink_wizard::magink_wizard::MaginkWizardContractRef;
        use openbrush::contracts::ownable::ownable_external::Ownable;
        use openbrush::contracts::psp34::psp34_external::PSP34;

        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        const CLAIMING_ERA: u8 = 0;

        #[ink_e2e::test(additional_contracts = "magink_wizard/Cargo.toml")]
        async fn e2e_claiming_badges_works(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {
            // instantiate magink_wizard contract
            let token_constructor = MaginkWizardContractRef::new();
            let magink_wizard_address = client
                .instantiate(
                    "magink_wizard",
                    &ink_e2e::alice(),
                    token_constructor,
                    0,
                    None,
                )
                .await
                .expect("Failed to instantiate Wizard NFT contract")
                .account_id;

            // Instantiate magink contract
            let magink_constructor = MaginkRef::new(magink_wizard_address);
            let magink_address = client
                .instantiate("magink", &ink_e2e::alice(), magink_constructor, 0, None)
                .await
                .expect("Magink instantiate failed")
                .account_id;

            // update magink_wizard owner to magink co ntract
            let change_owner =
                build_message::<MaginkWizardContractRef>(magink_wizard_address.clone())
                    .call(|p| p.transfer_ownership(magink_address));
            client
                .call(&ink_e2e::alice(), change_owner, 0, None)
                .await
                .expect("Transfer of Wizard NFT ownership failed");

            // Verify that magink address now owns Wizard NFT
            let magink_wizard_owner =
                build_message::<MaginkWizardContractRef>(magink_wizard_address.clone())
                    .call(|p| p.owner());
            let owner_result = client
                .call_dry_run(&ink_e2e::alice(), &magink_wizard_owner, 0, None)
                .await
                .return_value();
            assert_eq!(owner_result.unwrap(), magink_address);

            // start the magink claiming_era.
            let start_message = build_message::<MaginkRef>(magink_address.clone())
                .call(|p| p.start(CLAIMING_ERA));
            client
                .call(&ink_e2e::bob(), start_message, 0, None)
                .await
                .expect("Calling `start` failed!");

            let bob_profile_message = build_message::<MaginkRef>(magink_address.clone())
                .call(|p| p.get_profile());
            let bob_profile = client
                .call_dry_run(&ink_e2e::bob(), &bob_profile_message, 0, None)
                .await
                .return_value();
            match bob_profile {
                Some(profile) => {
                    assert_eq!(
                        profile.badges_claimed, 0,
                        "Profile has not been instantiated"
                    );
                    assert_eq!(
                        profile.claim_era, CLAIMING_ERA,
                        "Claiming era is not correct"
                    );
                }
                None => panic!("Profile not found"),
            }

            // build the claim badge function call
            for x in 0..9 {
                let claim_message = build_message::<MaginkRef>(magink_address.clone())
                    .call(|p| p.claim());
                client
                    .call(&ink_e2e::bob(), claim_message, 0, None)
                    .await
                    .expect("Claiming a badge failed");
                advance_block();
            }

            let bob_profile = client
                .call_dry_run(&ink_e2e::bob(), &bob_profile_message, 0, None)
                .await
                .return_value();
            println!("bob_profile: {:?}", bob_profile);
            match bob_profile {
                Some(profile) => {
                    assert_eq!(
                        profile.badges_claimed, 9,
                        "Profile has not been instantiated"
                    );
                }
                None => panic!("Profile not found"),
            }

            // Users should be able to mint an NFT now for completing the course
            let sample_metadata =
                "bafybeibwbgwzqigw7touxmixxvkd3wfcf2rcljgbt75na7rwwnw4ojgljy";
            let mint_wizard_message = build_message::<MaginkRef>(magink_address.clone())
                .call(|p| p.mint_wizard(sample_metadata.as_bytes().to_vec()));
            client
                .call(&ink_e2e::bob(), mint_wizard_message, 0, None)
                .await
                .expect("Minting a wizard NFT failed");

            // If minting was successful, total supply should be 1 and,
            let magink_nft_total_supply =
                build_message::<MaginkWizardContractRef>(magink_wizard_address.clone())
                    .call(|p| p.total_supply());
            let total_supply_result = client
                .call_dry_run(&ink_e2e::bob(), &magink_nft_total_supply, 0, None)
                .await
                .return_value();
            println!(
                "Total supply of wizard NFT after 1st mint: {}",
                total_supply_result
            );
            assert_eq!(total_supply_result, 1);

            // let bob_nft_balance_message =
            //     build_message::<MaginkWizardContractRef>(magink_wizard_address.clone())
            //         .call(|p| {
            //             p.balance_of(
            //                 &ink_e2e::bob().address().public_key().to_account_id(),
            //             )
            //         });
            // let bob_balance = client
            //     .call_dry_run(&ink_e2e::bob(), &bob_nft_balance_message, 0, None)
            //     .await
            //     .return_value();
            // println!("Bob NFT balance, {}", bob_balance);
            assert_eq!(bob_balance, 1);
            Ok(())
        }

        fn advance_block() {
            ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
        }
    }
}

/// Evaluate `$x:expr` and if not true return `Err($y:expr)`.
///
/// Used as `ensure!(expression_to_ensure, expression_to_return_on_false)`.
#[macro_export]
macro_rules! ensure {
    ( $x:expr, $y:expr $(,)? ) => {{
        if !$x {
            return Err($y.into());
        }
    }};
}
