use anchor_lang::prelude::*;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod util;
pub mod constants;
use instructions::{
    create_bonding_curve::*, initialize::*, set_params::*, swap::*, create_pool::*, lock_pool::*, add_wl::*, remove_wl::*,
    open_stake_window::*, stake::*, close_stake_window::*, unstake::*, refund_deposit::*
};
use state::bonding_curve::CreateBondingCurveParams;
use state::global::*;
use state::stake::{OpenStakeWindowParams, StakeParams};
declare_id!("AWiK9JaGPGBjM6Zn7yB8dNf8qCXcvizxJ1zKnkEXxQpp");

#[program]
pub mod reelfun_contract {

    use super::*;

    pub fn initialize(ctx: Context<Initialize>, params: GlobalSettingsInput) -> Result<()> {
        Initialize::handler(ctx, params)
    }
    pub fn set_params(ctx: Context<SetParams>, params: GlobalSettingsInput) -> Result<()> {
        SetParams::handler(ctx, params)
    }

    pub fn create_pool(ctx: Context<InitializePoolWithConfig>) -> Result<()> {
        instructions::initialize_pool_with_config(ctx)
    }

    pub fn lock_pool(ctx: Context<LockPool>) -> Result<()> {
        instructions::lock_pool(ctx)
    }

    pub fn add_wl(ctx: Context<AddWl>, new_creator: Pubkey) -> Result<()> {
        AddWl::handler(ctx, new_creator)
    }

    pub fn remove_wl(_ctx: Context<RemoveWl>) -> Result<()> {
        // Account closure handled by Anchor's `close` constraint on RemoveWl
        Ok(())
    }

    pub fn open_stake_window(ctx: Context<OpenStakeWindow>, params: OpenStakeWindowParams) -> Result<()> {
        OpenStakeWindow::handler(ctx, params)
    }

    pub fn stake(ctx: Context<Stake>, params: StakeParams) -> Result<()> {
        Stake::handler(ctx, params)
    }

    pub fn close_stake_window(ctx: Context<CloseStakeWindow>) -> Result<()> {
        CloseStakeWindow::handler(ctx)
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        Unstake::handler(ctx)
    }

    pub fn refund_deposit(ctx: Context<RefundDeposit>, params: RefundDepositParams) -> Result<()> {
        RefundDeposit::handler(ctx, params)
    }

    #[access_control(ctx.accounts.validate(&params))]
    pub fn create_bonding_curve(
        ctx: Context<CreateBondingCurve>,
        params: CreateBondingCurveParams,
    ) -> Result<()> {
        CreateBondingCurve::handler(ctx, params)
    }

    #[access_control(ctx.accounts.validate(&params))]
    pub fn swap(ctx: Context<Swap>, params: SwapParams) -> Result<()> {
        Swap::handler(ctx, params)
    }
}
