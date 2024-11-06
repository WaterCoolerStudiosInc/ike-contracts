#[cfg(test)]
mod sources;

#[cfg(test)]
mod helpers;

#[cfg(test)]
mod tests {
    use crate::helpers;
    use crate::helpers::{
        call_function, gov_token_transfer, query_allowance, query_governance_acceptance_threshold,
        query_governance_execution_threshold, query_governance_rejection_threshold,
        query_governance_vote_delay, query_governance_vote_period, query_owner,
        query_token_balance, transfer_role_admin, update_days, CastType, Vote, DAY,
    };
    use crate::sources::*;
    use drink::{
        chain_api::ChainApi,
        runtime::MinimalRuntime,
        session::{Session, NO_ARGS},
        AccountId32 as AccountId,
    };
    use std::error::Error;

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    pub struct GovernanceData {
        pub block_created: u64,
        pub vote_weight: u128,
        pub stake_weight: u128,
    }
    pub const BIPS: u128 = 10000000;
    const TOTAL_SUPPLY: u128 = 100_000_000_000_000_000_u128;
    const ACC_THRESHOLD: u128 = TOTAL_SUPPLY / 20;
    const REJECT_THRESHOLD: u128 = TOTAL_SUPPLY / 10;
    const EXEC_THRESHOLD: u128 = TOTAL_SUPPLY / 10;
    const USER_SUPPLY: u128 = TOTAL_SUPPLY / 10;
    const REWARDS_PER_SECOND: u128 = 100_000u128;
    struct TestContext {
        sess: Session<MinimalRuntime>,
        gov_token: AccountId,
        gov_nft: AccountId,
        stake_contract: AccountId,
        governance: AccountId,
        vault: AccountId,
        vesting: AccountId,
        alice: AccountId,
        bob: AccountId,
        charlie: AccountId,
        dave: AccountId,
        ed: AccountId,
        validators: Vec<AccountId>,
    }
    struct MultiSigCTX {
        sess: Session<MinimalRuntime>,
        registry: AccountId,
        multisig: AccountId,
        alice: AccountId,
        bob: AccountId,
        charlie: AccountId,
        dave: AccountId,
        ed: AccountId,
    }
    fn setup(
        acc_threshold: u128,
        reject_threshold: u128,
        exec_threshold: u128,
    ) -> Result<TestContext, Box<dyn Error>> {
        let bob = AccountId::new([1u8; 32]);
        let alice = AccountId::new([2u8; 32]);
        let charlie = AccountId::new([3u8; 32]);
        let dave = AccountId::new([4u8; 32]);
        let ed = AccountId::new([5u8; 32]);
        let validator1 = AccountId::new([101u8; 32]);
        let validator2 = AccountId::new([102u8; 32]);
        let validator3 = AccountId::new([103u8; 32]);
        let validator4 = AccountId::new([104u8; 32]);
        let validator5 = AccountId::new([105u8; 32]);
        let validator6 = AccountId::new([106u8; 32]);
        
        let mut sess: Session<MinimalRuntime> = Session::<MinimalRuntime>::new().unwrap();

        sess.chain_api()
            .add_tokens(alice.clone(), 100_000_000e10 as u128);
        sess.chain_api()
            .add_tokens(bob.clone(), 100_000_000e10 as u128);
        sess.chain_api()
            .add_tokens(charlie.clone(), 100_000_000e10 as u128);
        sess.chain_api()
            .add_tokens(dave.clone(), 100_000_000e10 as u128);
        sess.chain_api()
            .add_tokens(ed.clone(), 100_000_000e10 as u128);

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
        let rr: Result<AccountId, drink::errors::LangError> = sess.last_call_return().unwrap();
        let registry = rr.unwrap();
        sess.set_transcoder(registry.clone(), &transcoder_registry().unwrap());
        println!("registry: {:?}", registry.to_string());
        sess.set_actor(bob.clone());
        /**
         * sess: Session<MinimalRuntime>,
            registry: AccountId,
            sender: AccountId,
            admin: AccountId,
            validator: AccountId,
            pool_create_amount: u128,
            existential_deposit: u128,
         */
        let (_new_agent, sess) =
            helpers::call_add_agent(sess, registry.clone(), bob.clone(), bob.clone(), validator2.clone(), 100e12 as u128, 500)?;
        let (_new_agent, sess) =
            helpers::call_add_agent(sess, registry.clone(), bob.clone(), bob.clone(), validator1.clone(), 100e12 as u128, 500)?;
        let (_new_agent, sess) =
            helpers::call_add_agent(sess, registry.clone(), bob.clone(), bob.clone(), validator3.clone(), 100e12 as u128, 500)?;
        let (_new_agent, sess) =
            helpers::call_add_agent(sess, registry.clone(), bob.clone(), bob.clone(), validator4.clone(), 100e12 as u128, 500)?;
        let (_new_agent, mut sess) =
            helpers::call_add_agent(sess, registry.clone(), bob.clone(), bob.clone(), validator5.clone(), 100e12 as u128, 500)?;
        /**
        *    vault: AccountId,
           registry: AccountId,
           governance_token: AccountId,
           multisig_hash: Hash,
           gov_nft_hash: Hash,
           staking_hash: Hash,
           exec_threshold: u128,
           reject_threshold: u128,
           acc_threshold: u128,
           interest_rate: u128,
        */
        //acc_threshold:u128,reject_threshold:u128,exec_threshold
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
                exec_threshold.to_string(),
                reject_threshold.to_string(),
                acc_threshold.to_string(),
                REWARDS_PER_SECOND.to_string(),
            ],
            vec![1],
            None,
            &transcoder_governance().unwrap(),
        )?;
        sess.set_transcoder(governance.clone(), &transcoder_governance().unwrap());
        println!("governance: {:?}", governance.to_string());
        let sess = call_function(
            sess,
            &vault,
            &bob,
            String::from("IVault::transfer_role_adjust_fee"),
            Some([governance.clone().to_string()].to_vec()),
            None,
            helpers::transcoder_vault(),
        )
        .unwrap();
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
        let rr: Result<AccountId, drink::errors::LangError> = sess.last_call_return().unwrap();
        let stake_contract = rr.unwrap();
        let mut sess = helpers::transfer_role_admin(
            sess,
            &registry,
            &bob,
            &helpers::RoleType::UpdateAgents,
            &stake_contract,
        )
        .unwrap();
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
        let rr: Result<AccountId, drink::errors::LangError> = sess.last_call_return().unwrap();
        let multisig = rr.unwrap();
        sess.set_transcoder(multisig.clone(), &transcoder_multisig().unwrap());
        println!("multisig: {:?}", multisig.to_string());

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
        let rr: Result<AccountId, drink::errors::LangError> = sess.last_call_return().unwrap();
        let gov_nft = rr.unwrap();
        sess.set_transcoder(gov_nft.clone(), &transcoder_governance_nft().unwrap());
        println!("gov_nft: {:?}", gov_nft.to_string());

        let sess = gov_token_transfer(sess, &gov_token, &bob, &stake_contract, TOTAL_SUPPLY / 50)?;
        let sess = gov_token_transfer(sess, &gov_token, &bob, &governance, TOTAL_SUPPLY / 50)?;
        let sess = gov_token_transfer(sess, &gov_token, &bob, &alice, USER_SUPPLY)?;
        let sess = gov_token_transfer(sess, &gov_token, &bob, &charlie, USER_SUPPLY)?;
        let sess = gov_token_transfer(sess, &gov_token, &bob, &dave, USER_SUPPLY)?;
        let sess = gov_token_transfer(sess, &gov_token, &bob, &ed, USER_SUPPLY)?;
        let validators = vec![
            validator1, validator2, validator3, validator4, validator5, validator6,
        ];
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
            validators: validators,
        })
    }

    fn multi_sig_test_setup() -> Result<MultiSigCTX, Box<dyn Error>> {
        let bob = AccountId::new([1u8; 32]);
        let alice = AccountId::new([2u8; 32]);
        let charlie = AccountId::new([3u8; 32]);
        let dave = AccountId::new([4u8; 32]);
        let ed = AccountId::new([5u8; 32]);

        let mut sess: Session<MinimalRuntime> = Session::<MinimalRuntime>::new().unwrap();

        sess.chain_api()
            .add_tokens(alice.clone(), 100_000_000e10 as u128);
        sess.chain_api()
            .add_tokens(bob.clone(), 100_000_000e10 as u128);
        sess.chain_api()
            .add_tokens(charlie.clone(), 100_000_000e10 as u128);
        sess.chain_api()
            .add_tokens(dave.clone(), 100_000_000e10 as u128);
        sess.chain_api()
            .add_tokens(ed.clone(), 100_000_000e10 as u128);
        sess.upload(bytes_governance_nft())?;
        sess.upload(bytes_registry())?;
        sess.upload(bytes_share_token())?;
        sess.upload(bytes_multisig())?;
        sess.upload(bytes_governance_staking())?;
        sess.upload(bytes_vesting())?;
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
        let rr: Result<AccountId, drink::errors::LangError> = sess.last_call_return().unwrap();
        let registry = rr.unwrap();
        sess.set_transcoder(registry.clone(), &transcoder_registry().unwrap());
        println!("registry: {:?}", registry.to_string());
        let mut sess = call_function(
            sess,
            &registry,
            &bob,
            String::from("transfer_role"),
            Some(vec![]),
            None,
            transcoder_registry(),
        )
        .unwrap();
        let multisig = sess
            .deploy(
                bytes_multisig(),
                "new",
                &[alice.to_string(), registry.to_string(), bob.to_string()],
                vec![1],
                None,
                &transcoder_multisig().unwrap(),
            )
            .unwrap();
        let mut sess = call_function(
            sess,
            &multisig,
            &alice,
            String::from("add_signer"),
            Some(vec![alice.to_string()]),
            None,
            transcoder_multisig(),
        )
        .unwrap();
        let mut sess = call_function(
            sess,
            &multisig,
            &alice,
            String::from("add_signer"),
            Some(vec![bob.to_string()]),
            None,
            transcoder_multisig(),
        )
        .unwrap();
        let mut sess = call_function(
            sess,
            &multisig,
            &alice,
            String::from("add_signer"),
            Some(vec![charlie.to_string()]),
            None,
            transcoder_multisig(),
        )
        .unwrap();
        let mut sess = call_function(
            sess,
            &multisig,
            &alice,
            String::from("add_signer"),
            Some(vec![dave.to_string()]),
            None,
            transcoder_multisig(),
        )
        .unwrap();
        let mut sess = call_function(
            sess,
            &multisig,
            &alice,
            String::from("add_signer"),
            Some(vec![ed.to_string()]),
            None,
            transcoder_multisig(),
        )
        .unwrap();
        Ok(MultiSigCTX {
            sess,
            registry,
            multisig,
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
    //(ACC_THRESHOLD,REJECT_THRESHOLD,EXEC_THRESHOLD)
    fn wrap_tokens(mut ctx: TestContext, amount: u128) -> Result<TestContext, Box<dyn Error>> {
        let cast = CastType::Direct(vec![
            (ctx.validators[0].clone(), BIPS / 2),
            (ctx.validators[1].clone(), BIPS / 2),
        ]);
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
            Some(vec![
                amount.to_string(),
                "None".to_string(),
                cast.to_string(),
                "None".to_string(),
            ]),
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
            Some(vec![
                amount.to_string(),
                "None".to_string(),
                cast.to_string(),
                "None".to_string(),
            ]),
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
            Some(vec![
                amount.to_string(),
                "None".to_string(),
                cast.to_string(),
                "None".to_string(),
            ]),
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
            Some(vec![
                amount.to_string(),
                "None".to_string(),
                cast.to_string(),
                "None".to_string(),
            ]),
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
            Some(vec![
                amount.to_string(),
                "None".to_string(),
                cast.to_string(),
                "None".to_string(),
            ]),
            None,
            transcoder_governance_staking(),
        )
        .unwrap();
        ctx.sess = sess;
        Ok(ctx)
    }

    #[test]
    fn multi_sig() -> Result<(), Box<dyn Error>> {
        let ctx = multi_sig_test_setup();
        Ok(())
    }

    #[test]
    fn test_mint_update() -> Result<(), Box<dyn Error>> {
        let ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
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
        let cast = CastType::Direct(vec![
            (ctx.validators[0].clone(), BIPS / 2),
            (ctx.validators[1].clone(), BIPS / 2),
        ]);
        let sess = call_function(
            sess,
            &ctx.stake_contract,
            &ctx.bob,
            String::from("wrap_tokens"),
            Some(vec![
                (TOTAL_SUPPLY / 10).to_string(),
                "None".to_string(),
                cast.to_string(),
                "None".to_string(),
            ]),
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
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
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
        let (result, sess) = query_owner(sess, ctx.gov_nft, 1_u128).unwrap();
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

        let (balance_in_wallet, sess) =
            query_token_balance(sess, &ctx.gov_token, &ctx.alice).unwrap();
        let (balance_in_staking, sess) =
            query_token_balance(sess, &ctx.gov_token, &ctx.stake_contract).unwrap();
        let total_rewards_2_days = REWARDS_PER_SECOND * 2 * DAY as u128;
        let rewards_share_alice = total_rewards_2_days / 5;
        println!("{:?}{}", "alice rewards ", rewards_share_alice);
        println!("{}", USER_SUPPLY);
        assert_eq!(balance_in_wallet, USER_SUPPLY + rewards_share_alice);
        assert_eq!(
            balance_in_staking,
            (TOTAL_SUPPLY / 50) + (4 * USER_SUPPLY) - rewards_share_alice
        );

        Ok(())
    }
    #[test]
    fn earn_interest() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        Ok(())
    }
    #[test]
    fn nft_unlocks_fail_with_active_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::ChangeStakingRewardRate(70000000_128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();

        let (proposals, sess) = helpers::query_governance_get_all_proposals(sess, &ctx.governance)?;
        println!("all proposals: {:?}", proposals);

        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        println!("{:?}", proposal.clone().prop_id.to_string());
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
        match call_function(
            sess,
            &ctx.stake_contract,
            &ctx.alice,
            String::from("create_unwrap_request"),
            Some(vec![1_u128.to_string()]),
            None,
            transcoder_governance_staking(),
        ) {
            Ok(_) => panic!("Should panic because of a proposal resuse"),
            Err(_) => (),
        }

        Ok(())
    }
    #[test]
    fn nft_unlocks_work_with_expired_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::ChangeStakingRewardRate(70000000_128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();

        let (proposals, sess) = helpers::query_governance_get_all_proposals(sess, &ctx.governance)?;
        println!("all proposals: {:?}", proposals);

        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        println!("{:?}", proposal.clone().prop_id.to_string());

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

        let sess = update_days(sess, 30);

        let sess = call_function(
            sess,
            &ctx.stake_contract,
            &ctx.alice,
            String::from("create_unwrap_request"),
            Some(vec![1_u128.to_string()]),
            None,
            transcoder_governance_staking(),
        );
        Ok(())
    }
    #[test]
    fn change_interest_rate_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::ChangeStakingRewardRate(70000000_128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();

        let (proposals, sess) = helpers::query_governance_get_all_proposals(sess, &ctx.governance)?;
        println!("all proposals: {:?}", proposals);

        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        println!("{:?}", proposal.clone().prop_id.to_string());
        let sess = update_days(sess, 3_u64);
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.bob,
            String::from("vote"),
            Some(vec![
                proposal.prop_id.to_string(),
                2.to_string(),
                Vote::Pro.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();

        Ok(())
    }
    #[test]
    fn cancel_proposal_works() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::ChangeStakingRewardRate(70000000_128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();

        let (proposals, sess) = helpers::query_governance_get_all_proposals(sess, &ctx.governance)?;
        println!("all proposals: {:?}", proposals);

        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        println!("{:?}", proposal.clone().prop_id.to_string());
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.alice,
            String::from("cancel_proposal"),
            Some(vec![1.to_string()]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        Ok(())
    }
    #[test]
    fn cancel_proposal_fails_during_active_period() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::ChangeStakingRewardRate(70000000_128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();

        let (proposals, sess) = helpers::query_governance_get_all_proposals(sess, &ctx.governance)?;
        println!("all proposals: {:?}", proposals);

        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        let sess = update_days(sess, 7_u64);
        println!("{:?}", proposal.clone().prop_id.to_string());
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.alice,
            String::from("cancel_proposal"),
            Some(vec![1.to_string()]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        Ok(())
    }
    #[test]
    fn double_proposal_creation_fails() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::ChangeStakingRewardRate(70000000_128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();

        match call_function(
            sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::ChangeStakingRewardRate(70000000_128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        ) {
            Ok(_) => panic!("Should panic because of a proposal resuse"),
            Err(_) => (),
        }
        Ok(())
    }
    #[test]
    fn double_proposal_creation_after_expiry() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::ChangeStakingRewardRate(70000000_128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let sess = update_days(sess, 10_u64);

        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::ChangeStakingRewardRate(70000000_128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        Ok(())
    }
    #[test]
    fn proposal_creation_fails_with_invalid_nft_weight() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 40).unwrap();
        match call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::ChangeStakingRewardRate(70000000_128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        ) {
            Ok(_) => panic!("Should panic because of a proposal resuse"),
            Err(_) => (),
        }
        Ok(())
    }
    #[test]
    fn proposal_creation_fails_with_invalid_nft() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();
        match call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::ChangeStakingRewardRate(70000000_128).to_string(),
                3.to_string(),
            ]),
            None,
            transcoder_governance(),
        ) {
            Ok(_) => panic!("Should panic because of a proposal resuse"),
            Err(_) => (),
        }
        Ok(())
    }
    #[test]
    fn vault_fee_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();
        println!("{:?}", helpers::PropType::FeeChange(2333_u16).to_string());
        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::FeeChange(2333_u16).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();

        let (proposals, sess) = helpers::query_governance_get_all_proposals(sess, &ctx.governance)?;
        println!("all proposals: {:?}", proposals);

        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        println!("{:?}", proposal.clone().prop_id.to_string());
        let sess = update_days(sess, 3_u64);
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.bob,
            String::from("vote"),
            Some(vec![
                proposal.prop_id.to_string(),
                2.to_string(),
                Vote::Pro.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let sess = call_function(
            sess,
            &ctx.vault,
            &ctx.bob,
            String::from("IVault::get_fee_percentage"),
            None,
            None,
            helpers::transcoder_vault(),
        )
        .unwrap();
        let res: Result<u16, drink::errors::LangError> = sess.last_call_return().unwrap();
        assert_eq!(res.unwrap(), 2333);
        Ok(())
    }
    #[test]
    fn test_vote_delay_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();
        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::VoteDelayUpdate(3 * DAY).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        let sess = update_days(sess, 3_u64);
        println!("{:?}", proposal.clone().prop_id.to_string());
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.bob,
            String::from("vote"),
            Some(vec![
                proposal.prop_id.to_string(),
                2.to_string(),
                Vote::Pro.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (value, sess) = query_governance_vote_delay(sess, ctx.governance).unwrap();
        assert_eq!(3 * DAY, value);
        Ok(())
    }
    #[test]
    fn test_invalid_delay_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();
        match call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::VoteDelayUpdate(9 * DAY).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        ) {
            Ok(_) => panic!("Should panic because of invalid delay input"),
            Err(_) => (),
        }
        Ok(())
    }
    #[test]
    fn test_vote_period_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();
        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::VotePeriodUpdate(12 * DAY).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        let sess = update_days(sess, 3_u64);
        println!("{:?}", proposal.clone().prop_id.to_string());
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.bob,
            String::from("vote"),
            Some(vec![
                proposal.prop_id.to_string(),
                2.to_string(),
                Vote::Pro.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (value, sess) = query_governance_vote_period(sess, ctx.governance).unwrap();
        assert_eq!(12 * DAY, value);
        Ok(())
    }
    #[test]
    fn test_vote_invalid_period_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();
        match call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::VotePeriodUpdate(33 * DAY).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        ) {
            Ok(_) => panic!("Should panic because of invalid vote period input"),
            Err(_) => (),
        }

        Ok(())
    }
    //CompoundIncentiveChange(
    #[test]
    fn update_incentive_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();
        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::CompoundIncentiveChange(1000_u16).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        let sess = update_days(sess, 3_u64);
        println!("{:?}", proposal.clone().prop_id.to_string());
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.bob,
            String::from("vote"),
            Some(vec![
                proposal.prop_id.to_string(),
                2.to_string(),
                Vote::Pro.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        Ok(())
    }
    /**
    *   PropType::AcceptanceWeightUpdate(update) => {
                       self.update_acceptance_threshold(*update)
                   }
                   PropType::UpdateRejectThreshhold(update) => {
                       self.update_reject_threshold(*update)
                   }
                   PropType::UpdateExecThreshhold(update) => {
                       self.update_execution_threshold(*update)
                   }
    */
    #[test]
    fn acceptance_weight_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();
        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::AcceptanceWeightUpdate(100_000_000_999_u128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        let sess = update_days(sess, 3_u64);
        println!("{:?}", proposal.clone().prop_id.to_string());
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.bob,
            String::from("vote"),
            Some(vec![
                proposal.prop_id.to_string(),
                2.to_string(),
                Vote::Pro.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (value, sess) = query_governance_acceptance_threshold(sess, ctx.governance).unwrap();
        assert_eq!(100_000_000_999_u128, value.unwrap());
        println!("{:?}", value);
        Ok(())
    }
    #[test]
    fn rejection_threshold_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();
        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::UpdateRejectThreshhold(100_000_000_999_u128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        let sess = update_days(sess, 3_u64);
        println!("{:?}", proposal.clone().prop_id.to_string());
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.bob,
            String::from("vote"),
            Some(vec![
                proposal.prop_id.to_string(),
                2.to_string(),
                Vote::Pro.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (value, sess) = query_governance_rejection_threshold(sess, ctx.governance).unwrap();
        assert_eq!(100_000_000_999_u128, value.unwrap());
        Ok(())
    }
    #[test]
    fn execution_threshold_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();
        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::UpdateExecThreshhold(100_000_000_999_u128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        let sess = update_days(sess, 3_u64);
        println!("{:?}", proposal.clone().prop_id.to_string());
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.bob,
            String::from("vote"),
            Some(vec![
                proposal.prop_id.to_string(),
                2.to_string(),
                Vote::Pro.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (value, sess) = query_governance_execution_threshold(sess, ctx.governance).unwrap();
        assert_eq!(100_000_000_999_u128, value.unwrap());
        Ok(())
    }
    #[test]
    fn transfer_funds_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();
        /**
        pub struct TokenTransfer {
            token: AccountId,
            amount: u128,
            to: AccountId,
        } */
        /*let transfer = helpers::TokenTransfer {
            token: ctx.gov_token,
            amount: (TOTAL_SUPPLY / 50),
            to: ctx.dave,
        };
        **/
        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::TransferFunds(ctx.gov_token, TOTAL_SUPPLY / 50, ctx.dave)
                    .to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();

        let (proposals, sess) = helpers::query_governance_get_all_proposals(sess, &ctx.governance)?;
        println!("all proposals: {:?}", proposals);

        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        println!("{:?}", proposal.clone().prop_id.to_string());
        let sess = update_days(sess, 3_u64);
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.bob,
            String::from("vote"),
            Some(vec![
                proposal.prop_id.to_string(),
                2.to_string(),
                Vote::Pro.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();

        Ok(())
    }
    #[test]
    fn transfer_native_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();
        println!(
            "{}",
            helpers::PropType::NativeTokenTransfer(ctx.dave.clone(), 100000000000_u128).to_string()
        );
        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::NativeTokenTransfer(ctx.dave, 100000000000_u128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();

        let (proposals, mut sess) =
            helpers::query_governance_get_all_proposals(sess, &ctx.governance)?;
        println!("all proposals: {:?}", proposals);
        sess.chain_api()
            .add_tokens(ctx.governance.clone(), 100_000_000e10 as u128);
        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        println!("{:?}", proposal.clone().prop_id.to_string());
        let sess = update_days(sess, 3_u64);
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.bob,
            String::from("vote"),
            Some(vec![
                proposal.prop_id.to_string(),
                2.to_string(),
                Vote::Pro.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();

        Ok(())
    }
    #[test]
    fn add_multisigner_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::AddCouncilMember(ctx.dave).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();

        let (proposals, sess) = helpers::query_governance_get_all_proposals(sess, &ctx.governance)?;
        println!("all proposals: {:?}", proposals);

        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        println!("{:?}", proposal.clone().prop_id.to_string());
        let sess = update_days(sess, 3_u64);
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.bob,
            String::from("vote"),
            Some(vec![
                proposal.prop_id.to_string(),
                2.to_string(),
                Vote::Pro.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();

        Ok(())
    }

    #[test]
    fn double_votes_fail() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD / 2, REJECT_THRESHOLD, EXEC_THRESHOLD / 2).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 40).unwrap();
        //ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();
        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::UpdateExecThreshhold(100_000_000_999_u128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        let sess = update_days(sess, 3_u64);
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.bob,
            String::from("vote"),
            Some(vec![
                proposal.prop_id.to_string(),
                2.to_string(),
                Vote::Pro.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        println!("proposal: {:?}", proposal);
        match call_function(
            sess,
            &ctx.governance,
            &ctx.bob,
            String::from("vote"),
            Some(vec![
                proposal.prop_id.to_string(),
                2.to_string(),
                Vote::Pro.to_string(),
            ]),
            None,
            transcoder_governance(),
        ) {
            Ok(_) => panic!("Should panic because of invalid vote period input"),
            Err(_) => (),
        }

        Ok(())
    }
    #[test]
    fn proposals_can_be_rejected() -> Result<(), Box<dyn Error>> {
        println!("{}{}", "the rejection threshold", REJECT_THRESHOLD / 2);
        let mut ctx = setup(ACC_THRESHOLD / 2, REJECT_THRESHOLD / 2, EXEC_THRESHOLD / 2).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 40).unwrap();
        //ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();
        let sess = call_function(
            ctx.sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::UpdateExecThreshhold(100_000_000_999_u128).to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        println!("proposal: {:?}", proposal);
        let sess = update_days(sess, 3_u64);
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.bob,
            String::from("vote"),
            Some(vec![
                proposal.prop_id.to_string(),
                2.to_string(),
                Vote::Con.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (proposal, sess) =
            helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        println!("proposal: {:?}", proposal);
        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.charlie,
            String::from("vote"),
            Some(vec![
                proposal.prop_id.to_string(),
                3.to_string(),
                Vote::Con.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();
        let (proposals, sess) = helpers::query_governance_get_all_proposals(sess, &ctx.governance)?;
        assert_eq!(proposals.len() as u128, 0_u128);

        Ok(())
    }
    #[test]
    fn proposal_creation() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        Ok(())
    }

    #[test]
    fn vesting_admin_can_transfer() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        // Bob is admin
        let (admin, sess) = helpers::query_vesting_get_admin(ctx.sess, &ctx.vesting)?;
        assert_eq!(admin.unwrap(), ctx.bob);

        // Transfer admin to Charlie
        let sess = call_function(
            sess,
            &ctx.vesting,
            &ctx.bob,
            String::from("admin_transfer"),
            Some(vec![ctx.charlie.to_string()]),
            None,
            transcoder_vesting(),
        )?;

        // Charlie is admin
        let (admin, _sess) = helpers::query_vesting_get_admin(sess, &ctx.vesting)?;
        assert_eq!(admin.unwrap(), ctx.charlie);

        Ok(())
    }

    #[test]
    fn vesting_admin_can_relinquish() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let sess = helpers::vesting_activate(ctx.sess, &ctx.vesting, &ctx.bob)?;

        let sess = call_function(
            sess,
            &ctx.vesting,
            &ctx.bob,
            String::from("admin_relinquish"),
            None,
            None,
            transcoder_vesting(),
        )?;

        let (admin, _sess) = helpers::query_vesting_get_admin(sess, &ctx.vesting)?;
        assert!(admin.is_none());

        Ok(())
    }

    #[test]
    fn vesting_admin_can_abort_contract() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let cliff = 100e12 as u128;

        let charlie_schedule = helpers::Schedule {
            amount: 0,
            cliff,
            offset: 0,
            duration: 0,
        };

        // Add funding
        let sess =
            helpers::gov_token_transfer(ctx.sess, &ctx.gov_token, &ctx.bob, &ctx.vesting, cliff)?;

        let sess = helpers::vesting_add_recipients(
            sess,
            &ctx.vesting,
            &ctx.bob,
            vec![&ctx.charlie],
            vec![&charlie_schedule],
        )?;

        // Abort
        let (admin_balance_before, sess) =
            helpers::query_token_balance(sess, &ctx.gov_token, &ctx.bob)?;
        let sess = call_function(
            sess,
            &ctx.vesting,
            &ctx.bob,
            String::from("admin_abort"),
            None,
            None,
            transcoder_vesting(),
        )?;
        let (admin_balance_after, sess) =
            helpers::query_token_balance(sess, &ctx.gov_token, &ctx.bob)?;
        let (contract_balance_after, _sess) =
            helpers::query_token_balance(sess, &ctx.gov_token, &ctx.vesting)?;
        assert_eq!(admin_balance_after, admin_balance_before + cliff);
        assert_eq!(contract_balance_after, 0);

        Ok(())
    }

    #[test]
    fn vesting_admin_can_add_recipient() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let (schedule, sess) =
            helpers::query_vesting_get_schedule(ctx.sess, &ctx.vesting, &ctx.charlie)?;
        assert!(schedule.is_none());

        let charlie_schedule = helpers::Schedule {
            amount: 0,
            cliff: 0,
            offset: 0,
            duration: 0,
        };

        let sess = helpers::vesting_add_recipients(
            sess,
            &ctx.vesting,
            &ctx.bob,
            vec![&ctx.charlie],
            vec![&charlie_schedule],
        )?;

        let (schedule, _sess) =
            helpers::query_vesting_get_schedule(sess, &ctx.vesting, &ctx.charlie)?;
        assert_eq!(schedule.unwrap(), charlie_schedule);

        Ok(())
    }

    #[test]
    fn vesting_admin_cannot_add_recipient_once_active() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let charlie_schedule = helpers::Schedule {
            amount: 0,
            cliff: 0,
            offset: 0,
            duration: 0,
        };

        let sess = helpers::vesting_activate(ctx.sess, &ctx.vesting, &ctx.bob)?;

        match helpers::vesting_add_recipients(
            sess,
            &ctx.vesting,
            &ctx.bob,
            vec![&ctx.charlie],
            vec![&charlie_schedule],
        ) {
            Ok(_) => panic!("Should panic because vesting has been activated"),
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn vesting_admin_cannot_add_duplicate_recipient() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let charlie_schedule = helpers::Schedule {
            amount: 0,
            cliff: 0,
            offset: 0,
            duration: 0,
        };

        let sess = helpers::vesting_add_recipients(
            ctx.sess,
            &ctx.vesting,
            &ctx.bob,
            vec![&ctx.charlie],
            vec![&charlie_schedule],
        )?;

        match helpers::vesting_add_recipients(
            sess,
            &ctx.vesting,
            &ctx.bob,
            vec![&ctx.charlie],
            vec![&charlie_schedule],
        ) {
            Ok(_) => panic!("Should panic because charlie already has schedule"),
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn vesting_non_admin_cannot_add_recipient() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let charlie_schedule = helpers::Schedule {
            amount: 0,
            cliff: 0,
            offset: 0,
            duration: 0,
        };

        match helpers::vesting_add_recipients(
            ctx.sess,
            &ctx.vesting,
            &ctx.ed, // not admin
            vec![&ctx.charlie],
            vec![&charlie_schedule],
        ) {
            Ok(_) => panic!("Should panic because caller is not admin"),
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn vesting_admin_can_remove_recipient() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let charlie_schedule = helpers::Schedule {
            amount: 0,
            cliff: 0,
            offset: 0,
            duration: 0,
        };

        let sess = helpers::vesting_add_recipients(
            ctx.sess,
            &ctx.vesting,
            &ctx.bob,
            vec![&ctx.charlie],
            vec![&charlie_schedule],
        )?;

        let (schedule, sess) =
            helpers::query_vesting_get_schedule(sess, &ctx.vesting, &ctx.charlie)?;
        assert_eq!(schedule.unwrap(), charlie_schedule);

        let sess =
            helpers::vesting_remove_recipients(sess, &ctx.vesting, &ctx.bob, vec![&ctx.charlie])?;

        let (schedule, _sess) =
            helpers::query_vesting_get_schedule(sess, &ctx.vesting, &ctx.charlie)?;
        assert!(schedule.is_none());

        Ok(())
    }

    #[test]
    fn vesting_admin_cannot_remove_recipient_once_active() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let charlie_schedule = helpers::Schedule {
            amount: 0,
            cliff: 0,
            offset: 0,
            duration: 0,
        };

        let sess = helpers::vesting_add_recipients(
            ctx.sess,
            &ctx.vesting,
            &ctx.bob,
            vec![&ctx.charlie],
            vec![&charlie_schedule],
        )?;

        let sess = helpers::vesting_activate(sess, &ctx.vesting, &ctx.bob)?;

        match helpers::vesting_remove_recipients(sess, &ctx.vesting, &ctx.bob, vec![&ctx.charlie]) {
            Ok(_) => panic!("Should panic because vesting has been activated"),
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn vesting_non_admin_cannot_remove_recipient() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let charlie_schedule = helpers::Schedule {
            amount: 0,
            cliff: 0,
            offset: 0,
            duration: 0,
        };

        let sess = helpers::vesting_add_recipients(
            ctx.sess,
            &ctx.vesting,
            &ctx.bob,
            vec![&ctx.charlie],
            vec![&charlie_schedule],
        )?;

        match helpers::vesting_remove_recipients(
            sess,
            &ctx.vesting,
            &ctx.ed, // not bob
            vec![&ctx.charlie],
        ) {
            Ok(_) => panic!("Should panic because caller is not admin"),
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn vesting_can_not_claim_before_active() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let cliff = 100e12 as u128;
        let offset = helpers::DAY;

        let charlie_schedule = helpers::Schedule {
            amount: 0,
            cliff,
            offset,
            duration: 0,
        };

        // Add funding
        let sess =
            helpers::gov_token_transfer(ctx.sess, &ctx.gov_token, &ctx.bob, &ctx.vesting, cliff)?;

        let sess = helpers::vesting_add_recipients(
            sess,
            &ctx.vesting,
            &ctx.bob,
            vec![&ctx.charlie],
            vec![&charlie_schedule],
        )?;

        let sess = helpers::update_in_milliseconds(sess, offset);

        match helpers::vesting_claim(sess, &ctx.vesting, &ctx.charlie) {
            Ok(_) => panic!("Should panic because vesting is not active"),
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn vesting_claim_cliff_only() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let cliff = 100e12 as u128;
        let offset = helpers::DAY;

        let charlie_schedule = helpers::Schedule {
            amount: 0,
            cliff,
            offset,
            duration: 0,
        };

        // Add funding
        let sess =
            helpers::gov_token_transfer(ctx.sess, &ctx.gov_token, &ctx.bob, &ctx.vesting, cliff)?;

        let sess = helpers::vesting_add_recipients(
            sess,
            &ctx.vesting,
            &ctx.bob,
            vec![&ctx.charlie],
            vec![&charlie_schedule],
        )?;

        let sess = helpers::vesting_activate(sess, &ctx.vesting, &ctx.bob)?;

        let sess = helpers::update_in_milliseconds(sess, offset);

        let (charlie_balance_before, sess) =
            helpers::query_token_balance(sess, &ctx.gov_token, &ctx.charlie)?;

        let sess = helpers::vesting_claim(sess, &ctx.vesting, &ctx.charlie)?;

        let (charlie_balance_after, _sess) =
            helpers::query_token_balance(sess, &ctx.gov_token, &ctx.charlie)?;

        assert_eq!(charlie_balance_after, charlie_balance_before + cliff);

        Ok(())
    }

    #[test]
    fn vesting_claim_amount_only_with_no_duration() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let amount = 100e12 as u128;
        let offset = helpers::DAY;

        let charlie_schedule = helpers::Schedule {
            amount,
            cliff: 0,
            offset,
            duration: 0,
        };

        // Add funding
        let sess =
            helpers::gov_token_transfer(ctx.sess, &ctx.gov_token, &ctx.bob, &ctx.vesting, amount)?;

        let sess = helpers::vesting_add_recipients(
            sess,
            &ctx.vesting,
            &ctx.bob,
            vec![&ctx.charlie],
            vec![&charlie_schedule],
        )?;

        let sess = helpers::vesting_activate(sess, &ctx.vesting, &ctx.bob)?;

        let sess = helpers::update_in_milliseconds(sess, offset);

        let (charlie_balance_before, sess) =
            helpers::query_token_balance(sess, &ctx.gov_token, &ctx.charlie)?;

        let sess = helpers::vesting_claim(sess, &ctx.vesting, &ctx.charlie)?;

        let (charlie_balance_after, _sess) =
            helpers::query_token_balance(sess, &ctx.gov_token, &ctx.charlie)?;

        assert_eq!(charlie_balance_after, charlie_balance_before + amount);

        Ok(())
    }

    #[test]
    fn vesting_claim_flow() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let amount = 100e12 as u128;
        let cliff = 50e12 as u128;
        let offset = helpers::DAY;
        let duration = helpers::DAY * 14;

        let charlie_schedule = helpers::Schedule {
            amount,
            cliff,
            offset,
            duration,
        };

        // Add funding
        let sess = helpers::gov_token_transfer(
            ctx.sess,
            &ctx.gov_token,
            &ctx.bob,
            &ctx.vesting,
            amount + cliff,
        )?;

        let sess = helpers::vesting_add_recipients(
            sess,
            &ctx.vesting,
            &ctx.bob,
            vec![&ctx.charlie],
            vec![&charlie_schedule],
        )?;

        let sess = helpers::vesting_activate(sess, &ctx.vesting, &ctx.bob)?;

        let (charlie_balance_initial, sess) =
            helpers::query_token_balance(sess, &ctx.gov_token, &ctx.charlie)?;

        // Claim cliff and half of amount
        let sess = helpers::update_in_milliseconds(sess, offset + duration / 2);
        let (charlie_balance_before, sess) =
            helpers::query_token_balance(sess, &ctx.gov_token, &ctx.charlie)?;
        let sess = helpers::vesting_claim(sess, &ctx.vesting, &ctx.charlie)?;
        let (charlie_balance_after, sess) =
            helpers::query_token_balance(sess, &ctx.gov_token, &ctx.charlie)?;
        assert_eq!(
            charlie_balance_after,
            charlie_balance_before + cliff + amount / 2
        );

        // Claim remaining half of amount
        let sess = helpers::update_in_milliseconds(sess, duration / 2);
        let (charlie_balance_before, sess) =
            helpers::query_token_balance(sess, &ctx.gov_token, &ctx.charlie)?;
        let sess = helpers::vesting_claim(sess, &ctx.vesting, &ctx.charlie)?;
        let (charlie_balance_after, _sess) =
            helpers::query_token_balance(sess, &ctx.gov_token, &ctx.charlie)?;
        assert_eq!(charlie_balance_after, charlie_balance_before + amount / 2);

        // Ensure 100% was claimed
        assert_eq!(
            charlie_balance_after,
            charlie_balance_initial + amount + cliff
        );

        Ok(())
    }

    #[test]
    fn unlock_nft_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup(ACC_THRESHOLD, REJECT_THRESHOLD, EXEC_THRESHOLD).unwrap();
        ctx = wrap_tokens(ctx, TOTAL_SUPPLY / 10).unwrap();

        let sess = call_function(
            ctx.sess,
            &ctx.gov_nft,
            &ctx.alice,
            String::from("is_collection_locked"),
            Some(vec![]),
            None,
            transcoder_governance_nft(),
        )
        .unwrap();

        let rr: Result<bool, drink::errors::LangError> = sess.last_call_return().unwrap();
        let transfer_status = rr.unwrap();

        assert_eq!(transfer_status, true);

        let sess = call_function(
            sess,
            &ctx.governance,
            &ctx.alice,
            String::from("create_proposal"),
            Some(vec![
                helpers::PropType::UnlockTransfer().to_string(),
                1.to_string(),
            ]),
            None,
            transcoder_governance(),
        )
        .unwrap();

        // let rr: Result<helpers::Proposal, drink::errors::LangError> = sess.last_call_return().unwrap();
        // let proposal = rr.unwrap();

        // When retrieving the proposal it returns None
        // let (proposal, sess) = helpers::query_governance_get_proposal_by_nft(sess, &ctx.governance, 1_u128).unwrap();
        // let proposal_string: String = proposal.prop_id.to_string();
        Ok(())
    }
}
