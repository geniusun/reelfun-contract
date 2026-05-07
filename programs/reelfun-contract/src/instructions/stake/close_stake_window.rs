use anchor_lang::prelude::*;

use crate::{
    errors::ContractError,
    state::stake::*,
};

#[derive(Accounts)]
pub struct CloseStakeWindow<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        constraint = stake_window.creator == creator.key() @ ContractError::Unauthorized,
    )]
    pub stake_window: Account<'info, StakeWindow>,

    pub clock: Sysvar<'info, Clock>,
}

impl CloseStakeWindow<'_> {
    pub fn handler(ctx: Context<CloseStakeWindow>) -> Result<()> {
        let clock = Clock::get()?;
        let stake_window = &mut ctx.accounts.stake_window;

        require!(
            stake_window.can_close(&clock),
            ContractError::WindowNotReady
        );

        stake_window.is_closed = true;

        msg!(
            "Stake window closed for drama: {}, episode: {}, total weight: {}",
            stake_window.drama_id,
            stake_window.episode_id,
            stake_window.total_stake_weight
        );

        Ok(())
    }
}
