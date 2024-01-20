use anchor_lang::prelude::*;
use validator_history::ValidatorHistory;

use crate::{Pool, VList};

#[derive(Accounts)]
pub struct CalculateScore<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECK:
    pub validator_history_account: UncheckedAccount<'info>,

    /// CHECK:
    pub vote_account: UncheckedAccount<'info>,

    #[account(
        seeds = [Pool::SEED, pool.admin.as_ref()],
        bump,
        has_one = admin
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        seeds = [VList::SEED, admin.key.as_ref(), pool.key().as_ref()],
        bump,
        has_one = pool
    )]
    pub v_list: Account<'info, VList>,
}

pub fn handler(ctx: Context<CalculateScore>) -> Result<()> {
    let CalculateScore {
        validator_history_account,
        vote_account,
        v_list,
        ..
    } = ctx.accounts;

    let clock = Clock::get()?;

    // deserialize validator history acc
    let validator_history_data = validator_history_account.try_borrow_mut_data()?;
    let mut validator_history_slice: &[u8] = &validator_history_data;
    let validator_history = ValidatorHistory::try_deserialize(&mut validator_history_slice)?;

    // Calculate score based on the last 5 entries
    let current_epoch = clock.epoch;
    let start_epoch = if current_epoch > 5 {
        current_epoch - 5
    } else {
        0
    };
    let end_epoch = current_epoch;

    let epoch_entries = validator_history
        .history
        .epoch_range(start_epoch as u16, end_epoch as u16);

    let mut total_score = 0;
    let mut entries_count = 0;

    for entry_option in epoch_entries {
        if let Some(entry) = entry_option {
            let epoch_credits = entry.epoch_credits;
            let commission = entry.commission;
            let commission_percentage = commission as f64 / 100.0;
            total_score += (epoch_credits as f64 * (1.0 - commission_percentage)) as u32;
            entries_count += 1;
        }
    }

    // Calculate the average score if there are valid entries
    if entries_count > 0 {
        let average_score = total_score / entries_count;
        v_list.insert_or_update(vote_account.key(), average_score, current_epoch);
    }

    Ok(())
}
