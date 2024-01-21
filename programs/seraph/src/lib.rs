use anchor_lang::prelude::*;

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("9EK4NR8LwFBzV6jCNYoshQ9yyuM6yDxB8zsqrhTsK5z");

#[program]
pub mod seraph {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handler(ctx)
    }

    pub fn calculate_score(ctx: Context<CalculateScore>) -> Result<()> {
        calculate_score::handler(ctx)
    }

    pub fn delegate_stake<'info>(ctx: Context<'_, '_, '_, 'info, DelegateStake>) -> Result<()> {
        delegate_stake::handler(ctx)
    }

    pub fn redelegate_stake<'info>(ctx: Context<'_, '_, '_, 'info, RedelegateStake>) -> Result<()> {
        redelegate_stake::handler(ctx)
    }

    pub fn deactivate_stake<'info>(ctx: Context<'_, '_, '_, 'info, DeactivateStake>) -> Result<()> {
        deactivate_stake::handler(ctx)
    }
}
