use anchor_lang::{
    prelude::*,
    solana_program::{
        program::invoke_signed,
        stake::state::StakeState,
        stake::{self, instruction::redelegate},
        sysvar::stake_history,
    },
};
use anchor_spl::stake::{Stake as StakeProgram, StakeAccount};

use crate::Pool;

#[derive(Accounts)]
pub struct RedelegateStake<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [Pool::SEED, pool.admin.as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    /// CHECK:
    #[account(mut)]
    pub stake_account: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    /// CHECK:
    pub old_validator_vote: AccountInfo<'info>,

    /// CHECK:
    pub new_validator_vote: AccountInfo<'info>,

    /// CHECK:
    #[account(address = stake_history::ID)]
    pub stake_history: UncheckedAccount<'info>,

    /// CHECK:
    #[account(address = stake::config::ID)]
    pub stake_config: UncheckedAccount<'info>,

    // new stake account to make the reDelegation
    #[account(
        init,
        payer = admin,
        space = std::mem::size_of::<StakeState>(),
        owner = stake::program::ID,
    )]
    pub redelegate_stake_account: Account<'info, StakeAccount>,

    pub system_program: Program<'info, System>,

    pub stake_program: Program<'info, StakeProgram>,
}

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, RedelegateStake>) -> Result<()> {
    let RedelegateStake {
        admin,
        pool,
        stake_account,
        stake_config,
        stake_history: _,
        clock: _,
        old_validator_vote: _,
        new_validator_vote,
        system_program: _,
        stake_program: _,
        redelegate_stake_account,
    } = ctx.accounts;

    let redelegate_ix = redelegate(
        stake_account.key,
        admin.key,
        new_validator_vote.key,
        &redelegate_stake_account.key(),
    )
    .last()
    .unwrap()
    .clone();

    msg!("Redelegating Stake");

    // redelegate account to dest validator
    invoke_signed(
        &redelegate_ix,
        &[
            stake_account.to_account_info(),
            new_validator_vote.to_account_info(),
            redelegate_stake_account.to_account_info(),
            stake_config.to_account_info(),
            admin.to_account_info(),
        ],
        &[&[Pool::SEED, pool.admin.as_ref(), &[pool.bump]]],
    )?;

    Ok(())
}
