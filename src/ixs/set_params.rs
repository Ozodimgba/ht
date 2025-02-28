use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use crate::{curve::BondingCurve, errors::HypeBondError, contexts::Global};

pub fn set_params(
    ctx: Context<SetParams>,
    fee_recipient: Pubkey,
    initial_virtual_token_reserves: u64,
    initial_virtual_sol_reserves: u64,
    initial_real_token_reserves: u64,
    token_total_supply: u64,
    fee_basis_points: u64,
    discovery_duration: i64,        // Duration of discovery phase in seconds
    discovery_start_price: u64,     // Starting price in lamports per token
    discovery_end_price: u64,       // Ending price in lamports per token
) -> Result<()> {
    require!(ctx.accounts.global.initialized, HypeBondError::NotInitialized);
    require!(
        ctx.accounts.user.key() == ctx.accounts.global.authority,
        HypeBondError::NotAuthorized
    );

    let global = &mut ctx.accounts.global;
    global.fee_recipient = fee_recipient;
    global.initial_virtual_token_reserves = initial_virtual_token_reserves;
    global.initial_virtual_sol_reserves = initial_virtual_sol_reserves;
    global.initial_real_token_reserves = initial_real_token_reserves;
    global.token_total_supply = token_total_supply;
    global.fee_basis_points = fee_basis_points;

    // Set market health parameters
    global.min_trade_amount = 10; // Example: 10 tokens minimum
    global.max_trade_amount = token_total_supply / 10; // Example: 10% of supply
    global.base_slippage = 100; // 1% base slippage
    global.volume_multiplier = 10; // 0.1% additional slippage per volume threshold

    // Initialize bonding curve with discovery parameters
    if let Some(bonding_curve) = &mut ctx.accounts.bonding_curve {
        let current_time = Clock::get()?.unix_timestamp;
        bonding_curve.initialize_discovery(
            current_time,
            discovery_duration,
            discovery_start_price,
            discovery_end_price,
            token_total_supply,
        )?;
        msg!("Initialized discovery phase for bonding curve");
    }

    Ok(())
}

#[derive(Accounts)]
pub struct SetParams<'info> {
    #[account(
        mut,
        seeds = [b"global"],
        bump
    )]
    pub global: Account<'info, Global>,

    #[account(mut)]
    pub user: Signer<'info>,

    // Optional bonding curve account
    #[account(
        mut,
        seeds = [b"bonding-curve", mint.key().as_ref()],
        bump,
    )]
    pub bonding_curve: Option<Account<'info, BondingCurve>>,
    
    // Mint for the bonding curve
    pub mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,

    /// CHECK: Event authority
    pub event_authority: UncheckedAccount<'info>,

    /// CHECK: Program
    pub program: UncheckedAccount<'info>,
}