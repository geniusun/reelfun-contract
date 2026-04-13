use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use crate::{
    errors::ContractError,
    state::{bonding_curve::BondingCurve, stake::*, global::Global},
};

#[derive(Accounts)]
#[instruction(params: OpenStakeWindowParams)]
pub struct OpenStakeWindow<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        seeds = [Global::SEED_PREFIX.as_bytes()],
        bump,
    )]
    pub global: Box<Account<'info, Global>>,

    pub mint: Box<Account<'info, Mint>>,

    #[account(
        seeds = [BondingCurve::SEED_PREFIX.as_bytes(), mint.key().as_ref()],
        constraint = bonding_curve.creator == creator.key() @ ContractError::Unauthorized,
        bump,
    )]
    pub bonding_curve: Box<Account<'info, BondingCurve>>,

    #[account(
        init,
        payer = creator,
        space = 8 + StakeWindow::INIT_SPACE,
        seeds = [
            StakeWindow::SEED_PREFIX.as_bytes(),
            params.drama_id.as_ref(),
            params.episode_id.to_le_bytes().as_ref(),
        ],
        bump,
    )]
    pub stake_window: Account<'info, StakeWindow>,

    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>,
}

impl OpenStakeWindow<'_> {
    pub fn handler(ctx: Context<OpenStakeWindow>, params: OpenStakeWindowParams) -> Result<()> {
        let clock = Clock::get()?;
        let stake_window = &mut ctx.accounts.stake_window;

        require!(
            params.window_duration_hours >= 24 && params.window_duration_hours <= 48,
            ContractError::InvalidWindowDuration
        );

        let window_duration_seconds = (params.window_duration_hours as i64) * 3600;

        stake_window.drama_id = params.drama_id;
        stake_window.episode_id = params.episode_id;
        stake_window.bonding_curve = ctx.accounts.bonding_curve.key();
        stake_window.creator = ctx.accounts.creator.key();
        stake_window.open_time = clock.unix_timestamp;
        stake_window.close_time = clock.unix_timestamp + window_duration_seconds;
        stake_window.is_closed = false;
        stake_window.total_stake_weight = 0;
        stake_window.bump = ctx.bumps.stake_window;

        msg!(
            "Stake window opened for drama: {}, episode: {}, closes at: {}",
            params.drama_id,
            params.episode_id,
            stake_window.close_time
        );

        Ok(())
    }
}
