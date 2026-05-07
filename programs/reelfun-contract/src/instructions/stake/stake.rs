use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

use crate::{
    errors::ContractError,
    state::{bonding_curve::BondingCurve, stake::*, global::Global},
};

#[derive(Accounts)]
#[instruction(params: StakeParams)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [Global::SEED_PREFIX.as_bytes()],
        bump,
    )]
    pub global: Box<Account<'info, Global>>,

    pub mint: Box<Account<'info, Mint>>,

    #[account(
        seeds = [BondingCurve::SEED_PREFIX.as_bytes(), mint.key().as_ref()],
        bump,
    )]
    pub bonding_curve: Box<Account<'info, BondingCurve>>,

    #[account(
        mut,
        seeds = [
            StakeWindow::SEED_PREFIX.as_bytes(),
            params.drama_id.as_ref(),
            params.episode_id.to_le_bytes().as_ref(),
        ],
        constraint = stake_window.bonding_curve == bonding_curve.key() @ ContractError::InvalidBondingCurve,
        bump,
    )]
    pub stake_window: Box<Account<'info, StakeWindow>>,

    #[account(
        init,
        payer = user,
        space = 8 + UserStake::INIT_SPACE,
        seeds = [
            UserStake::SEED_PREFIX.as_bytes(),
            stake_window.key().as_ref(),
            user.key().as_ref(),
        ],
        bump,
    )]
    pub user_stake: Box<Account<'info, UserStake>>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    pub user_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint,
        associated_token::authority = stake_window,
    )]
    pub stake_token_account: Box<Account<'info, TokenAccount>>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub clock: Sysvar<'info, Clock>,
}

impl Stake<'_> {
    pub fn handler(ctx: Context<Stake>, params: StakeParams) -> Result<()> {
        let clock = Clock::get()?;
        let stake_window = &mut ctx.accounts.stake_window;
        let user_stake = &mut ctx.accounts.user_stake;

        // Validate window is open
        require!(
            stake_window.is_open(&clock),
            ContractError::WindowNotOpen
        );

        // Validate stake amount
        require!(
            params.stake_amount > 0,
            ContractError::InvalidStakeAmount
        );

        // Validate lock days (1, 3, 7, 30, or custom up to 365)
        require!(
            params.lock_days > 0 && params.lock_days <= 365,
            ContractError::InvalidLockDays
        );

        // Validate prompt length
        require!(
            params.prompt.len() > 0 && params.prompt.len() <= 500,
            ContractError::InvalidPrompt
        );

        // Transfer tokens from user to stake account
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.stake_token_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, params.stake_amount)?;

        // Transfer deposit (0.01 SOL) from user to stake_window account
        let deposit_transfer = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &stake_window.key(),
            UserStake::DEPOSIT_AMOUNT,
        );
        anchor_lang::solana_program::program::invoke(
            &deposit_transfer,
            &[
                ctx.accounts.user.to_account_info(),
                stake_window.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // Calculate weight
        let stake_weight = UserStake::calculate_weight(params.stake_amount, params.lock_days);

        // Initialize user stake
        user_stake.stake_window = stake_window.key();
        user_stake.user = ctx.accounts.user.key();
        user_stake.bonding_curve = ctx.accounts.bonding_curve.key();
        user_stake.stake_amount = params.stake_amount;
        user_stake.lock_days = params.lock_days;
        user_stake.stake_weight = stake_weight;
        user_stake.normalized_weight = stake_weight; // Will be updated when window closes
        user_stake.prompt = params.prompt.clone();
        user_stake.deposit_paid = UserStake::DEPOSIT_AMOUNT;
        user_stake.stake_time = clock.unix_timestamp;
        user_stake.unlock_time = clock.unix_timestamp + ((params.lock_days as i64) * 86400);
        user_stake.is_unlocked = false;
        user_stake.deposit_refunded = false;
        user_stake.bump = ctx.bumps.user_stake;

        // Update total stake weight (normalized weights will be calculated on window close)
        stake_window.total_stake_weight = stake_window
            .total_stake_weight
            .checked_add(stake_weight)
            .ok_or(ContractError::Overflow)?;

        msg!(
            "User {} staked {} tokens for {} days, weight: {}, prompt: '{}'",
            user_stake.user,
            user_stake.stake_amount,
            user_stake.lock_days,
            user_stake.stake_weight,
            user_stake.prompt
        );

        Ok(())
    }
}
