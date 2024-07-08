#[cfg(test)]
mod sources;

#[cfg(test)]
mod tests {
    use drink::{
        chain_api::ChainApi,
        runtime::MinimalRuntime,
        session::Session,
        AccountId32,
    };
   
    use drink::session::NO_ARGS;
    use drink::session::contract_transcode::ContractMessageTranscoder;
    use std::error::Error;
    use crate::sources::*;
    use std::rc::Rc;
    use governance::PropType;
    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    pub struct GovernanceData {
        pub block_created:u64,
        pub vote_weight:u128
     }
    const TOTAL_SUPPLY:u128=100_000_000_000_000_000_u128;
    let ACC_THRESHOLD=TOTAL_SUPPLY/5;
    let REJECT_THRESHOLD=TOTAL_SUPPLY/5;
    let EXEC_THRESHOLD=TOTAL_SUPPLY/5;
    struct TestContext {
        sess: Session<MinimalRuntime>,
        gov_token:AccountId32,
        gov_nft:AccountId32,
        stake_contract:AccountId32,
        governance:AccountId32,
        vault:AccountId32,
        alice: AccountId32,
        bob: AccountId32,
        charlie: AccountId32,
        dave: AccountId32,
        ed: AccountId32,
    }
    pub fn call_function(
        mut sess: Session<MinimalRuntime>,
        contract: &AccountId32,
        sender: &AccountId32,
        func_name: String,
        args: Option<Vec<String>>,
        value: Option<u128>,
        transcoder: Option<Rc<ContractMessageTranscoder>>,
    ) -> Result<Session<MinimalRuntime>, Box<dyn Error>> {
        println!("Calling: {}()", func_name);
        if let Some(args) = args {
            sess.set_actor(sender.clone());
            sess.set_transcoder(contract.clone(), &transcoder.unwrap());
            sess.call_with_address(contract.clone(), &func_name, &args, value)?;
        } else {
            sess.set_actor(sender.clone());
            sess.set_transcoder(contract.clone(), &transcoder.unwrap());
            sess.call_with_address(contract.clone(), &func_name, NO_ARGS, value)?;
        }
    
        // Print debug logs
        let encoded = &sess.last_call_result().unwrap().debug_message;
        let decoded = encoded.iter().map(|b| *b as char).collect::<String>();
        let messages: Vec<String> = decoded.split('\n').map(|s| s.to_string()).collect();
        for line in messages {
            if line.len() > 0 {
                println!("LOG: {}", line);
            }
        }
    
        Ok(sess)
    }
    
    fn setup() -> Result<TestContext, Box<dyn Error>> {
        let bob = AccountId32::new([1u8; 32]);
        let alice = AccountId32::new([2u8; 32]);
        let charlie = AccountId32::new([3u8; 32]);
        let dave = AccountId32::new([4u8; 32]);
        let ed = AccountId32::new([5u8; 32]);

        let mut sess: Session<MinimalRuntime> = Session::<MinimalRuntime>::new().unwrap();
        let gov_token = sess.deploy::<String>(
            bytes_governance_token(),
            "new",
            &[],
            vec![2],
            None,
            &transcoder_governance_token().unwrap(),
        )?;

        sess.upload(bytes_governance_nft()).expect("Session should upload registry bytes");

        sess.chain_api().add_tokens(alice.clone(), 100_000_000e10 as u128);
        sess.chain_api().add_tokens(bob.clone(), 100_000_000e10 as u128);
        sess.chain_api().add_tokens(charlie.clone(), 100_000_000e10 as u128);
        sess.chain_api().add_tokens(dave.clone(), 100_000_000e10 as u128);
        sess.chain_api().add_tokens(ed.clone(), 100_000_000e10 as u128);

     

        let stake_contract=sess.deploy(
            bytes_governance_staking(),
            "new",
            &[  gov_token.to_string(),
                alice.to_string(),
                hash_governance_nft(),
                100_000.to_string(),
                
            ],
            vec![2],
            None,
            &transcoder_governance_staking().unwrap(),
        )?;
        sess.set_transcoder(stake_contract.clone(),&transcoder_governance_staking().unwrap());

        let mut sess = call_function(
            sess,
            &stake_contract,
            &bob,
            String::from("get_governance_nft"),
            None,
            None,
            transcoder_governance_staking(),
        ).unwrap();
        let rr: Result<AccountId32, drink::errors::LangError> = sess.last_call_return().unwrap();
        let gov_nft = rr.unwrap();


        println!("{:?}",stake_contract.to_string());
        println!("{:?}",gov_token.to_string());
        println!("{:?}",gov_nft.to_string());
        let user_tokens=TOTAL_SUPPLY/5;
        let sess=call_function(
            sess,
            &gov_token,
            &bob,
            String::from("PSP22::transfer_from"),
            Some(vec![ bob.to_string(),alice.to_string(), user_tokens.to_string(), "[]".to_string()]),
            None,
            transcoder_governance_token(),
        )?;
        let sess=call_function(
            sess,
            &gov_token,
            &bob, 
            String::from("PSP22::transfer_from"),
            Some(vec![bob.to_string(), charlie.to_string(), user_tokens.to_string(), "[]".to_string()]),
            None,
            transcoder_governance_token(),
        )?;
        let sess=call_function(
            sess,
            &gov_token,
            &bob, 
            String::from("PSP22::transfer_from"),
            Some(vec![bob.to_string(), dave.to_string(), user_tokens.to_string(), "[]".to_string()]),
            None,
            transcoder_governance_token(),
        )?;
        let mut sess=call_function(
            sess,
            &gov_token,
            &bob, 
            String::from("PSP22::transfer_from"),
            Some(vec![bob.to_string(), ed.to_string(), user_tokens.to_string(), "[]".to_string()]),
            None,
            transcoder_governance_token(),
        )?;
        sess.upload(bytes_registry()).expect("Session should upload registry bytes");
        sess.upload(bytes_share_token()).expect("Session should upload token bytes");

        let vault = sess.deploy(
            bytes_vault(),
            "new",
            &[
                hash_share_token(),
                hash_registry(),
            ],
            vec![1],
            None,
            &transcoder_vault().unwrap(),
        )?;
        /**
         * vault: AccountId,
            _multisig:AccountId,
            _gov_nft: AccountId,
            exec_threshold: u128,
            reject_threshold: u128,
            acc_threshold: u128,   
         */
        let multisig = sess.deploy(
            bytes_multisig(),
            "new",
            &[
                hash_share_token(),
                hash_registry(),
            ],
            vec![1],
            None,
            &transcoder_vault().unwrap(),
        )?;
        let governance = sess.deploy(
            bytes_governance(),
            "new",
            &[
               vault.to_string(),
               bob.to_string(),
               gov_nft.to_string(),
               EXEC_THRESHOLD.to_string(),
               REJECT_THRESHOLD.to_string(),
               ACC_THRESHOLD.to_string(), 
            ],
            vec![1],
            None,
            &transcoder_governance().unwrap(),
        )?;
        
        Ok(TestContext {
            sess,
            gov_token,
            gov_nft,
            stake_contract,
            governance,
            vault,            
            alice,
            bob,
            charlie,
            dave,
            ed,
        })
        
    }
    fn wrap_tokens(mut ctx:TestContext,amount:u128) -> Result<TestContext, Box<dyn Error>> {       
            let mut sess = call_function(
                ctx.sess,
                &ctx.gov_token,
                &ctx.alice,
                String::from("PSP22::approve"),
                Some(vec![ctx.stake_contract.to_string(), amount.to_string()]),
                None,
                transcoder_governance_token(),
            ).unwrap();
            let mut sess = call_function(
                sess,
                &ctx.stake_contract,
                &ctx.alice,
                String::from("wrap_tokens"),
                Some(vec![amount.to_string(),"None".to_string()]),
                None,
                transcoder_governance_staking(),
            ).unwrap();
            let mut sess = call_function(
                sess,
                &ctx.gov_token,
                &ctx.bob,
                String::from("PSP22::approve"),
                Some(vec![ctx.stake_contract.to_string(), amount.to_string()]),
                None,
                transcoder_governance_token(),
            ).unwrap();
            let mut sess = call_function(
                sess,
                &ctx.stake_contract,
                &ctx.bob,
                String::from("wrap_tokens"),
                Some(vec![amount.to_string(),"None".to_string()]),
                None,
                transcoder_governance_staking(),
            ).unwrap();
            let mut sess = call_function(
                sess,
                &ctx.gov_token,
                &ctx.charlie,
                String::from("PSP22::approve"),
                Some(vec![ctx.stake_contract.to_string(),amount.to_string()]),
                None,
                transcoder_governance_token(),
            ).unwrap();
            let mut sess = call_function(
                sess,
                &ctx.stake_contract,
                &ctx.charlie,
                String::from("wrap_tokens"),
                Some(vec![amount.to_string(),"None".to_string()]),
                None,
                transcoder_governance_staking(),
            ).unwrap();
            let mut sess = call_function(
                sess,
                &ctx.gov_token,
                &ctx.dave,
                String::from("PSP22::approve"),
                Some(vec![ctx.stake_contract.to_string(),amount.to_string()]),
                None,
                transcoder_governance_token(),
            ).unwrap();
            let mut sess = call_function(
                sess,
                &ctx.stake_contract,
                &ctx.dave,
                String::from("wrap_tokens"),
                Some(vec![amount.to_string(),"None".to_string()]),
                None,
                transcoder_governance_staking(),
            ).unwrap();
            let mut sess = call_function(
                sess,
                &ctx.gov_token,
                &ctx.ed,
                String::from("PSP22::approve"),
                Some(vec![ctx.stake_contract.to_string(),amount.to_string()]),
                None,
                transcoder_governance_token(),
            ).unwrap();
            let mut sess = call_function(
                sess,
                &ctx.stake_contract,
                &ctx.ed,
                String::from("wrap_tokens"),
                Some(vec![amount.to_string(),"None".to_string()]),
                None,
                transcoder_governance_staking(),
            ).unwrap();
            ctx.sess=sess;
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
            Some(vec![ctx.stake_contract.to_string(), TOTAL_SUPPLY.to_string()]),
            None,
            transcoder_governance_token(),
        ).unwrap();

  
        
        let  sess = call_function(
            sess,
            &ctx.stake_contract,
            &ctx.bob,
            String::from("wrap_tokens"),
            Some(vec![(TOTAL_SUPPLY/10).to_string(),"None".to_string()]),
            None,
            transcoder_governance_staking(),
        ).unwrap();
        
        let mut sess = call_function(
            sess,
            &ctx.gov_nft,
            &ctx.bob,
            String::from("PSP34::total_supply"),
            Some(vec![]),
            None,
            transcoder_governance_nft(),
        ).unwrap();
        let rr: Result<u128, drink::errors::LangError> = sess.last_call_return().unwrap();
        let total_supply = rr.unwrap();
        println!("{:?}",total_supply);
        let  sess = call_function(
            sess,
            &ctx.stake_contract,
            &ctx.bob,
            String::from("add_token_value"),
            Some(vec![10000_u128.to_string(),1_u128.to_string()]),
            None,
            transcoder_governance_staking(),
        ).unwrap();
        let  sess = call_function(
            sess,
            &ctx.stake_contract,
            &ctx.bob,
            String::from("remove_token_value"),
            Some(vec![5000_u128.to_string(),1_u128.to_string()]),
            None,
            transcoder_governance_staking(),
        ).unwrap();
        let  sess = call_function(
            sess,
            &ctx.gov_nft,
            &ctx.bob,
            String::from("get_governance_data"),
            Some(vec![1_u128.to_string()]),
            None,
            transcoder_governance_nft(),
        ).unwrap();
        let gdata:Result<GovernanceData,drink::errors::LangError>= sess.last_call_return().unwrap();
        println!("{:?}",gdata);
        let expected= (TOTAL_SUPPLY/10)+5000;
        assert_eq!(gdata.unwrap().vote_weight,expected);
        Ok(())           
    }
    #[test]
    fn burn_remint() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup().unwrap();
        ctx=wrap_tokens(ctx,TOTAL_SUPPLY/5).unwrap();
        
        Ok(())
    }
    #[test]
    fn earn_interest() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup().unwrap();
        ctx=wrap_tokens(ctx,TOTAL_SUPPLY/5).unwrap();

        Ok(())
    }
    #[test]
    fn change_interest_rate() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup().unwrap();
        ctx=wrap_tokens(ctx,TOTAL_SUPPLY/5).unwrap();
        let prop= PropType::UpdateStakingRewards(70000000_128);
        let sess=call_function(ctx.sess,&ctx.governance,&ctx.alice,String::from('create_proposal'),Some(vec![prop,1]),None,transcoder_governance());
        Ok(())
    }
    #[test]
    fn make_and_vote_proposal() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup().unwrap();
        ctx=wrap_tokens(ctx,TOTAL_SUPPLY/5).unwrap();

        Ok(())
    }
    #[test]
    fn double_proposals_fail() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup().unwrap();
        ctx=wrap_tokens(ctx,TOTAL_SUPPLY/5).unwrap();

        Ok(())
    }
    #[test]
    fn double_votes_fail() -> Result<(), Box<dyn Error>> {
        let mut ctx = setup().unwrap();
        ctx=wrap_tokens(ctx,TOTAL_SUPPLY/5).unwrap();

        Ok(())
    }
}