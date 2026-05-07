use anchor_lang::prelude::*;

use crate::{
    errors::ContractError,
    state::stake::*,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RefundDepositParams {
    pub generation_complete_time: i64,
}

#[derive(Accounts)]
pub struct RefundDeposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

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

    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>,
}

impl RefundDeposit<'_> {
    pub fn handler(ctx: Context<RefundDeposit>, params: RefundDepositParams) -> Result<()> {
        let clock = Clock::get()?;
        let user_stake = &mut ctx.accounts.user_stake;
        let stake_window = &ctx.accounts.stake_window;

        require!(
            !user_stake.deposit_refunded,
            ContractError::DepositAlreadyRefunded
        );

        require!(
            user_stake.can_refund_deposit(&clock, params.generation_complete_time),
            ContractError::RefundNotReady
        );

        // Transfer deposit back to user from stake_window account
        let episode_id_bytes = stake_window.episode_id.to_le_bytes();
        let seeds = &[
            StakeWindow::SEED_PREFIX.as_bytes(),
            stake_window.drama_id.as_ref(),
            episode_id_bytes.as_ref(),
            &[stake_window.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let refund_instruction = anchor_lang::solana_program::system_instruction::transfer(
            &stake_window.key(),
            &user_stake.user,
            user_stake.deposit_paid,
        );

        anchor_lang::solana_program::program::invoke_signed(
            &refund_instruction,
            &[
                stake_window.to_account_info(),
                ctx.accounts.user.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            signer_seeds,
        )?;

        user_stake.deposit_refunded = true;

        msg!(
            "Refunded {} lamports deposit to user {}",
            user_stake.deposit_paid,
            user_stake.user
        );

        Ok(())
    }
}
