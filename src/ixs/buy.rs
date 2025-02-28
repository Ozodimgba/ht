use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Token, TokenAccount, Mint},
    associated_token::AssociatedToken,
};
use crate::{curve::{BondingCurve, CurveState}, errors::HypeBondError, contexts::Global};

pub fn buy(ctx: Context<Buy>, amount: u64, max_sol_cost: u64) -> Result<()> {
    // 1. Access key accounts
    let curve = &mut ctx.accounts.bonding_curve;
    let user = &ctx.accounts.user;
    let global = &ctx.accounts.global;
    let current_time = Clock::get()?.unix_timestamp;
    
    // Check and potentially update curve state based on time
    curve.check_and_update_state(current_time)?;
    
    // 2. Calculate price using appropriate formula based on phase
    let sol_required = curve.calculate_buy_price(amount, current_time)?;
    require!(sol_required <= max_sol_cost, HypeBondError::TooMuchSolRequired);

    // 3. Calculate fee
    let fee_amount = sol_required
        .checked_mul(global.fee_basis_points)
        .unwrap()
        .checked_div(10000)
        .unwrap();

    // 4. Transfer SOL: User -> Bonding Curve
    let transfer_amount = sol_required.checked_add(fee_amount).unwrap();
    anchor_lang::system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: user.to_account_info(),
                to: ctx.accounts.associated_bonding_curve.to_account_info(),
            },
        ),
        transfer_amount
    )?;

    // 5. Transfer fee: Bonding Curve -> Fee Recipient
    anchor_lang::system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.associated_bonding_curve.to_account_info(),
                to: ctx.accounts.fee_recipient.to_account_info(),
            },
        ),
        fee_amount
    )?;

    // 6. Transfer tokens: Bonding Curve -> User
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.associated_bonding_curve.to_account_info(),
                to: ctx.accounts.associated_user.to_account_info(),
                authority: curve.to_account_info(),
            },
        ),
        amount
    )?;

    // 7. Update bonding curve state
    curve.update_reserves(amount, sol_required, true)?;
    curve.update_volume(amount, current_time);

    // 8. Log phase and price information
    if curve.curve_state == CurveState::PriceDiscovery {
        msg!("Buy in discovery phase: {} tokens at {} lamports per token", 
            amount, 
            curve.get_discovery_price(current_time)?);
    } else {
        msg!("Buy in bonding curve phase: {} tokens for {} SOL", 
            amount, 
            sol_required as f64 / 1_000_000_000.0);
    }

    Ok(())
}

#[derive(Accounts)]
pub struct Buy<'info> {
    // Global config PDA storing fee and initial parameters
    pub global: Account<'info, Global>,
    
    // Account that receives trading fees
    #[account(mut)]
    pub fee_recipient: UncheckedAccount<'info>,
    
    // Token mint for the trading pair
    pub mint: Account<'info, Mint>,
    
    // PDA storing bonding curve state and reserves
    #[account(
        mut,
        seeds = [b"bonding-curve", mint.key().as_ref()],
        bump,
    )]
    pub bonding_curve: Account<'info, BondingCurve>,
    
    // Token account owned by bonding curve (holds tokens)
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = bonding_curve
    )]
    pub associated_bonding_curve: Account<'info, TokenAccount>,
    
    // User's token account to receive/send tokens
    #[account(
        mut, 
        associated_token::mint = mint,
        associated_token::authority = user
    )]
    pub associated_user: Account<'info, TokenAccount>,
    
    // User performing the trade
    #[account(mut)]
    pub user: Signer<'info>,
    
    // Required programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}