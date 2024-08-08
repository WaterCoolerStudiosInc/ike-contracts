#[cfg(test)]
mod sources;

#[cfg(test)]
mod helpers;

#[cfg(test)]
mod tests {
    use crate::helpers::{
        DAY,
        call_function,
        query_allowance,
        query_owner,
        query_proposal,
        query_token_balance,
        update_days,
        gov_token_transfer,
    };
    use crate::sources::*;
    use drink::{
        AccountId32,
        chain_api::ChainApi,
        runtime::MinimalRuntime,
        session::{NO_ARGS, Session},
    };
    use std::error::Error;
    use std::fmt;

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TokenTransfer {
        token: AccountId32,
        amount: u128,
        to: AccountId32,
    }
    #[derive(Debug, PartialEq, Eq, scale::Encode, Clone, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum PropType {
        TransferFunds(TokenTransfer),
        UpdateStakingRewards(u128),
        AddCouncilMember(AccountId32),
        RemoveCouncilMember(AccountId32),
        ThresholdChange(u16),
        FeeChange(u16),
        VoteDelayUpdate(u64),
        VotePeriodUpdate(u64),
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Proposal {
        pub creation_timestamp: u64,
        pub creator_id: u128,
        pub prop_id: String,
        pub prop_type: PropType,
        pub pro_vote_count: u128,
        pub con_vote_count: u128,
        pub vote_start: u64,
        pub vote_end: u64,
    }
    impl fmt::Display for PropType {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{:?}", self)
            // or, alternatively:
            // fmt::Debug::fmt(self, f)
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    pub struct GovernanceData {
        pub block_created: u64,
        pub vote_weight: u128,
    }
    const TOTAL_SUPPLY: u128 = 100_000_000_000_000_000_u128;
    const ACC_THRESHOLD: u128 = TOTAL_SUPPLY / 20;
    const REJECT_THRESHOLD: u128 = TOTAL_SUPPLY / 10;
    const EXEC_THRESHOLD: u128 = TOTAL_SUPPLY / 10;
    const USER_SUPPLY: u128 = TOTAL_SUPPLY / 10;
    const REWARDS_PER_SECOND: u128 = 100_000u128;

    struct TestContext {
        sess: Session<MinimalRuntime>,
        gov_token: AccountId32,
        gov_nft: AccountId32,
        stake_contract: AccountId32,
        governance: AccountId32,
        vault: AccountId32,
        vesting: AccountId32,
        alice: AccountId32,
        bob: AccountId32,
        charlie: AccountId32,
        dave: AccountId32,
        ed: AccountId32,
    }

    fn setup() -> Result<TestContext, Box<dyn Error>> {
        let bob = AccountId32::new([1u8; 32]);
        let alice = AccountId32::new([2u8; 32]);
        let charlie = AccountId32::new([3u8; 32]);
        let dave = AccountId32::new([4u8; 32]);
        let ed = AccountId32::new([5u8; 32]);

        let mut sess: Session<MinimalRuntime> = Session::<MinimalRuntime>::new().unwrap();

        sess.chain_api().add_tokens(alice.clone(), 100_000_000e10 as u128);
        sess.chain_api().add_tokens(bob.clone(), 100_000_000e10 as u128);
        sess.chain_api().add_tokens(charlie.clone(), 100_000_000e10 as u128);
        sess.chain_api().add_tokens(dave.clone(), 100_000_000e10 as u128);
        sess.chain_api().add_tokens(ed.clone(), 100_000_000e10 as u128);

        sess.upload(bytes_governance_nft())?;
        sess.upload(bytes_registry())?;
        sess.upload(bytes_share_token())?;
        sess.upload(bytes_multisig())?;
        sess.upload(bytes_governance_staking())?;
        sess.upload(bytes_vesting())?;

        let gov_token = sess.deploy(
            bytes_governance_token(),
            "new",
            NO_ARGS,
            vec![2],
            None,
            &transcoder_governance_token().unwrap(),
        )?;
        sess.set_transcoder(gov_token.clone(), &transcoder_governance_token().unwrap());
        println!("gov_token: {:?}", gov_token.to_string());

        let vesting = sess.deploy(
            bytes_vesting(),
            "new",
            &[gov_token.to_string()],
            vec![1],
            None,
            &transcoder_vesting().unwrap(),
        )?;
        sess.set_transcoder(vesting.clone(), &transcoder_vesting().unwrap());
        println!("vesting: {:?}", vesting.to_string());


        let vault = sess.deploy(
            bytes_vault(),
            "new",
            &[hash_share_token(), hash_registry(), hash_nominator()],
            vec![1],
            None,
            &transcoder_vault().unwrap(),
        )?;
        sess.set_transcoder(vault.clone(), &transcoder_vault().unwrap());
        println!("vault: {:?}", vault.to_string());

        let mut sess = call_function(
            sess,
            &vault,
            &bob,
            String::from("IVault::get_registry_contract"),
            None,
            None,
            transcoder_vault(),
        )
        .unwrap();
        let rr: Result<AccountId32, drink::errors::LangError> = sess.last_call_return().unwrap();
        let registry = rr.unwrap();
        sess.set_transcoder(registry.clone(), &transcoder_registry().unwrap());
        println!("registry: {:?}", registry.to_string());

        let governance = sess.deploy(
            bytes_governance(),
            "new",
            &[
                vault.to_string(),
                registry.to_string(),
                gov_token.to_string(),
                hash_multisig(),
                hash_governance_nft(),
                hash_governance_staking(),
                EXEC_THRESHOLD.to_string(),
                REJECT_THRESHOLD.to_string(),
                ACC_THRESHOLD.to_string(),
                REWARDS_PER_SECOND.to_string(),
            ],
            vec![1],
            None,
            &transcoder_governance().unwrap(),
        )?;
        sess.set_transcoder(governance.clone(), &transcoder_governance().unwrap());
        println!("governance: {:?}", governance.to_string());

        let mut sess = call_function(
            sess,
            &governance,
            &bob,
            String::from("get_staking"),
            None,
            None,
            transcoder_governance(),
        )
        .unwrap();
        let rr: Result<AccountId32, drink::errors::LangError> = sess.last_call_return().unwrap();
        let stake_contract = rr.unwrap();
        sess.set_transcoder(stake_contract.clone(), &transcoder_governance().unwrap());
        println!("stake_contract: {:?}", stake_contract.to_string());

        let mut sess = call_function(
            sess,
            &governance,
            &bob,
            String::from("get_multisig"),
            None,
            None,
            transcoder_governance(),
        )
        .unwrap();
        let rr: Result<AccountId32, drink::errors::LangError> = sess.last_call_return().unwrap();
        let multisig = rr.unwrap();
        sess.set_transcoder(multisig.clone(), &transcoder_multisig().unwrap());
        println!("multisig: {:?}", stake_contract.to_string());

        let mut sess = call_function(
            sess,
            &stake_contract,
            &bob,
            String::from("get_governance_nft"),
            None,
            None,
            transcoder_governance_staking(),
        )
        .unwrap();
        let rr: Result<AccountId32, drink::errors::LangError> = sess.last_call_return().unwrap();
        let gov_nft = rr.unwrap();
        sess.set_transcoder(gov_nft.clone(), &transcoder_governance_nft().unwrap());
        println!("gov_nft: {:?}", gov_nft.to_string());

        let sess = gov_token_transfer(sess, &gov_token, &bob, &stake_contract, TOTAL_SUPPLY/10)?;
        let sess = gov_token_transfer(sess, &gov_token, &bob, &alice, USER_SUPPLY)?;
        let sess = gov_token_transfer(sess, &gov_token, &bob, &charlie, USER_SUPPLY)?;
        let sess = gov_token_transfer(sess, &gov_token, &bob, &dave, USER_SUPPLY)?;
        let sess = gov_token_transfer(sess, &gov_token, &bob, &ed, USER_SUPPLY)?;

        Ok(TestContext {
            sess,
            gov_token,
            gov_nft,
            stake_contract,
            governance,
            vault,
            vesting,
            alice,
            bob,
            charlie,
            dave,
            ed,
        })
    }

    //Alice id 1
    //Bob id 2
    //Charlie idi 3
    //dave id 4
    //ed id 5
    fn wrap_tokens(mut ctx: TestContext, amount: u128) -> Result<TestContext, Box<dyn Error>> {
        let mut sess = call_function(
            ctx.sess,
            &ctx.gov_token,
            &ctx.alice,
            String::from("PSP22::approve"),
            Some(vec![ctx.stake_contract.to_string(), amount.to_string()]),
            None,
            transcoder_governance_token(),
        )
        .unwrap();
        let mut sess = call_function(
            sess,
            &ctx.stake_contract,
            &ctx.alice,
            String::from("wrap_tokens"),
            Some(vec![amount.to_string(), "None".to_string()]),
            None,
            transcoder_governance_staking(),
        )
        .unwrap();
        let mut sess = call_function(
            sess,
            &ctx.gov_token,
            &ctx.bob,
            String::from("PSP22::approve"),
            Some(vec![ctx.stake_contract.to_string(), amount.to_string()]),
            None,
            transcoder_governance_token(),
        )
        .unwrap();
        let mut sess = call_function(
            sess,
            &ctx.stake_contract,
            &ctx.bob,
            String::from("wrap_tokens"),
            Some(vec![amount.to_string(), "None".to_string()]),
            None,
            transcoder_governance_staking(),
        )
        .unwrap();
        let mut sess = call_function(
            sess,
            &ctx.gov_token,
            &ctx.charlie,
            String::from("PSP22::approve"),
            Some(vec![ctx.stake_contract.to_string(), amount.to_string()]),
            None,
            transcoder_governance_token(),
        )
        .unwrap();
        let mut sess = call_function(
            sess,
            &ctx.stake_contract,
            &ctx.charlie,
            String::from("wrap_tokens"),
            Some(vec![amount.to_string(), "None".to_string()]),
            None,
            transcoder_governance_staking(),
        )
        .unwrap();
        let mut sess = call_function(
            sess,
            &ctx.gov_token,
            &ctx.dave,
            String::from("PSP22::approve"),
            Some(vec![ctx.stake_contract.to_string(), amount.to_string()]),
            None,
            transcoder_governance_token(),
        )
        .unwrap();
        let mut sess = call_function(
            sess,
            &ctx.stake_contract,
            &ctx.dave,
            String::from("wrap_tokens"),
            Some(vec![amount.to_string(), "None".to_string()]),
            None,
            transcoder_governance_staking(),
        )
        .unwrap();
        let mut sess = call_function(
            sess,
            &ctx.gov_token,
            &ctx.ed,
            String::from("PSP22::approve"),
            Some(vec![ctx.stake_contract.to_string(), amount.to_string()]),
            None,
            transcoder_governance_token(),
        )
        .unwrap();
        let mut sess = call_function(
            sess,
            &ctx.stake_contract,
            &ctx.ed,
            String::from("wrap_tokens"),
            Some(vec![amount.to_string(), "None".to_string()]),
            None,
            transcoder_governance_staking(),
        )
        .unwrap();
        ctx.sess = sess;
        Ok(ctx)
    }

    #[test]
    fn test_mint_update() -> Result<(), Box<dyn Error>> {
        let ctx = setup().unwrap();
        // Bob approves Ed to transfer 1k sAZERO
        let mut sess = call_function(
            ctx.sess,
            &ctx.gov_token,
            &ctx.bob,
            String::from("PSP22::approve"),
            Some(vec![
                ctx.stake_contract.to_string(),
                TOTAL_SUPPLY.to_string(),
            ]),
            None,
            transcoder_governance_token(),
        )
        .unwrap();

        let sess = call_function(
            sess,
            &ctx.stake_contract,
            &ctx.bob,
            String::from("wrap_tokens"),
            Some(vec![(TOTAL_SUPPLY / 10).to_string(), "None".to_string()]),
            None,
            transcoder_governance_staking(),
        )
        .unwrap();

        let mut sess = call_function(
            sess,
            &ctx.gov_nft,
            &ctx.bob,
            String::from("PSP34::total_supply"),
            Some(vec![]),
            None,
            transcoder_governance_nft(),
        )
        .unwrap();
        let rr: Result<u128, drink::errors::LangError> = sess.last_call_return().unwrap();
        let total_supply = rr.unwrap();
        println!("{:?}", total_supply);
        let sess = call_function(
            sess,
            &ctx.stake_contract,
            &ctx.bob,
            String::from("add_token_value"),
            Some(vec![5000_u128.to_string(), 1_u128.to_string()]),
            None,
            transcoder_governance_staking(),
        )
        .unwrap();
       
        let sess = call_function(
            sess,
            &ctx.gov_nft,
            &ctx.bob,
            String::from("get_governance_data"),
            Some(vec![1_u128.to_string()]),
            None,
            transcoder_governance_nft(),
        )
        .unwrap();
        let gdata: Result<GovernanceData, drink::errors::LangError> =
            sess.last_call_return().unwrap();
        println!("{:?}", gdata);
        let expected = (TOTAL_SUPPLY / 10) + 5000;
        assert_eq!(gdata.unwrap().vote_weight, expected);
        Ok(())
    }
    #[test]
    fn test_burn_remint() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup().unwrap();
        ctx = wrap_tokens(ctx, USER_SUPPLY).unwrap();
        let sess = call_function(
            ctx.sess,
            &ctx.gov_nft,
            &ctx.alice,
            String::from("get_governance_data"),
            Some(vec![1_u128.to_string()]),
            None,
            transcoder_governance_nft(),
        )
        .unwrap();
        let sess = call_function(
            sess,
            &ctx.gov_nft,
            &ctx.alice,
            String::from("PSP34::approve"),
            Some(vec![
                (&ctx.stake_contract).to_string(),
                String::from("None"),
                true.to_string(),
            ]),
            None,
            transcoder_governance_nft(),
        )
        .unwrap();
        let (allowed, sess) =
            query_allowance(sess, &ctx.gov_nft, &ctx.alice, &ctx.stake_contract).unwrap();
        println!("{:?}", allowed);
        let (result,sess)=query_owner(sess,ctx.gov_nft,1_u128).unwrap();
        println!("{:?}", result);
        let sess = update_days(sess, 2);
        let sess = call_function(
            sess,
            &ctx.stake_contract,
            &ctx.alice,
            String::from("create_unwrap_request"),
            Some(vec![1_u128.to_string()]),
            None,
            transcoder_governance_staking(),
        )
        .unwrap();

        let sess = update_days(sess, 14);
        let sess = call_function(
            sess,
            &ctx.stake_contract,
            &ctx.alice,
            String::from("complete_request"),
            Some(vec![1_u128.to_string()]),
            None,
            transcoder_governance_staking(),
        )
        .unwrap();

        let (balance_in_wallet, sess) = query_token_balance(sess, &ctx.gov_token, &ctx.alice).unwrap();
        let (balance_in_staking, sess) = query_token_balance(sess, &ctx.gov_token, &ctx.stake_contract).unwrap();
        let total_rewards_2_days = REWARDS_PER_SECOND * 2 * DAY as u128;
        let rewards_share_alice = total_rewards_2_days / 5;
        assert_eq!(balance_in_wallet, USER_SUPPLY + rewards_share_alice);
        assert_eq!(balance_in_staking, (TOTAL_SUPPLY / 10) + (4 * USER_SUPPLY) - rewards_share_alice);

        Ok(())
    }
    #[test]
    fn earn_interest() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup().unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 5).unwrap();

        Ok(())
    }
    #[test]
    fn change_interest_rate_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup().unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();
        let prop = PropType::UpdateStakingRewards(70000000_128);
        println!("{:?}", prop.to_string());
        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![prop.to_string(), 1.to_string()]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        println!("{}", "Querying Proposal");
        let (res, sess) = query_proposal(sess, ctx.governance.clone(), 1_u128).unwrap();
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.bob,
            String::from("vote"),
            Some(vec![
                res.prop_id.to_string(),
                2.to_string(),
                true.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (res, sess) = query_proposal(sess, ctx.governance.clone(), 1_u128).unwrap();
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.charlie,
            String::from("vote"),
            Some(vec![
                res.prop_id.to_string(),
                3.to_string(),
                true.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        Ok(())
    }
    #[test]
    fn make_and_vote_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup().unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 5).unwrap();

        Ok(())
    }
    #[test]
    fn double_proposals_fail() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup().unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 5).unwrap();

        Ok(())
    }
    #[test]
    fn double_votes_fail() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup().unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 5).unwrap();

        Ok(())
    }
    #[test]
    fn proposal_creation() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup().unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        Ok(())
    }
}
