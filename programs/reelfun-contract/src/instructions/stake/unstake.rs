use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

use crate::{
    errors::ContractError,
    state::stake::*,
};

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        constraint = stake_window.is_closed @ ContractError::WindowStillOpen,
    )]
    pub stake_window: Account<'info, StakeWindow>,

    #[account(
        mut,
        constraint = user_stake.stake_window == stake_window.key() @ ContractError::InvalidStakeWindow,
        constraint = user_stake.user == user.key() @ ContractError::Unauthorized,
    )]
    pub user_stake: Account<'info, UserStake>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = stake_window,
    )]
    pub stake_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    pub user_token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub clock: Sysvar<'info, Clock>,
}

impl Unstake<'_> {
    pub fn handler(ctx: Context<Unstake>) -> Result<()> {
        let clock = Clock::get()?;
        let user_stake = &mut ctx.accounts.user_stake;

        require!(
            !user_stake.is_unlocked,
            ContractError::AlreadyUnlocked
        );

        require!(
            user_stake.can_unlock(&clock),
            ContractError::StillLocked
        );

        // Transfer tokens back to user
        let stake_window = &ctx.accounts.stake_window;
        let episode_id_bytes = stake_window.episode_id.to_le_bytes();
        let seeds = &[
            StakeWindow::SEED_PREFIX.as_bytes(),
            stake_window.drama_id.as_ref(),
            episode_id_bytes.as_ref(),
            &[stake_window.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.stake_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: stake_window.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token::transfer(cpi_ctx, user_stake.stake_amount)?;

        user_stake.is_unlocked = true;

        msg!(
            "User {} unlocked {} tokens",
            user_stake.user,
            user_stake.stake_amount
        );

        Ok(())
    }
}
