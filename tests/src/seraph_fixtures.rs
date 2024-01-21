#![allow(clippy::await_holding_refcell_ref)]
use anchor_lang::{
    prelude::Rent,
    solana_program::{
        clock::Clock,
        pubkey::Pubkey,
        vote::state::{VoteInit, VoteState, VoteStateVersions},
    },
    AccountSerialize, InstructionData, ToAccountMetas,
};
use rand::Rng;
use seraph::{Pool, VList};
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    epoch_schedule::EpochSchedule,
    instruction::Instruction,
    signature::Keypair,
    signer::Signer,
    stake::{
        instruction::{authorize, initialize},
        state::{Authorized, Lockup, StakeAuthorize, StakeState},
    },
    transaction::Transaction,
};
use std::{cell::RefCell, rc::Rc};

use jito_tip_distribution::{
    sdk::derive_tip_distribution_account_address, state::TipDistributionAccount,
};
use validator_history::{self, constants::MAX_ALLOC_BYTES, ClusterHistory, ValidatorHistory};

pub const TOTAL_VALIDATORS: usize = 20;
const AIRDROP_LAMPORTS: u64 = 10_000_000_000_000_000;
const TOTAL_EPOCHS: usize = 50;

pub struct STestFixture {
    pub ctx: Rc<RefCell<ProgramTestContext>>,
    pub vote_accounts: Vec<Keypair>,
    pub identity_keypairs: Vec<Keypair>,
    pub epoch_credits: Vec<Vec<(u64, u64, u64)>>,
    pub commisions: Vec<Vec<u8>>,
    pub cluster_history_account: Pubkey,
    pub validator_history_accounts: Vec<Pubkey>,
    pub validator_history_config: Pubkey,
    pub tip_distribution_accounts: Vec<Pubkey>,
    pub stakers: Vec<Keypair>,
    pub stake_accounts: Vec<Keypair>,
    pub admin: Keypair,
    pub pool: Pubkey,
    pub v_list: Pubkey,
    pub redelegate_stake_accounts: Vec<Keypair>,
    pub keypair: Keypair,
}

impl STestFixture {
    pub async fn new() -> Self {
        /*
           Initializes test context with ValidatorHistory and TipDistribution programs loaded, as well as
           a vote account and a system account for signing transactions.

           Returns a fixture with relevant account addresses and keypairs.
        */
        let mut program = ProgramTest::new(
            "validator-history",
            validator_history::ID,
            processor!(validator_history::entry),
        );
        program.add_program(
            "jito-tip-distribution",
            jito_tip_distribution::id(),
            processor!(jito_tip_distribution::entry),
        );
        program.add_program("seraph", seraph::id(), processor!(seraph::entry));

        let epoch = 0;
        let vote_accounts: Vec<Keypair> = (0..TOTAL_VALIDATORS).map(|_| Keypair::new()).collect();
        let stakers: Vec<Keypair> = (0..TOTAL_VALIDATORS).map(|_| Keypair::new()).collect();
        let stake_accounts: Vec<Keypair> = (0..TOTAL_VALIDATORS).map(|_| Keypair::new()).collect();
        let redelegate_stake_accounts = (0..2).map(|_| Keypair::new()).collect();
        let identity_keypairs: Vec<Keypair> =
            (0..TOTAL_VALIDATORS).map(|_| Keypair::new()).collect();
        let epoch_credits: Vec<Vec<(u64, u64, u64)>> = (0..TOTAL_VALIDATORS)
            .map(|i| {
                (0..TOTAL_EPOCHS)
                    .map(|_| {
                        let second_value = rand::thread_rng().gen_range(20, 46);
                        let third_value = rand::thread_rng().gen_range(5, 19);
                        (i as u64, second_value, third_value)
                    })
                    .collect()
            })
            .collect();
        let commisions: Vec<Vec<u8>> = (0..TOTAL_VALIDATORS)
            .map(|_| {
                (0..TOTAL_EPOCHS)
                    .map(|_| rand::thread_rng().gen_range(5, 14) as u8)
                    .collect()
            })
            .collect();
        let cluster_history_account =
            Pubkey::find_program_address(&[ClusterHistory::SEED], &validator_history::id()).0;
        let tip_distribution_accounts: Vec<Pubkey> = (0..TOTAL_VALIDATORS)
            .map(|i| {
                derive_tip_distribution_account_address(
                    &jito_tip_distribution::id(),
                    &vote_accounts[i].pubkey(),
                    epoch,
                )
                .0
            })
            .collect();
        let validator_history_accounts: Vec<Pubkey> = (0..TOTAL_VALIDATORS)
            .map(|i| {
                Pubkey::find_program_address(
                    &[
                        validator_history::state::ValidatorHistory::SEED,
                        vote_accounts[i].pubkey().as_ref(),
                    ],
                    &validator_history::id(),
                )
                .0
            })
            .collect();
        let validator_history_config = Pubkey::find_program_address(
            &[validator_history::state::Config::SEED],
            &validator_history::id(),
        )
        .0;
        let keypair = Keypair::new();

        // Seraph Accounts
        let admin = Keypair::new();
        let pool = Pool::pubkey(admin.pubkey());
        let v_list = VList::pubkey(admin.pubkey(), pool);
        let rent = Rent::default();
        let stake_account_lamports = rent
            .minimum_balance(std::mem::size_of::<solana_sdk::stake::state::StakeState>())
            + 1_000_000_000_000;

        for i in 0..TOTAL_VALIDATORS {
            // add identities
            program.add_account(
                identity_keypairs[i].pubkey(),
                system_account(AIRDROP_LAMPORTS),
            );
            // add vote accounts
            program.add_account(
                vote_accounts[i].pubkey(),
                new_vote_account(
                    identity_keypairs[i].pubkey(),
                    vote_accounts[i].pubkey(),
                    1,
                    Some(epoch_credits[i][0..10].to_vec()),
                ),
            );

            // add stakers
            program.add_account(stakers[i].pubkey(), system_account(AIRDROP_LAMPORTS));

            // add stake accounts
            program.add_account(
                stake_accounts[i].pubkey(),
                new_stake_account(stake_account_lamports),
            );
        }

        program.add_account(admin.pubkey(), system_account(AIRDROP_LAMPORTS));
        program.add_account(keypair.pubkey(), system_account(AIRDROP_LAMPORTS));

        let ctx = Rc::new(RefCell::new(program.start_with_context().await));

        Self {
            ctx,
            validator_history_config,
            validator_history_accounts,
            cluster_history_account,
            epoch_credits,
            commisions,
            identity_keypairs,
            vote_accounts,
            tip_distribution_accounts,
            stakers,
            stake_accounts,
            admin,
            pool,
            v_list,
            redelegate_stake_accounts,
            keypair,
        }
    }

    pub async fn load_and_deserialize<T: anchor_lang::AccountDeserialize>(
        &self,
        address: &Pubkey,
    ) -> T {
        let ai = self
            .ctx
            .borrow_mut()
            .banks_client
            .get_account(*address)
            .await
            .unwrap()
            .unwrap();

        T::try_deserialize(&mut ai.data.as_slice()).unwrap()
    }

    pub async fn initialize_seraph(&self) {
        let initialize_seraph = Instruction {
            program_id: seraph::id(),
            accounts: seraph::accounts::Initialize {
                admin: self.admin.pubkey(),
                pool: self.pool,
                v_list: self.v_list,
                system_program: anchor_lang::solana_program::system_program::id(),
            }
            .to_account_metas(None),
            data: seraph::instruction::Initialize {}.data(),
        };
        let transaction = Transaction::new_signed_with_payer(
            &[initialize_seraph],
            Some(&self.keypair.pubkey()),
            &[&self.keypair, &self.admin],
            self.ctx
                .borrow_mut()
                .get_new_latest_blockhash()
                .await
                .unwrap(),
        );
        if let Err(e) = self
            .ctx
            .borrow_mut()
            .banks_client
            .process_transaction_with_preflight(transaction)
            .await
        {
            panic!("Error: {}", e);
        }
    }

    pub async fn init_and_auth_stake_accounts_to_admin(&self) {
        for i in 0..TOTAL_VALIDATORS {
            let init_ix = initialize(
                &self.stake_accounts[i].pubkey(),
                &Authorized {
                    staker: self.stakers[i].pubkey(),
                    withdrawer: self.stakers[i].pubkey(),
                },
                &Lockup::default(),
            );

            let auth_s_ix = authorize(
                &self.stake_accounts[i].pubkey(),
                &self.stakers[i].pubkey(),
                &self.admin.pubkey(),
                StakeAuthorize::Staker,
                None,
            );

            let auth_w_ix = authorize(
                &self.stake_accounts[i].pubkey(),
                &self.stakers[i].pubkey(),
                &self.admin.pubkey(),
                StakeAuthorize::Withdrawer,
                None,
            );

            let transaction = Transaction::new_signed_with_payer(
                &[init_ix, auth_w_ix, auth_s_ix],
                Some(&self.keypair.pubkey()),
                &[&self.keypair, &self.stakers[i]],
                self.ctx
                    .borrow_mut()
                    .get_new_latest_blockhash()
                    .await
                    .unwrap(),
            );

            if let Err(e) = self
                .ctx
                .borrow_mut()
                .banks_client
                .process_transaction_with_preflight(transaction)
                .await
            {
                panic!("Error: {}", e);
            }
        }
    }

    pub async fn initialize_config(&self) {
        let instruction = Instruction {
            program_id: validator_history::id(),
            accounts: validator_history::accounts::InitializeConfig {
                config: self.validator_history_config,
                system_program: anchor_lang::solana_program::system_program::id(),
                signer: self.keypair.pubkey(),
            }
            .to_account_metas(None),
            data: validator_history::instruction::InitializeConfig {
                authority: self.keypair.pubkey(),
            }
            .data(),
        };
        let set_tip_distribution_instruction = Instruction {
            program_id: validator_history::id(),
            accounts: validator_history::accounts::SetNewTipDistributionProgram {
                config: self.validator_history_config,
                new_tip_distribution_program: jito_tip_distribution::id(),
                admin: self.keypair.pubkey(),
            }
            .to_account_metas(None),
            data: validator_history::instruction::SetNewTipDistributionProgram {}.data(),
        };
        let transaction = Transaction::new_signed_with_payer(
            &[instruction, set_tip_distribution_instruction],
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            self.ctx
                .borrow_mut()
                .get_new_latest_blockhash()
                .await
                .unwrap(),
        );
        if let Err(e) = self
            .ctx
            .borrow_mut()
            .banks_client
            .process_transaction_with_preflight(transaction)
            .await
        {
            panic!("Error: {}", e);
        }
    }

    pub async fn initialize_validator_history_accounts(&self) {
        for i in 0..TOTAL_VALIDATORS {
            let instruction = Instruction {
                program_id: validator_history::id(),
                accounts: validator_history::accounts::InitializeValidatorHistoryAccount {
                    validator_history_account: self.validator_history_accounts[i],
                    vote_account: self.vote_accounts[i].pubkey(),
                    system_program: anchor_lang::solana_program::system_program::id(),
                    signer: self.keypair.pubkey(),
                }
                .to_account_metas(None),
                data: validator_history::instruction::InitializeValidatorHistoryAccount {}.data(),
            };

            let mut ixs = vec![instruction];

            // Realloc validator history account
            let num_reallocs = (ValidatorHistory::SIZE - MAX_ALLOC_BYTES) / MAX_ALLOC_BYTES + 1;
            ixs.extend(vec![
                Instruction {
                    program_id: validator_history::id(),
                    accounts: validator_history::accounts::ReallocValidatorHistoryAccount {
                        validator_history_account: self.validator_history_accounts[i],
                        vote_account: self.vote_accounts[i].pubkey(),
                        config: self.validator_history_config,
                        system_program: anchor_lang::solana_program::system_program::id(),
                        signer: self.keypair.pubkey(),
                    }
                    .to_account_metas(None),
                    data: validator_history::instruction::ReallocValidatorHistoryAccount {}.data(),
                };
                num_reallocs
            ]);

            let transaction = Transaction::new_signed_with_payer(
                &ixs,
                Some(&self.keypair.pubkey()),
                &[&self.keypair],
                self.ctx
                    .borrow_mut()
                    .get_new_latest_blockhash()
                    .await
                    .unwrap(),
            );
            self.submit_transaction_assert_success(transaction).await;
        }
    }

    pub async fn copy_vote_accounts(&self, total_epochs: usize) {
        for i in 0..TOTAL_VALIDATORS {
            self.ctx.borrow_mut().set_account(
                &self.vote_accounts[i].pubkey(),
                &new_vote_account(
                    self.vote_accounts[i].pubkey(),
                    self.vote_accounts[i].pubkey(),
                    self.commisions[i][total_epochs],
                    Some(self.epoch_credits[i][0..total_epochs].to_vec()),
                )
                .into(),
            );
            let instruction = Instruction {
                program_id: validator_history::id(),
                data: validator_history::instruction::CopyVoteAccount {}.data(),
                accounts: validator_history::accounts::CopyVoteAccount {
                    validator_history_account: self.validator_history_accounts[i],
                    vote_account: self.vote_accounts[i].pubkey(),
                    signer: self.keypair.pubkey(),
                }
                .to_account_metas(None),
            };

            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&self.keypair.pubkey()),
                &[&self.keypair],
                self.ctx
                    .borrow_mut()
                    .get_new_latest_blockhash()
                    .await
                    .unwrap(),
            );
            if let Err(e) = self
                .ctx
                .borrow_mut()
                .banks_client
                .process_transaction_with_preflight(transaction)
                .await
            {
                panic!("Error: {}", e);
            }
        }
    }

    pub async fn initialize_cluster_history_account(&self) {
        let instruction = Instruction {
            program_id: validator_history::id(),
            accounts: validator_history::accounts::InitializeClusterHistoryAccount {
                cluster_history_account: self.cluster_history_account,
                system_program: anchor_lang::solana_program::system_program::id(),
                signer: self.keypair.pubkey(),
            }
            .to_account_metas(None),
            data: validator_history::instruction::InitializeClusterHistoryAccount {}.data(),
        };

        let mut ixs = vec![instruction];

        // Realloc cluster history account
        let num_reallocs = (ClusterHistory::SIZE - MAX_ALLOC_BYTES) / MAX_ALLOC_BYTES + 1;
        ixs.extend(vec![
            Instruction {
                program_id: validator_history::id(),
                accounts: validator_history::accounts::ReallocClusterHistoryAccount {
                    cluster_history_account: self.cluster_history_account,
                    system_program: anchor_lang::solana_program::system_program::id(),
                    signer: self.keypair.pubkey(),
                }
                .to_account_metas(None),
                data: validator_history::instruction::ReallocClusterHistoryAccount {}.data(),
            };
            num_reallocs
        ]);
        let transaction = Transaction::new_signed_with_payer(
            &ixs,
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            self.ctx
                .borrow_mut()
                .get_new_latest_blockhash()
                .await
                .unwrap(),
        );
        self.submit_transaction_assert_success(transaction).await;
    }

    pub async fn advance_num_epochs(&self, num_epochs: u64) {
        let clock: Clock = self
            .ctx
            .borrow_mut()
            .banks_client
            .get_sysvar()
            .await
            .expect("Failed getting clock");
        let epoch_schedule: EpochSchedule = self.ctx.borrow().genesis_config().epoch_schedule;
        let target_epoch = clock.epoch + num_epochs;
        let target_slot = epoch_schedule.get_first_slot_in_epoch(target_epoch);

        self.ctx
            .borrow_mut()
            .warp_to_slot(target_slot)
            .expect("Failed warping to future epoch");
    }

    pub async fn submit_transaction_assert_success(&self, transaction: Transaction) {
        let mut ctx = self.ctx.borrow_mut();
        if let Err(e) = ctx
            .banks_client
            .process_transaction_with_preflight(transaction)
            .await
        {
            panic!("Error: {}", e);
        }
    }

    pub async fn submit_transaction_assert_error(
        &self,
        transaction: Transaction,
        error_message: &str,
    ) {
        if let Err(e) = self
            .ctx
            .borrow_mut()
            .banks_client
            .process_transaction_with_preflight(transaction)
            .await
        {
            assert!(e.to_string().contains(error_message));
        } else {
            panic!("Error: Transaction succeeded. Expected {}", error_message);
        }
    }
}

pub fn system_account(lamports: u64) -> Account {
    Account {
        lamports,
        owner: anchor_lang::system_program::ID,
        executable: false,
        rent_epoch: 0,
        data: vec![],
    }
}

pub fn new_vote_account(
    node_pubkey: Pubkey,
    vote_pubkey: Pubkey,
    commission: u8,
    maybe_epoch_credits: Option<Vec<(u64, u64, u64)>>,
) -> Account {
    let vote_init = VoteInit {
        node_pubkey,
        authorized_voter: vote_pubkey,
        authorized_withdrawer: vote_pubkey,
        commission,
    };
    let clock = Clock {
        epoch: 0,
        slot: 0,
        unix_timestamp: 0,
        leader_schedule_epoch: 0,
        epoch_start_timestamp: 0,
    };
    let mut vote_state = VoteState::new(&vote_init, &clock);
    if let Some(epoch_credits) = maybe_epoch_credits {
        vote_state.epoch_credits = epoch_credits;
    }
    let vote_state_versions = VoteStateVersions::new_current(vote_state);
    let mut data = vec![0; VoteState::size_of()];
    VoteState::serialize(&vote_state_versions, &mut data).unwrap();

    Account {
        lamports: 1000000,
        data,
        owner: anchor_lang::solana_program::vote::program::ID,
        ..Account::default()
    }
}

pub fn new_stake_account(lamports: u64) -> Account {
    let data = vec![0; StakeState::size_of()];
    Account {
        lamports,
        data,
        owner: anchor_lang::solana_program::stake::program::ID,
        ..Account::default()
    }
}

pub fn new_tip_distribution_account(vote_account: Pubkey, mev_commission_bps: u16) -> Account {
    let tda = TipDistributionAccount {
        validator_vote_account: vote_account,
        validator_commission_bps: mev_commission_bps,
        ..TipDistributionAccount::default()
    };
    let mut data = vec![];
    tda.try_serialize(&mut data).unwrap();
    Account {
        lamports: 1000000,
        data,
        owner: jito_tip_distribution::id(),
        ..Account::default()
    }
}
