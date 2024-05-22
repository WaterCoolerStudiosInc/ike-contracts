#![cfg_attr(not(feature = "std"), no_std, no_main)]

mod staking{
    use psp22::{PSP22Burnable, PSP22};
    use psp34::{PSP34,Id}
    use governance_nft::{GovernanceNFT}

    #[ink(storage)]
    pub struct Staking {
        owner:AccountId,
        governance_token:AccountId,
        nft:AccountId,
        governance_nfts:Mapping::<AccountId,Vec<u128>>
    }
    impl Staking {
        fn transfer_psp22_from(
            &self,
            from: &AccountId,
            to: &AccountId,
            amount: Balance,
        ) -> Result<(), VaultError> {
            let mut token: contract_ref!(PSP22) = self.governance_token.into();
            if let Err(e) = token.transfer_from(*from, *to, amount, Vec::new()) {
                return Err(VaultError::TokenError(e));
            }
            Ok(())
        }
        fn burn_psp34(
            &self,
            from: &AccountId,
            id:Id
        ) -> Result<(), VaultError> {
            let mut token: contract_ref!(PSP34) = self.nft.into();
            if let Err(e) = token.burn(*from,id) {
                return Err(VaultError::TokenError(e));
            }
            Ok(())
        }
        #[ink(constructor)]
        pub fn new(
            governance_token: AccountId,
            governance_nft_hash: Hash,
        ) -> Self {
            let caller = Self::env().caller();
            let now = Self::env().block_timestamp();

            let nft_ref = NFTRef::new()
                .endowment(0)
                .code_hash(governance_nft_hash)
                .salt_bytes(
                    &[9_u8.to_le_bytes().as_ref(), caller.as_ref()].concat()[..4],
                )
                .instantiate();
          

            Self {
                data: VaultData::new(
                    caller,
                    governance_token,
                    TokenRef::to_account_id(nft_ref),
                ),
            }
        }
}