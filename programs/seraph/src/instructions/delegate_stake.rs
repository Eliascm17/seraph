use anchor_lang::{
    prelude::*,
    solana_program::{
        program::invoke_signed,
        stake::{self, instruction::delegate_stake},
        sysvar::stake_history,
    },
};
use anchor_spl::stake::Stake as StakeProgram;

use crate::Pool;

#[derive(Accounts)]
pub struct DelegateStake<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [Pool::SEED, pool.admin.as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    pub clock: Sysvar<'info, Clock>,

    /// CHECK:
    pub validator_vote: AccountInfo<'info>,

    /// CHECK:
    #[account(mut)]
    pub stake_account: AccountInfo<'info>,

    /// CHECK:
    #[account(address = stake_history::ID)]
    pub stake_history: UncheckedAccount<'info>,

    /// CHECK:
    #[account(address = stake::config::ID)]
    pub stake_config: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    pub stake_program: Program<'info, StakeProgram>,
}

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, DelegateStake>) -> Result<()> {
    let DelegateStake {
        admin,
        pool,
        stake_account,
        stake_config,
        stake_history,
        clock,
        validator_vote,
        stake_program,
        ..
    } = ctx.accounts;

    msg!("Delegating stake");

    invoke_signed(
        &delegate_stake(&stake_account.key(), admin.key, validator_vote.key),
        &[
            stake_program.to_account_info(),
            stake_account.to_account_info(),
            admin.to_account_info(),
            validator_vote.to_account_info(),
            clock.to_account_info(),
            stake_history.to_account_info(),
            stake_config.to_account_info(),
        ],
        &[&[Pool::SEED, pool.admin.as_ref(), &[pool.bump]]],
    )?;

    Ok(())
}
