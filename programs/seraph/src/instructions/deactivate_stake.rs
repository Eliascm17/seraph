use anchor_lang::prelude::*;
use anchor_spl::stake::{
    deactivate_stake, DeactivateStake as DeactivateStakeAccount, Stake as StakeProgram,
};

use crate::Pool;

#[derive(Accounts)]
pub struct DeactivateStake<'info> {
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

    pub system_program: Program<'info, System>,

    pub stake_program: Program<'info, StakeProgram>,
}

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, DeactivateStake>) -> Result<()> {
    let DeactivateStake {
        admin,
        pool,
        stake_account,
        clock,
        system_program: _,
        stake_program,
    } = ctx.accounts;

    msg!("Deactivating Stake");

    deactivate_stake(CpiContext::new_with_signer(
        stake_program.to_account_info(),
        DeactivateStakeAccount {
            stake: stake_account.to_account_info(),
            staker: admin.to_account_info(),
            clock: clock.to_account_info(),
        },
        &[&[Pool::SEED, pool.admin.as_ref(), &[pool.bump]]],
    ))?;

    Ok(())
}
