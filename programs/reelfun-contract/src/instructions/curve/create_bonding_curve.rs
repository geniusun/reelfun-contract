use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{
        create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3,
    },
    token::{mint_to, Mint, MintTo, Token, TokenAccount},
};

use crate::state::{bonding_curve::*, global::*, whitelist::*};

use crate::{errors::ContractError, events::CreateEvent};

// locker not used here — lock logic inlined to avoid lifetime/stack issues

#[event_cpi]
#[derive(Accounts)]
#[instruction(params: CreateBondingCurveParams)]
pub struct CreateBondingCurve<'info> {
    #[account(
        init,
        payer = creator,
        mint::decimals = global.mint_decimals,
        mint::authority = bonding_curve,
        mint::freeze_authority = bonding_curve
    )]
    mint: Box<Account<'info, Mint>>,

    #[account(mut)]
    creator: Signer<'info>,
    #[account(
        init,
        payer = creator,
        seeds = [BondingCurve::SEED_PREFIX.as_bytes(), mint.to_account_info().key.as_ref()],
        bump,
        space = 8 + BondingCurve::INIT_SPACE,
    )]
    bonding_curve: Box<Account<'info, BondingCurve>>,

    /// CHECK: Bonding curve ATA - created via CPI in handler, not in try_accounts
    #[account(mut)]
    bonding_curve_token_account: UncheckedAccount<'info>,

    #[account(
        seeds = [Global::SEED_PREFIX.as_bytes()],
        constraint = global.initialized == true @ ContractError::NotInitialized,
        constraint = global.status == ProgramStatus::Running @ ContractError::ProgramNotRunning,
        bump,
    )]
    global: Box<Account<'info, Global>>,
    
    /// CHECK: whitelist PDA - validated manually when whitelist_enabled
    pub whitelist: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Using seed to validate metadata account
    metadata: UncheckedAccount<'info>,

    /// CHECK: system program account
    pub system_program: Program<'info, System>,
    /// CHECK: token program account
    pub token_program: Program<'info, Token>,
    /// CHECK: associated token program account
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// CHECK: token metadata program account
    pub token_metadata_program: UncheckedAccount<'info>,
    /// CHECK: rent account
    pub rent: Sysvar<'info, Rent>,
}

impl CreateBondingCurve<'_> {
    pub fn validate(&self, _params: &CreateBondingCurveParams) -> Result<()> {
        // start_time can be in the future (delayed launch) or past/None (immediate)
        // No restriction needed — creator decides when trading opens
        Ok(())
    }

    pub fn handler(
        mut ctx: Context<CreateBondingCurve>,
        params: CreateBondingCurveParams,
    ) -> Result<()> {
        // Check whitelist manually if enabled
        if ctx.accounts.global.whitelist_enabled {
            let wl_key = ctx.accounts.whitelist.key();
            let creator_key = ctx.accounts.creator.key();
            let (expected_wl, _) = Pubkey::find_program_address(
                &[Whitelist::SEED_PREFIX.as_bytes(), creator_key.as_ref()],
                ctx.program_id,
            );
            require!(wl_key == expected_wl, ContractError::NotWhiteList);
            require!(!ctx.accounts.whitelist.data_is_empty(), ContractError::NotWhiteList);
        }

        // Update bonding curve params
        let clock = Clock::get()?;
        ctx.accounts.bonding_curve.update_from_params(
            ctx.accounts.mint.key(),
            ctx.accounts.creator.key(),
            &ctx.accounts.global,
            &params,
            &clock,
            ctx.bumps.bonding_curve,
        );
        msg!("CreateBondingCurve::update_from_params: created bonding_curve");

        // Create ATA via CPI (moved from try_accounts to handler to reduce stack)
        Self::create_ata(&ctx.accounts)?;

        // Initialize metadata + mint tokens
        Self::do_mint_and_meta(&ctx.accounts, &params, ctx.bumps.bonding_curve)?;

        // Lock: revoke mint authority, freeze ATA, emit event
        {
            let mint_k = ctx.accounts.mint.key();
            let signer = BondingCurve::get_signer(&ctx.bumps.bonding_curve, &mint_k);
            let signer_seeds: &[&[&[u8]]; 1] = &[&signer[..]];

            // Revoke mint authority
            anchor_spl::token::set_authority(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    anchor_spl::token::SetAuthority {
                        current_authority: ctx.accounts.bonding_curve.to_account_info(),
                        account_or_mint: ctx.accounts.mint.to_account_info(),
                    },
                    signer_seeds,
                ),
                anchor_spl::token::spl_token::instruction::AuthorityType::MintTokens,
                None,
            )?;

            // Freeze ATA
            anchor_spl::token::freeze_account(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    anchor_spl::token::FreezeAccount {
                        account: ctx.accounts.bonding_curve_token_account.to_account_info(),
                        mint: ctx.accounts.mint.to_account_info(),
                        authority: ctx.accounts.bonding_curve.to_account_info(),
                    },
                    signer_seeds,
                ),
            )?;
        }

        // Emit event
        let bc = &ctx.accounts.bonding_curve;
        emit!(CreateEvent {
            name: params.name.clone(),
            symbol: params.symbol.clone(),
            uri: params.uri.clone(),
            mint: ctx.accounts.mint.key(),
            creator: ctx.accounts.creator.key(),
            virtual_sol_reserves: bc.virtual_sol_reserves,
            virtual_token_reserves: bc.virtual_token_reserves,
            token_total_supply: bc.token_total_supply,
            real_sol_reserves: bc.real_sol_reserves,
            real_token_reserves: bc.real_token_reserves,
            start_time: bc.start_time,
        });

        msg!("CreateBondingCurve::handler: success");
        Ok(())
    }

    #[inline(never)]
    fn create_ata(accounts: &CreateBondingCurve) -> Result<()> {
        // Create ATA for bonding curve via CPI
        anchor_spl::associated_token::create(
            CpiContext::new(
                accounts.associated_token_program.to_account_info(),
                anchor_spl::associated_token::Create {
                    payer: accounts.creator.to_account_info(),
                    associated_token: accounts.bonding_curve_token_account.to_account_info(),
                    authority: accounts.bonding_curve.to_account_info(),
                    mint: accounts.mint.to_account_info(),
                    system_program: accounts.system_program.to_account_info(),
                    token_program: accounts.token_program.to_account_info(),
                },
            ),
        )?;
        msg!("CreateBondingCurve::create_ata: done");
        Ok(())
    }

    #[inline(never)]
    fn do_mint_and_meta<'info>(
        accounts: &CreateBondingCurve<'info>,
        params: &CreateBondingCurveParams,
        bonding_curve_bump: u8,
    ) -> Result<()> {
        let mint_k = accounts.mint.key();
        let mint_authority_signer = BondingCurve::get_signer(&bonding_curve_bump, &mint_k);
        let mint_auth_signer_seeds = &[&mint_authority_signer[..]];

        Self::do_meta(accounts, mint_auth_signer_seeds, params)?;

        let token_total_supply = accounts.bonding_curve.token_total_supply;
        mint_to(
            CpiContext::new_with_signer(
                accounts.token_program.to_account_info(),
                MintTo {
                    authority: accounts.bonding_curve.to_account_info(),
                    to: accounts.bonding_curve_token_account.to_account_info(),
                    mint: accounts.mint.to_account_info(),
                },
                mint_auth_signer_seeds,
            ),
            token_total_supply,
        )?;
        Ok(())
    }

    #[inline(never)]
    fn do_meta<'info>(
        accounts: &CreateBondingCurve<'info>,
        mint_auth_signer_seeds: &[&[&[u8]]; 1],
        params: &CreateBondingCurveParams,
    ) -> Result<()> {
        let token_data: DataV2 = DataV2 {
            name: params.name.clone(),
            symbol: params.symbol.clone(),
            uri: params.uri.clone(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };
        let metadata_ctx = CpiContext::new_with_signer(
            accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                payer: accounts.creator.to_account_info(),
                mint: accounts.mint.to_account_info(),
                metadata: accounts.metadata.to_account_info(),
                update_authority: accounts.bonding_curve.to_account_info(),
                mint_authority: accounts.bonding_curve.to_account_info(),
                system_program: accounts.system_program.to_account_info(),
                rent: accounts.rent.to_account_info(),
            },
            mint_auth_signer_seeds,
        );
        create_metadata_accounts_v3(metadata_ctx, token_data, false, true, None)?;
        msg!("CreateBondingCurve::do_meta: done");
        Ok(())
    }
}
