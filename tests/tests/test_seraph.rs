#![allow(clippy::await_holding_refcell_ref)]
use anchor_lang::{prelude::Clock, system_program, InstructionData, ToAccountMetas};
use seraph::VList;
use solana_program_test::*;
use solana_sdk::{
    instruction::Instruction,
    signer::Signer,
    stake::config,
    sysvar::{clock, stake_history},
    transaction::Transaction,
};
use tests::seraph_fixtures::{STestFixture, TOTAL_VALIDATORS};
use validator_history::ValidatorHistory;

#[tokio::test]
async fn test_seraph() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize seraph and config
    let fixture = STestFixture::new().await;
    let ctx = &fixture.ctx;
    fixture.initialize_config().await;
    fixture.initialize_validator_history_accounts().await;

    // seraph specific setup
    fixture.initialize_seraph().await;
    fixture.init_and_auth_stake_accounts_to_admin().await;

    fixture.copy_vote_accounts(1).await;

    let account: ValidatorHistory = fixture
        .load_and_deserialize(&fixture.validator_history_accounts[4])
        .await;

    let clock: Clock = ctx
        .borrow_mut()
        .banks_client
        .get_sysvar()
        .await
        .expect("clock");

    assert_eq!(clock.epoch, 0);
    assert_eq!(account.history.idx, 0);
    assert_eq!(account.history.arr[0].epoch, 0);
    assert!(account.history.arr[0].vote_account_last_update_slot <= clock.slot);

    fixture.advance_num_epochs(6).await;

    fixture.copy_vote_accounts(7).await;

    let account: ValidatorHistory = fixture
        .load_and_deserialize(&fixture.validator_history_accounts[4])
        .await;

    assert_eq!(account.history.idx, 1);
    assert_eq!(account.history.arr[1].epoch, 6);

    // score all validators
    for i in 0..TOTAL_VALIDATORS {
        let instruction = Instruction {
            program_id: seraph::id(),
            data: seraph::instruction::CalculateScore {}.data(),
            accounts: seraph::accounts::CalculateScore {
                admin: fixture.admin.pubkey(),
                validator_history_account: fixture.validator_history_accounts[i],
                vote_account: fixture.vote_accounts[i].pubkey(),
                pool: fixture.pool,
                v_list: fixture.v_list,
            }
            .to_account_metas(None),
        };

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&fixture.admin.pubkey()),
            &[&fixture.admin],
            fixture
                .ctx
                .borrow_mut()
                .get_new_latest_blockhash()
                .await
                .unwrap(),
        );
        if let Err(e) = ctx
            .borrow_mut()
            .banks_client
            .process_transaction_with_preflight(transaction)
            .await
        {
            panic!("Error: {}", e);
        }
    }

    // get top 10% performing validators
    let top_10_percentile = TOTAL_VALIDATORS / 10; // 20 / 10 => 2
    let v_list_account: VList = fixture.load_and_deserialize(&fixture.v_list).await;

    for i in 0..top_10_percentile {
        let instruction = Instruction {
            program_id: seraph::id(),
            data: seraph::instruction::DelegateStake {}.data(),
            accounts: seraph::accounts::DelegateStake {
                admin: fixture.admin.pubkey(),
                stake_account: fixture.stake_accounts[i].pubkey(), // can be any of the stake accounts for for simplicity I put "i"
                clock: clock::id(),
                validator_vote: v_list_account.validators[i].validator,
                stake_history: stake_history::id(),
                stake_config: config::ID,
                pool: fixture.pool,
                system_program: system_program::ID,
                stake_program: solana_sdk::stake::program::ID,
            }
            .to_account_metas(None),
        };

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&fixture.admin.pubkey()),
            &[&fixture.admin],
            fixture
                .ctx
                .borrow_mut()
                .get_new_latest_blockhash()
                .await
                .unwrap(),
        );
        if let Err(e) = ctx
            .borrow_mut()
            .banks_client
            .process_transaction_with_preflight(transaction)
            .await
        {
            panic!("Error: {}", e);
        }
    }

    for i in 0..top_10_percentile {
        let instruction = Instruction {
            program_id: seraph::id(),
            data: seraph::instruction::DeactivateStake {}.data(),
            accounts: seraph::accounts::DeactivateStake {
                admin: fixture.admin.pubkey(),
                stake_account: fixture.stake_accounts[i].pubkey(), // can be any of the stake accounts for for simplicity I put "i"
                clock: clock::id(),
                pool: fixture.pool,
                system_program: system_program::ID,
                stake_program: solana_sdk::stake::program::ID,
            }
            .to_account_metas(None),
        };

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&fixture.admin.pubkey()),
            &[&fixture.admin],
            fixture
                .ctx
                .borrow_mut()
                .get_new_latest_blockhash()
                .await
                .unwrap(),
        );
        if let Err(e) = ctx
            .borrow_mut()
            .banks_client
            .process_transaction_with_preflight(transaction)
            .await
        {
            panic!("Error: {}", e);
        }
    }

    Ok(())
}
