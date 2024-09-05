#[cfg(test)]
mod helpers;

#[cfg(test)]
mod sources;

#[cfg(test)]
mod tests {
    use crate::helpers;

    use drink::{
        chain_api::ChainApi,
        runtime::MinimalRuntime,
        session::Session,
        AccountId32,
    };
    use std::error::Error;

    struct TestContext {
        sess: Session<MinimalRuntime>,
        registry: AccountId32,
        share_token: AccountId32,
        vault: AccountId32,
        nominators: Vec<AccountId32>,
        validators: Vec<AccountId32>,
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

        let validator1 = AccountId32::new([101u8; 32]);
        let validator2 = AccountId32::new([102u8; 32]);
        let validator3 = AccountId32::new([103u8; 32]);

        let mut sess: Session<MinimalRuntime> = Session::<MinimalRuntime>::new().unwrap();

        // FUND DEFAULT ACCOUNTS
        sess.chain_api().add_tokens(alice.clone(), 100_000_000e12 as u128);
        sess.chain_api().add_tokens(bob.clone(), 100_000_000e12 as u128);
        sess.chain_api().add_tokens(charlie.clone(), 100_000_000e12 as u128);
        sess.chain_api().add_tokens(dave.clone(), 100_000_000e12 as u128);
        sess.chain_api().add_tokens(ed.clone(), 100_000_000e12 as u128);

        sess.upload(helpers::bytes_registry()).expect("Session should upload registry bytes");
        sess.upload(helpers::bytes_share_token()).expect("Session should upload token bytes");
        sess.upload(helpers::bytes_nominator()).expect("Session should upload nominator bytes");

        let vault = sess.deploy(
            helpers::bytes_vault(),
            "new",
            &[
                helpers::hash_share_token(),
                helpers::hash_registry(),
                helpers::hash_nominator(),
            ],
            vec![1],
            None,
            &helpers::transcoder_vault().unwrap(),
        )?;
        sess.set_transcoder(vault.clone(), &helpers::transcoder_vault().unwrap());

        let mut sess = helpers::call_function(
            sess,
            &vault,
            &bob,
            String::from("IVault::get_registry_contract"),
            None,
            None,
            helpers::transcoder_vault(),
        )
            .unwrap();
        let rr: Result<AccountId32, drink::errors::LangError> = sess.last_call_return().unwrap();
        let registry = rr.unwrap();
        sess.set_transcoder(registry.clone(), &helpers::transcoder_registry().unwrap());

        let mut sess = helpers::call_function(
            sess,
            &vault,
            &bob,
            String::from("IVault::get_share_token_contract"),
            None,
            None,
            helpers::transcoder_vault(),
        )
            .unwrap();
        let ss: Result<AccountId32, drink::errors::LangError> = sess.last_call_return().unwrap();
        let share_token = ss.unwrap();
        sess.set_transcoder(share_token.clone(), &helpers::transcoder_share_token().unwrap());

        sess.set_actor(bob.clone());

        // ADD AGENTS
        let (_new_agent, sess) = helpers::call_add_agent(
            sess,
            &registry,
            &bob,
            &bob,
            &validator1,
            100e12 as u128,
        )?;
        let (_new_agent, sess) = helpers::call_add_agent(
            sess,
            &registry,
            &bob,
            &bob,
            &validator2,
            100e12 as u128,
        )?;

        let (_, agents, sess) = helpers::get_agents(sess, &registry)?;

        let sess = helpers::call_update_agents(
            sess,
            &registry,
            &bob,
            vec![agents[0].address.to_string(), agents[1].address.to_string()],
            vec![String::from("100"), String::from("100")],
        )?;

        Ok(TestContext {
            sess,
            registry,
            share_token,
            vault,
            nominators: vec![agents[0].address.clone(), agents[1].address.clone()],
            validators: vec![validator1, validator2, validator3],
            alice,
            bob,
            charlie,
            dave,
            ed,
        })
    }

    #[test]
    fn test_fees_flow_multiple_stakes_success() -> Result<(), Box<dyn Error>> {
        let ctx: TestContext = setup().unwrap();
        const STAKE_AMOUNT: u128 = 10_000e10 as u128;
        const INTERVALS: u64 = 5;

        let (_, sess) = helpers::call_stake(ctx.sess, &ctx.vault, &ctx.share_token, &ctx.bob, STAKE_AMOUNT).unwrap();
        let sess = helpers::update_in_milliseconds(sess, helpers::YEAR / INTERVALS);
        let (fees, sess) = helpers::get_current_virtual_shares(sess, &ctx.vault).unwrap();
        assert_eq!(fees, 40e10 as u128);

        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.alice, STAKE_AMOUNT).unwrap();
        let sess = helpers::update_in_milliseconds(sess, helpers::YEAR / INTERVALS);
        let (fees, sess) = helpers::get_current_virtual_shares(sess, &ctx.vault).unwrap();
        assert_eq!(fees, 1203200000000u128);

        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.charlie, STAKE_AMOUNT).unwrap();
        let sess = helpers::update_in_milliseconds(sess, helpers::YEAR / INTERVALS);
        let (fees, sess) = helpers::get_current_virtual_shares(sess, &ctx.vault).unwrap();
        assert_eq!(fees, 2412819200000u128);

        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.dave, STAKE_AMOUNT).unwrap();
        let sess = helpers::update_in_milliseconds(sess, helpers::YEAR / INTERVALS);
        let (fees, sess) = helpers::get_current_virtual_shares(sess, &ctx.vault).unwrap();
        assert_eq!(fees, 4032096102400u128);

        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.ed, STAKE_AMOUNT).unwrap();
        let sess = helpers::update_in_milliseconds(sess, helpers::YEAR / INTERVALS);
        let (fees, sess) = helpers::get_current_virtual_shares(sess, &ctx.vault).unwrap();
        assert_eq!(fees, 6064288614912u128);

        let (total_pooled, sess) = helpers::get_total_pooled(sess, &ctx.vault).unwrap();
        assert_eq!(total_pooled, STAKE_AMOUNT * INTERVALS as u128);

        let (shares_before, sess) = helpers::query_token_balance(sess, &ctx.share_token, &ctx.bob).unwrap();
        let sess = helpers::call_withdraw_fees(sess, &ctx.vault, &ctx.bob).unwrap();
        let (shares_after, sess) = helpers::query_token_balance(sess, &ctx.share_token, &ctx.bob).unwrap();
        assert_eq!(shares_after - shares_before, fees);

        let (fees, _sess) = helpers::get_current_virtual_shares(sess, &ctx.vault).unwrap();
        assert_eq!(fees, 0);

        Ok(())
    }
    #[test]
    fn test_minimum_stake_panic_because_below_threshold() {
        let ctx: TestContext = setup().unwrap();
        let sess = ctx.sess;

        let minimum_stake = 1_000_000;

        match helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.alice, minimum_stake - 100) {
            Ok(_) => panic!("Should panic because stake is insufficient"),
            Err(_) => (),
        };
    }
    #[test]
    fn test_staking_redeem_flow() -> Result<(), Box<dyn Error>> {
        let ctx = setup().unwrap();

        // Verify nominators
        let (staked, unbonded, sess) = helpers::query_nominator_balance(ctx.sess, &ctx.nominators[0]).unwrap();
        assert_eq!(staked, 0, "Nominator #1 should have no staked AZERO");
        assert_eq!(unbonded, 0, "Nominator #1 should have no unbonded AZERO");
        let (staked, unbonded, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[1]).unwrap();
        assert_eq!(staked, 0, "Nominator #2 should have no staked AZERO");
        assert_eq!(unbonded, 0, "Nominator #2 should have no unbonded AZERO");

        // Staking of 5m AZERO
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.alice, 1_000_000e10 as u128).unwrap();
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.bob, 1_000_000e10 as u128).unwrap();
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.charlie, 1_000_000e10 as u128).unwrap();
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.dave, 1_000_000e10 as u128).unwrap();
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.ed, 1_000_000e10 as u128).unwrap();

        let (staked, unbonded, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[0]).unwrap();
        assert_eq!(
            staked, 2_500_000e10 as u128,
            "Nominator #1 should have half AZERO staked"
        );
        assert_eq!(unbonded, 0, "Nominator #1 should have no unbonded AZERO");
        let (_, _, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[1]).unwrap();
        assert_eq!(
            staked, 2_500_000e10 as u128,
            "Nominator #2 should have half AZERO staked"
        );
        assert_eq!(unbonded, 0, "Nominator #2 should have no unbonded AZERO");

        // Allow fees to accumulate
        let sess = helpers::update_days(sess, 2);

        // Unlock requests of 50k AZERO
        let (_, sess) = helpers::call_request_unlock(sess, &ctx.vault, &ctx.share_token, &ctx.alice, 10_000e10 as u128).unwrap();
        let (_, sess) = helpers::call_request_unlock(sess, &ctx.vault, &ctx.share_token, &ctx.bob, 10_000e10 as u128).unwrap();
        let (_, sess) = helpers::call_request_unlock(sess, &ctx.vault, &ctx.share_token, &ctx.charlie, 10_000e10 as u128).unwrap();
        let (_, sess) = helpers::call_request_unlock(sess, &ctx.vault, &ctx.share_token, &ctx.dave, 10_000e10 as u128).unwrap();
        let (_, sess) = helpers::call_request_unlock(sess, &ctx.vault, &ctx.share_token, &ctx.ed, 10_000e10 as u128).unwrap();

        let fees_50000_staked_2_days_shares = (50_000e10 as u128) * (2 * helpers::DAY as u128) / helpers::YEAR as u128 * 200 / helpers::BIPS;
        let (fees_50000_staked_2_days_azero, sess) = helpers::get_azero_from_shares(sess, &ctx.vault, fees_50000_staked_2_days_shares).unwrap();

        // Verify nominators
        let (staked, unbonded, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[0]).unwrap();
        assert_eq!(staked, (2_500_000e10 - 25_000e10) as u128 + (fees_50000_staked_2_days_azero / 2) + 2);
        assert_eq!(unbonded, 25_000e10 as u128 - (fees_50000_staked_2_days_azero / 2) - 2);
        let (staked, unbonded, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[1]).unwrap();
        assert_eq!(staked, (2_500_000e10 - 25_000e10) as u128 + (fees_50000_staked_2_days_azero / 2) + 2);
        assert_eq!(unbonded, 25_000e10 as u128 - (fees_50000_staked_2_days_azero / 2) - 2);

        // Wait for cooldown period to complete
        let sess = helpers::update_days(sess, 14);

        // Redeem AZERO minus fees
        let (redeemed, sess) = helpers::call_redeem_with_withdraw(sess, &ctx.vault, &ctx.alice, 0).unwrap();
        assert_eq!(redeemed, 10_000e10 as u128 + 22 - fees_50000_staked_2_days_azero / 5);
        let (redeemed, sess) = helpers::call_redeem(sess, &ctx.vault, &ctx.bob, 0).unwrap();
        assert_eq!(redeemed, 10_000e10 as u128 + 23 - fees_50000_staked_2_days_azero / 5);
        let (redeemed, sess) = helpers::call_redeem(sess, &ctx.vault, &ctx.charlie, 0).unwrap();
        assert_eq!(redeemed, 10_000e10 as u128 + 23 - fees_50000_staked_2_days_azero / 5);
        let (redeemed, sess) = helpers::call_redeem(sess, &ctx.vault, &ctx.dave, 0).unwrap();
        assert_eq!(redeemed, 10_000e10 as u128 + 23 - fees_50000_staked_2_days_azero / 5);
        let (redeemed, sess) = helpers::call_redeem(sess, &ctx.vault, &ctx.ed, 0).unwrap();
        assert_eq!(redeemed, 10_000e10 as u128 + 23 - fees_50000_staked_2_days_azero / 5);

        let (claimable_fees, sess) = helpers::get_current_virtual_shares(sess, &ctx.vault).unwrap();
        assert_eq!(claimable_fees, 43426511146997);

        let (shares_before, sess) = helpers::query_token_balance(sess, &ctx.share_token, &ctx.bob).unwrap();
        let sess = helpers::call_withdraw_fees(sess, &ctx.vault, &ctx.bob).unwrap();
        let (shares_after, _sess) = helpers::query_token_balance(sess, &ctx.share_token, &ctx.bob).unwrap();
        assert_eq!(shares_after - shares_before, claimable_fees);

        Ok(())
    }
    #[test]
    fn test_fee_adjustment_success() {
        let ctx = setup().unwrap();
        let sess = helpers::call_function(
            ctx.sess,
            &ctx.vault,
            &ctx.bob,
            String::from("IVault::adjust_fee"),
            Some(vec![String::from("1000")]),
            None,
            helpers::transcoder_vault(),
        )
        .unwrap();
        let sess = helpers::call_function(
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
        assert_eq!(res.unwrap(), 1000)
    }
    #[test]
    fn test_fee_adjustment_panic_because_caller_restricted() {
        let ctx = setup().unwrap();
        match helpers::call_function(
            ctx.sess,
            &ctx.vault,
            &ctx.ed, // not bob
            String::from("IVault::adjust_fee"),
            Some(vec![String::from("1234")]),
            None,
            helpers::transcoder_vault(),
        ) {
            Ok(_) => panic!("Should panic because caller does not have adjust fees role (Bob)"),
            Err(_) => (),
        }
    }
    #[test]
    fn test_fee_adjustment_panic_because_overflow() {
        let ctx = setup().unwrap();
        match helpers::call_function(
            ctx.sess,
            &ctx.vault,
            &ctx.alice,
            String::from("IVault::adjust_fee"),
            Some(vec![String::from("10000")]), // equal to BIPS
            None,
            helpers::transcoder_vault(),
        ) {
            Ok(_) => panic!("Should panic because new fee is too large"),
            Err(_) => (),
        };
    }
    #[test]
    fn test_withdraw_fees_after_one_second_success() {
        let ctx = setup().unwrap();

        const STAKE_AMOUNT: u128 = 10_000e10 as u128;

        // default annualized fee of 2%
        const EXPECTED_FEES: u128 = STAKE_AMOUNT * (helpers::SECOND as u128) / (helpers::YEAR as u128) * 200 / helpers::BIPS;

        // Stake 10k AZERO
        let (_, sess) = helpers::call_stake(ctx.sess, &ctx.vault, &ctx.share_token, &ctx.bob, STAKE_AMOUNT).unwrap();

        let sess = helpers::update_in_milliseconds(sess, helpers::SECOND);

        // Verify claimable fees
        let (shares_before, sess) = helpers::query_token_balance(sess, &ctx.share_token, &ctx.bob).unwrap();
        let (claimable_fees, sess) = helpers::get_current_virtual_shares(sess, &ctx.vault).unwrap();
        assert_eq!(claimable_fees, EXPECTED_FEES);

        // Withdraw fees
        let sess = helpers::call_withdraw_fees(sess, &ctx.vault, &ctx.bob).unwrap();

        // Verify shares
        let (shares_after, _sess) = helpers::query_token_balance(sess, &ctx.share_token, &ctx.bob).unwrap();
        assert_eq!(shares_after - shares_before, EXPECTED_FEES);
    }
    #[test]
    fn test_withdraw_fees_after_one_day_success() {
        let ctx = setup().unwrap();

        const STAKE_AMOUNT: u128 = 10_000e10 as u128;
        const EXPECTED_FEES: u128 = STAKE_AMOUNT * (helpers::DAY as u128) / (helpers::YEAR as u128) * 200 / helpers::BIPS;

        // Stake 10k AZERO
        let (_, sess) = helpers::call_stake(ctx.sess, &ctx.vault, &ctx.share_token, &ctx.bob, STAKE_AMOUNT).unwrap();

        let sess = helpers::update_days(sess, 1);

        // Verify claimable fees
        let (shares_before, sess) = helpers::query_token_balance(sess, &ctx.share_token, &ctx.bob).unwrap();
        let (claimable_fees, sess) = helpers::get_current_virtual_shares(sess, &ctx.vault).unwrap();
        assert_eq!(claimable_fees, EXPECTED_FEES);

        // Withdraw fees
        let sess = helpers::call_withdraw_fees(sess, &ctx.vault, &ctx.bob).unwrap();

        // Verify shares
        let (shares_after, _sess) = helpers::query_token_balance(sess, &ctx.share_token, &ctx.bob).unwrap();
        assert_eq!(shares_after - shares_before, EXPECTED_FEES); // annualized fee of 2%
    }
    #[test]
    fn test_withdraw_fees_after_one_year_success() {
        let ctx = setup().unwrap();

        const STAKE_AMOUNT: u128 = 10_000e10 as u128;
        const EXPECTED_FEES: u128 = STAKE_AMOUNT * 200 / helpers::BIPS;

        // Stake 10k AZERO
        let (_, sess) = helpers::call_stake(ctx.sess, &ctx.vault, &ctx.share_token, &ctx.bob, STAKE_AMOUNT).unwrap();

        // Verify claimable fees
        let (claimable_fees, sess) = helpers::get_current_virtual_shares(sess, &ctx.vault).unwrap();
        assert_eq!(claimable_fees, 0);

        let sess = helpers::update_in_milliseconds(sess, helpers::YEAR);

        // Verify claimable fees
        let (shares_before, sess) = helpers::query_token_balance(sess, &ctx.share_token, &ctx.bob).unwrap();
        let (claimable_fees, sess) = helpers::get_current_virtual_shares(sess, &ctx.vault).unwrap();
        assert_eq!(claimable_fees, EXPECTED_FEES);

        // Withdraw fees
        let sess = helpers::call_withdraw_fees(sess, &ctx.vault, &ctx.bob).unwrap();

        // Verify shares
        let (shares_after, _sess) = helpers::query_token_balance(sess, &ctx.share_token, &ctx.bob).unwrap();
        assert_eq!(shares_after - shares_before, EXPECTED_FEES);
    }
    #[test]
    fn test_withdraw_fees_after_adjusted_fee() {
        const STAKE: u128 = 10_000e10 as u128;
        const ONE_DAY_FEE_2_PERCENT: u128 = STAKE * helpers::DAY as u128 / helpers::YEAR as u128 * 2_00 / helpers::BIPS;
        const ONE_DAY_FEE_4_PERCENT: u128 = STAKE * helpers::DAY as u128 / helpers::YEAR as u128 * 4_00 / helpers::BIPS;

        let ctx = setup().unwrap();

        // Initial stake
        let (_, sess) = helpers::call_stake(ctx.sess, &ctx.vault, &ctx.share_token, &ctx.bob, STAKE).unwrap();

        // 2% fee for 1 day
        let sess = helpers::update_days(sess, 1);
        let (expected_fees, sess) = helpers::get_current_virtual_shares(sess, &ctx.vault).unwrap();
        assert_eq!(
            expected_fees, ONE_DAY_FEE_2_PERCENT,
            "Should have 2% fee for 1 day"
        );

        // Adjust fee to 4%
        let sess = helpers::call_function(
            sess,
            &ctx.vault,
            &ctx.bob,
            String::from("IVault::adjust_fee"),
            Some(vec![String::from("400")]), // 4% in helpers::BIPS
            None,
            helpers::transcoder_vault(),
        )
        .unwrap();

        // 4% fee for 1 day
        let sess = helpers::update_days(sess, 1);
        let (expected_fees, sess) = helpers::get_current_virtual_shares(sess, &ctx.vault).unwrap();
        assert_eq!(
            expected_fees,
            ONE_DAY_FEE_2_PERCENT + ONE_DAY_FEE_4_PERCENT + 599666, // compounding
            "Should show 2% fee for 1 day and 4% fee for 1 day",
        );

        let (shares_before, sess) = helpers::query_token_balance(sess, &ctx.share_token, &ctx.bob).unwrap();
        let sess = helpers::call_withdraw_fees(sess, &ctx.vault, &ctx.bob).unwrap();
        let (shares_after, _sess) = helpers::query_token_balance(sess, &ctx.share_token, &ctx.bob).unwrap();
        assert_eq!(
            shares_after - shares_before,
            ONE_DAY_FEE_2_PERCENT + ONE_DAY_FEE_4_PERCENT + 599666, // compounding,
            "Should withdraw 2% fee for 1 day and 4% fee for 1 day"
        );
    }
    #[test]
    fn test_withdraw_fees_panic_because_caller_restricted() {
        let ctx = setup().unwrap();
        let (_, sess) = helpers::call_stake(ctx.sess, &ctx.vault, &ctx.share_token, &ctx.bob, 1_000_000).unwrap();
        let sess = helpers::update_days(sess, 365);
        match helpers::call_withdraw_fees(
            sess,
            &ctx.vault,
            &ctx.ed, // not bob
        ) {
            Ok(_) => panic!("Should panic because caller does not have the fee to role (Bob)"),
            Err(_) => (),
        };
    }
    #[test]
    fn test_vault_transfer_role_adjust_fee_panic_because_caller_restricted() {
        let ctx = setup().unwrap();
        match helpers::call_function(
            ctx.sess,
            &ctx.vault,
            &ctx.alice, // not bob
            String::from("IVault::transfer_role_adjust_fee"),
            Some([ctx.charlie.to_string()].to_vec()),
            None,
            helpers::transcoder_vault(),
        ) {
            Ok(_) => panic!("Should panic because caller is restricted"),
            Err(_) => (),
        };
    }
    #[test]
    fn test_vault_transfer_role_adjust_fee_flow() {
        let ctx = setup().unwrap();

        let (adjust_fee, sess) = helpers::get_role_adjust_fee(ctx.sess, &ctx.vault).unwrap();
        assert_eq!(adjust_fee, ctx.bob);

        // Transfer role to Charlie
        let sess = helpers::call_function(
            sess,
            &ctx.vault,
            &adjust_fee,
            String::from("IVault::transfer_role_adjust_fee"),
            Some([ctx.charlie.to_string()].to_vec()),
            None,
            helpers::transcoder_vault(),
        )
            .unwrap();

        let (adjust_fee, _sess) = helpers::get_role_adjust_fee(sess, &ctx.vault).unwrap();
        assert_eq!(adjust_fee, ctx.charlie);
    }
    #[test]
    fn test_vault_transfer_role_fee_to_panic_because_caller_restricted() {
        let ctx = setup().unwrap();
        match helpers::call_function(
            ctx.sess,
            &ctx.vault,
            &ctx.alice, // not bob
            String::from("IVault::transfer_role_fee_to"),
            Some([ctx.charlie.to_string()].to_vec()),
            None,
            helpers::transcoder_vault(),
        ) {
            Ok(_) => panic!("Should panic because caller is restricted"),
            Err(_) => (),
        };
    }
    #[test]
    fn test_vault_transfer_role_fee_to_flow() {
        let ctx = setup().unwrap();

        let (fee_to, sess) = helpers::get_role_fee_to(ctx.sess, &ctx.vault).unwrap();
        assert_eq!(fee_to, ctx.bob);

        // Transfer role to Charlie
        let sess = helpers::call_function(
            sess,
            &ctx.vault,
            &fee_to,
            String::from("IVault::transfer_role_fee_to"),
            Some([ctx.charlie.to_string()].to_vec()),
            None,
            helpers::transcoder_vault(),
        )
            .unwrap();

        let (fee_to, _sess) = helpers::get_role_fee_to(sess, &ctx.vault).unwrap();
        assert_eq!(fee_to, ctx.charlie);
    }
    #[test]
    fn test_nominator_add_agent_role_flow() {
        let ctx = setup().unwrap();

        // Check roles
        let (role, sess) = helpers::get_role(ctx.sess, &ctx.registry, &helpers::RoleType::AddAgent).unwrap();
        assert_eq!(role, ctx.bob);
        let (admin, sess) = helpers::get_role_admin(sess, &ctx.registry, &helpers::RoleType::AddAgent).unwrap();
        assert_eq!(admin, ctx.bob);

        // Bob (admin) transfers role to Charlie
        let sess = helpers::transfer_role(sess, &ctx.registry, &admin, &helpers::RoleType::AddAgent, &ctx.charlie).unwrap();
        // Bob (admin) transfers admin to Charlie
        let sess = helpers::transfer_role_admin(sess, &ctx.registry, &admin, &helpers::RoleType::AddAgent, &ctx.charlie).unwrap();

        // Check roles
        let (role, sess) = helpers::get_role(sess, &ctx.registry, &helpers::RoleType::AddAgent).unwrap();
        assert_eq!(role, ctx.charlie);
        let (admin, _sess) = helpers::get_role_admin(sess, &ctx.registry, &helpers::RoleType::AddAgent).unwrap();
        assert_eq!(admin, ctx.charlie);
    }
    #[test]
    fn test_nominator_add_agent_role_panic_on_transfer_role_because_caller_not_admin() {
        let ctx = setup().unwrap();

        // Charlie (not admin) cannot transfer role
        match helpers::transfer_role(ctx.sess, &ctx.registry, &ctx.charlie, &helpers::RoleType::AddAgent, &ctx.dave) {
            Ok(_) => panic!("Should panic because caller is restricted"),
            Err(_) => (),
        };
    }
    #[test]
    fn test_nominator_add_agent_role_panic_on_transfer_admin_because_caller_not_admin() {
        let ctx = setup().unwrap();

        // Charlie (not admin) cannot transfer admin
        match helpers::transfer_role_admin(ctx.sess, &ctx.registry, &ctx.charlie, &helpers::RoleType::AddAgent, &ctx.dave) {
            Ok(_) => panic!("Should panic because caller is restricted"),
            Err(_) => (),
        };
    }
    #[test]
    fn test_nominator_update_agents_role_flow() {
        let ctx = setup().unwrap();

        // Check roles
        let (role, sess) = helpers::get_role(ctx.sess, &ctx.registry, &helpers::RoleType::UpdateAgents).unwrap();
        assert_eq!(role, ctx.bob);
        let (admin, sess) = helpers::get_role_admin(sess, &ctx.registry, &helpers::RoleType::UpdateAgents).unwrap();
        assert_eq!(admin, ctx.bob);

        // Bob (admin) transfers role to Charlie
        let sess = helpers::transfer_role(sess, &ctx.registry, &admin, &helpers::RoleType::UpdateAgents, &ctx.charlie).unwrap();
        // Bob (admin) transfers admin to Charlie
        let sess = helpers::transfer_role_admin(sess, &ctx.registry, &admin, &helpers::RoleType::UpdateAgents, &ctx.charlie).unwrap();

        // Check roles
        let (role, sess) = helpers::get_role(sess, &ctx.registry, &helpers::RoleType::UpdateAgents).unwrap();
        assert_eq!(role, ctx.charlie);
        let (admin, _sess) = helpers::get_role_admin(sess, &ctx.registry, &helpers::RoleType::UpdateAgents).unwrap();
        assert_eq!(admin, ctx.charlie);
    }
    #[test]
    fn test_nominator_update_agents_role_panic_on_transfer_role_because_caller_not_admin() {
        let ctx = setup().unwrap();

        // Charlie (not admin) cannot transfer role
        match helpers::transfer_role(ctx.sess, &ctx.registry, &ctx.charlie, &helpers::RoleType::UpdateAgents, &ctx.dave) {
            Ok(_) => panic!("Should panic because caller is restricted"),
            Err(_) => (),
        };
    }
    #[test]
    fn test_nominator_update_agents_role_panic_on_transfer_admin_because_caller_not_admin() {
        let ctx = setup().unwrap();

        // Charlie (not admin) cannot transfer admin
        match helpers::transfer_role_admin(ctx.sess, &ctx.registry, &ctx.charlie, &helpers::RoleType::UpdateAgents, &ctx.dave) {
            Ok(_) => panic!("Should panic because caller is restricted"),
            Err(_) => (),
        };
    }
    #[test]
    fn test_nominator_remove_agent_role_flow() {
        let ctx = setup().unwrap();

        // Check roles
        let (role, sess) = helpers::get_role(ctx.sess, &ctx.registry, &helpers::RoleType::RemoveAgent).unwrap();
        assert_eq!(role, ctx.bob);
        let (admin, sess) = helpers::get_role_admin(sess, &ctx.registry, &helpers::RoleType::RemoveAgent).unwrap();
        assert_eq!(admin, ctx.bob);

        // Bob (admin) transfers role to Charlie
        let sess = helpers::transfer_role(sess, &ctx.registry, &admin, &helpers::RoleType::RemoveAgent, &ctx.charlie).unwrap();
        // Bob (admin) transfers admin to Charlie
        let sess = helpers::transfer_role_admin(sess, &ctx.registry, &admin, &helpers::RoleType::RemoveAgent, &ctx.charlie).unwrap();

        // Check roles
        let (role, sess) = helpers::get_role(sess, &ctx.registry, &helpers::RoleType::RemoveAgent).unwrap();
        assert_eq!(role, ctx.charlie);
        let (admin, _sess) = helpers::get_role_admin(sess, &ctx.registry, &helpers::RoleType::RemoveAgent).unwrap();
        assert_eq!(admin, ctx.charlie);
    }
    #[test]
    fn test_nominator_remove_agent_role_panic_on_transfer_role_because_caller_not_admin() {
        let ctx = setup().unwrap();

        // Charlie (not admin) cannot transfer role
        match helpers::transfer_role(ctx.sess, &ctx.registry, &ctx.charlie, &helpers::RoleType::AddAgent, &ctx.dave) {
            Ok(_) => panic!("Should panic because caller is restricted"),
            Err(_) => (),
        };
    }
    #[test]
    fn test_nominator_remove_agent_role_panic_on_transfer_admin_because_caller_not_admin() {
        let ctx = setup().unwrap();

        // Charlie (not admin) cannot transfer admin
        match helpers::transfer_role_admin(ctx.sess, &ctx.registry, &ctx.charlie, &helpers::RoleType::AddAgent, &ctx.dave) {
            Ok(_) => panic!("Should panic because caller is restricted"),
            Err(_) => (),
        };
    }
    #[test]
    fn test_nominator_addition_panic_because_caller_restricted() {
        let ctx = setup().unwrap();

        match helpers::call_add_agent(
            ctx.sess,
            &ctx.registry,
            &ctx.charlie, // does not have `helpers::RoleType::AddAgent`
            &ctx.charlie,
            &ctx.validators[2],
            100e12 as u128,
        ) {
            Ok(_) => panic!("Should panic because caller is restricted"),
            Err(_) => (),
        };
    }
    #[test]
    fn test_nominator_update_panic_because_caller_restricted() {
        let ctx = setup().unwrap();

        match helpers::call_update_agents(
            ctx.sess,
            &ctx.registry,
            &ctx.charlie, // does not have `helpers::RoleType::UpdateAgents`
            vec![ctx.nominators[0].to_string()],
            vec![0.to_string()],
        ) {
            Ok(_) => panic!("Should panic because caller is restricted"),
            Err(_) => (),
        };
    }
    #[test]
    fn test_nominator_remove_panic_because_stake_is_non_zero() {
        let ctx = setup().unwrap();

        // Stake 1k AZERO
        let (_, sess) = helpers::call_stake(ctx.sess, &ctx.vault, &ctx.share_token, &ctx.alice, 1_000e12 as u128).unwrap();

        match helpers::call_remove_agent(
            sess,
            &ctx.registry,
            &ctx.bob, // has `helpers::RoleType::RemoveAgent`
            &ctx.nominators[0],
        ) {
            Ok(_) => panic!("Should panic because nominators[0] has stake"),
            Err(_) => (),
        };
    }
    #[test]
    fn test_nominator_remove_panic_because_caller_restricted() {
        let ctx = setup().unwrap();

        match helpers::call_remove_agent(
            ctx.sess,
            &ctx.registry,
            &ctx.charlie, // does not have `helpers::RoleType::RemoveAgent`
            &ctx.nominators[0],
        ) {
            Ok(_) => panic!("Should panic because caller is restricted"),
            Err(_) => (),
        };
    }
    #[test]
    fn test_nominator_remove_success() {
        let ctx = setup().unwrap();

        let (total_weight_before, agents_before, sess) = helpers::get_agents(
            ctx.sess,
            &ctx.registry,
        )
            .unwrap();

        // Remove agent
        let sess = helpers::call_remove_agent(
            sess,
            &ctx.registry,
            &ctx.bob, // has `helpers::RoleType::RemoveAgent`
            &ctx.nominators[0],
        )
            .unwrap();

        let (total_weight_after, agents_after, _sess) = helpers::get_agents(
            sess,
            &ctx.registry,
        )
            .unwrap();

        assert_eq!(agents_after.len(), agents_before.len() - 1);
        assert_eq!(total_weight_after, total_weight_before - agents_before[1].weight);
        assert_eq!(agents_after[0].address, agents_before[1].address);
        assert_eq!(agents_after[0].weight, agents_before[1].weight);
    }
    #[test]
    fn test_nominator_addition_equal_weights() -> Result<(), Box<dyn Error>> {
        let ctx = setup().unwrap();

        // Stake 10 million AZERO
        let (_, sess) = helpers::call_stake(ctx.sess, &ctx.vault, &ctx.share_token, &ctx.bob, 10_000_000).unwrap();

        let (stake1, _unbond, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[0]).unwrap();
        let (stake2, _unbond, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[1]).unwrap();
        assert_eq!(stake1, 5_000_000); // 50% to agent0
        assert_eq!(stake2, 5_000_000); // 50% to agent1

        let (total_weight_before, agents_before, sess) = helpers::get_agents(
            sess,
            &ctx.registry,
        )
            .unwrap();

        // Add nomination agent
        let (_new_agent, sess) = helpers::call_add_agent(
            sess,
            &ctx.registry,
            &ctx.bob,
            &ctx.bob,
            &ctx.validators[2],
            100e12 as u128,
        )?;

        let (total_weight_after, agents_after, sess) = helpers::get_agents(
            sess,
            &ctx.registry,
        )
            .unwrap();

        assert_eq!(agents_after.len(), agents_before.len() + 1);
        assert_eq!(total_weight_after, total_weight_before);
        assert_eq!(agents_after[2].weight, 0);

        // Update weight to 100
        let sess = helpers::call_update_agents(
            sess,
            &ctx.registry,
            &ctx.bob,
            vec![agents_after[2].address.to_string()],
            vec![100.to_string()],
        )
            .unwrap();

        let (total_weight_after, agents_after, sess) = helpers::get_agents(
            sess,
            &ctx.registry,
        )
            .unwrap();

        assert_eq!(agents_after.len(), agents_before.len() + 1);
        assert_eq!(total_weight_after, total_weight_before + 100);
        assert_eq!(agents_after[2].weight, 100);

        // Stake additional 10 million AZERO
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.bob, 10000000).unwrap();

        let (stake1, _, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[0]).unwrap();
        let (stake2, _, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[1]).unwrap();
        let (stake3, _, _sess) = helpers::query_nominator_balance(sess, &agents_after[2].address).unwrap();
        assert_eq!(stake1, 6666668);
        assert_eq!(stake2, 6666666);
        assert_eq!(stake3, 6666666);

        Ok(())
    }
    #[test]
    fn test_nominator_addition_unequal_weights() -> Result<(), Box<dyn Error>> {
        let ctx = setup().unwrap();

        // Stake 10m AZERO
        let (_, sess) = helpers::call_stake(ctx.sess, &ctx.vault, &ctx.share_token, &ctx.bob, 10_000_000).unwrap();

        let (stake1, _, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[0]).unwrap();
        let (stake2, _, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[1]).unwrap();
        assert_eq!(stake1, 5_000_000); // 50% to agent0
        assert_eq!(stake2, 5_000_000); // 50% to agent1

        let (total_weight_before, agents_before, sess) = helpers::get_agents(
            sess,
            &ctx.registry,
        )
            .unwrap();

        // Add nomination agent
        let (_new_agent, sess) = helpers::call_add_agent(
            sess,
            &ctx.registry,
            &ctx.bob,
            &ctx.bob,
            &ctx.validators[2],
            100e12 as u128,
        )?;

        let (total_weight_after, agents_after, sess) = helpers::get_agents(
            sess,
            &ctx.registry,
        )
            .unwrap();

        assert_eq!(agents_after.len(), agents_before.len() + 1);
        assert_eq!(total_weight_after, total_weight_before);
        assert_eq!(agents_after[2].weight, 0);

        // Update weight to 50
        let sess = helpers::call_update_agents(
            sess,
            &ctx.registry,
            &ctx.bob,
            vec![agents_after[2].address.to_string()],
            vec![50.to_string()],
        )
            .unwrap();

        let (total_weight_after, agents_after, sess) = helpers::get_agents(
            sess,
            &ctx.registry,
        )
            .unwrap();

        assert_eq!(total_weight_after, total_weight_before + 50);
        assert_eq!(agents_after[2].weight, 50);

        // Stake another 10m AZERO
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.bob, 10_000_000).unwrap();

        let (stake1, _, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[0]).unwrap();
        let (stake2, _, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[1]).unwrap();
        let (stake3, _, _sess) = helpers::query_nominator_balance(sess, &agents_after[2].address).unwrap();
        assert_eq!(stake1, 5_000_000 + 3_000_000);
        assert_eq!(stake2, 5_000_000 + 3_000_000);
        assert_eq!(stake3, 4_000_000);

        Ok(())
    }

    #[test]
    fn test_unlock_weight_change() -> Result<(), Box<dyn Error>> {
        let ctx = setup().unwrap();
        let sess = ctx.sess;

        // Stake 5 million AZERO
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.alice, 1000000).unwrap();
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.bob, 1000000).unwrap();
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.charlie, 1000000).unwrap();
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.dave, 1000000).unwrap();
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.ed, 1000000).unwrap();

        let sess = helpers::update_days(sess, 2);

        // Update agent #1 weight from 100/200 to 50/150
        let sess = helpers::call_update_agents(
            sess,
            &ctx.registry,
            &ctx.bob,
            vec![ctx.nominators[0].to_string()],
            vec![50.to_string()],
        )
            .unwrap();

        // Request unlocks of all 5 million sA0
        let (_, sess) = helpers::call_request_unlock(sess, &ctx.vault, &ctx.share_token, &ctx.alice, 1000000).unwrap();
        let (_, sess) = helpers::call_request_unlock(sess, &ctx.vault, &ctx.share_token, &ctx.bob, 1000000).unwrap();
        let (_, sess) = helpers::call_request_unlock(sess, &ctx.vault, &ctx.share_token, &ctx.charlie, 1000000).unwrap();
        let (_, sess) = helpers::call_request_unlock(sess, &ctx.vault, &ctx.share_token, &ctx.dave, 1000000).unwrap();
        let (_, sess) = helpers::call_request_unlock(sess, &ctx.vault, &ctx.share_token, &ctx.ed, 1000000).unwrap();

        let sess = helpers::update_days(sess, 14);

        // Fees accumulated for 5m AZERO staked for 2 days
        let expected_fees = 5_000_000 * 2 / 365 * 200 / helpers::BIPS;
        let (claimable_fees, sess) = helpers::get_current_virtual_shares(sess, &ctx.vault).unwrap();
        assert_eq!(claimable_fees, expected_fees);

        let (stake1, _, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[0]).unwrap();
        let (stake2, _, _sess) = helpers::query_nominator_balance(sess, &ctx.nominators[1]).unwrap();
        assert_eq!(stake1, expected_fees * 50 / 150); // agent 0 fees
        assert_eq!(stake2, 1 + expected_fees * 100 / 150); // dust and agent 1 fees

        Ok(())
    }
    #[test]
    fn test_withdraw_all() -> Result<(), Box<dyn Error>> {
        let ctx = setup().unwrap();
        let sess = ctx.sess;

        // Stake 5 million AZERO
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.alice, 1000000).unwrap();
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.bob, 1000000).unwrap();
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.charlie, 1000000).unwrap();
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.dave, 1000000).unwrap();
        let (_, sess) = helpers::call_stake(sess, &ctx.vault, &ctx.share_token, &ctx.ed, 1000000).unwrap();

        // Allow fees to accumulate
        let sess = helpers::update_days(sess, 2);

        // Update agent weight from 100/200 to 50/150
        let sess = helpers::call_update_agents(
            sess,
            &ctx.registry,
            &ctx.bob,
            vec![ctx.nominators[0].to_string()],
            vec![50.to_string()],
        )
            .unwrap();

        // Request unlocking of 5 million AZERO
        let (_, sess) = helpers::call_request_unlock(sess, &ctx.vault, &ctx.share_token, &ctx.alice, 1000000).unwrap();
        let (_, sess) = helpers::call_request_unlock(sess, &ctx.vault, &ctx.share_token, &ctx.bob, 1000000).unwrap();
        let (_, sess) = helpers::call_request_unlock(sess, &ctx.vault, &ctx.share_token, &ctx.charlie, 1000000).unwrap();
        let (_, sess) = helpers::call_request_unlock(sess, &ctx.vault, &ctx.share_token, &ctx.dave, 1000000).unwrap();
        let (_, sess) = helpers::call_request_unlock(sess, &ctx.vault, &ctx.share_token, &ctx.ed, 1000000).unwrap();

        // Wait for cooldown period
        let sess = helpers::update_days(sess, 14);

        // Fees accumulated for 5m AZERO staked for 2 days
        let expected_fees = 5_000_000 * 2 / 365 * 200 / helpers::BIPS;
        let (claimable_fees, sess) = helpers::get_current_virtual_shares(sess, &ctx.vault).unwrap();
        assert_eq!(claimable_fees, expected_fees);

        // Verify all AZERO is withdrawn except fees
        let (stake, _unbond, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[0]).unwrap();
        assert_eq!(stake, expected_fees * 50 / 150); // agent 0 weight
        let (stake, _unbond, sess) = helpers::query_nominator_balance(sess, &ctx.nominators[1]).unwrap();
        assert_eq!(stake, 1 + expected_fees * 100 / 150); // dust and agent 1 weight

        let fee_split = expected_fees / 5;
        let (redeemed, sess) = helpers::call_redeem_with_withdraw(sess, &ctx.vault, &ctx.alice, 0).unwrap();
        assert_eq!(redeemed, 1000000 + 32 - fee_split - 10);
        let (redeemed, sess) = helpers::call_redeem(sess, &ctx.vault, &ctx.bob, 0).unwrap();
        assert_eq!(redeemed, 1000000 + 32 - fee_split - 9);
        let (redeemed, sess) = helpers::call_redeem(sess, &ctx.vault, &ctx.charlie, 0).unwrap();
        assert_eq!(redeemed, 1000000 + 32 - fee_split - 8);
        let (redeemed, sess) = helpers::call_redeem(sess, &ctx.vault, &ctx.dave, 0).unwrap();
        assert_eq!(redeemed, 1000000 + 32 - fee_split - 8);
        let (redeemed, mut sess) = helpers::call_redeem(sess, &ctx.vault, &ctx.ed, 0).unwrap();
        assert_eq!(redeemed, 1000000 + 32 - fee_split - 8);

        let vault_balance = sess.chain_api().balance(&ctx.vault);
        assert_eq!(vault_balance, 1, "Vault should only have dust remaining");

        Ok(())
    }
    #[test]
    fn test_token_transfer_from_panics_properly() {
        let ctx = setup().unwrap();

        // Bob stakes 1m AZERO
        let (_, sess) = helpers::call_stake(ctx.sess, &ctx.vault, &ctx.share_token, &ctx.bob, 1_000_000).unwrap();

        // Ed attempts to transfer 1k of Bob's sA0
        match helpers::call_function(
            sess,
            &ctx.share_token,
            &ctx.ed, // not bob
            String::from("PSP22::transfer_from"),
            Some(vec![ctx.bob.to_string(), ctx.ed.to_string(), 1000.to_string(), "[]".to_string()]),
            None,
            helpers::transcoder_share_token(),
        )  {
            Ok(_) => panic!("Should panic because Bob has not approved Ed to transfer sA0"),
            Err(res) => println!("{:?}", res.to_string()),
        };
    }
    #[test]
    fn test_token_transfer_from_works_normally() {
        let ctx = setup().unwrap();

        // Bob stakes 1m AZERO
        let (_, sess) = helpers::call_stake(ctx.sess, &ctx.vault, &ctx.share_token, &ctx.bob, 1_000_000).unwrap();

        // Bob approves Ed to transfer 1k sA0
        let sess = helpers::call_function(
            sess,
            &ctx.share_token,
            &ctx.bob,
            String::from("PSP22::approve"),
            Some(vec![ctx.ed.to_string(), 1000.to_string()]),
            None,
            helpers::transcoder_share_token(),
        ).unwrap();

        // Ed transfers 1k of Bob's sA0
        helpers::call_function(
            sess,
            &ctx.share_token,
            &ctx.ed, // previously approved
            String::from("PSP22::transfer_from"),
            Some(vec![ctx.bob.to_string(),ctx.ed.to_string(),1000.to_string(),"[]".to_string()]),
            None,
            helpers::transcoder_share_token(),
        ).unwrap();
    }
    #[test]
    fn test_compound_call() -> Result<(), Box<dyn Error>> {
        let ctx = setup().unwrap();
        let mut sess = ctx.sess;

        // Fund nominator agents to simulate AZERO being claimed
        let mock_reward = 10_000;
        sess.chain_api().add_tokens(ctx.nominators[0].clone(), mock_reward);
        sess.chain_api().add_tokens(ctx.nominators[1].clone(), mock_reward);

        // Compound
        let sess = helpers::call_function(
            sess,
            &ctx.vault,
            &ctx.bob,
            String::from("IVault::compound"),
            None,
            None,
            helpers::transcoder_vault(),
        )
            .unwrap();

        let (total_pooled, _sess) = helpers::get_total_pooled(sess, &ctx.vault).unwrap();
        assert_eq!(total_pooled, mock_reward * 2);

        Ok(())
    }
}
