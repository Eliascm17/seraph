use anchor_lang::prelude::*;

use crate::{Pool, VList};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init, 
        payer = admin, 
        space = Pool::SIZE,
        seeds = [Pool::SEED, admin.key.as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        init, 
        payer = admin, 
        space = VList::SIZE,
        seeds = [VList::SEED, admin.key.as_ref(), pool.key().as_ref()],
        bump
    )]
    pub v_list: Account<'info, VList>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    let Initialize {
        admin, 
        pool, 
        v_list,
        system_program: _
    } = ctx.accounts;

    // get meta
    let clock = Clock::get()?;
    let pool_bump = *ctx.bumps.get("pool").unwrap();
    let v_list_bump= *ctx.bumps.get("v_list").unwrap();

    // init accounts
    pool.init(
        admin.key, 
        clock.slot, 
        clock.epoch, 
        pool_bump
    )?;

    v_list.init(admin.key,pool.key(), v_list_bump)?;

    Ok(())
}
