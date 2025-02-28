use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Token, TokenAccount, Mint},
    associated_token::AssociatedToken,
};
use crate::{curve::{BondingCurve, CurveState}, errors::HypeBondError, contexts::Global};

pub fn sell(ctx: Context<Sell>, amount: u64, min_sol_output: u64) -> Result<()> {
    // 1. Access key accounts & validate
    let curve = &mut ctx.accounts.bonding_curve;
    let user = &ctx.accounts.user;
    let global = &ctx.accounts.global;
    let current_time = Clock::get()?.unix_timestamp;
    
    // Check and potentially update curve state based on time
    curve.check_and_update_state(current_time)?;
    
    require!(!curve.complete, HypeBondError::BondingCurveComplete);
    require!(
        amount >= curve.min_trade_amount && amount <= curve.max_trade_amount,
        HypeBondError::InvalidTradeSize
    );

    // 2. Calculate SOL output using appropriate formula based on phase
    let sol_output = curve.calculate_sell_price(amount, current_time)?;
    require!(sol_output >= min_sol_output, HypeBondError::TooLittleSolReceived);

    // 3. Calculate fee (from SOL output)
    let fee_amount = sol_output
        .checked_mul(global.fee_basis_points)
        .unwrap()
        .checked_div(10000)
        .unwrap();

    // 4. Transfer tokens: User -> Bonding Curve
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.associated_user.to_account_info(),
                to: ctx.accounts.associated_bonding_curve.to_account_info(),
                authority: user.to_account_info(),
            },
        ),
        amount
    )?;

    // 5. Transfer SOL minus fee: Bonding Curve -> User
    let user_sol_amount = sol_output.checked_sub(fee_amount).unwrap();
    **ctx.accounts.associated_bonding_curve.to_account_info().try_borrow_mut_lamports()? -= user_sol_amount;
    **ctx.accounts.user.to_account_info().try_borrow_mut_lamports()? += user_sol_amount;

    // 6. Transfer fee: Bonding Curve -> Fee Recipient 
    **ctx.accounts.associated_bonding_curve.to_account_info().try_borrow_mut_lamports()? -= fee_amount;
    **ctx.accounts.fee_recipient.to_account_info().try_borrow_mut_lamports()? += fee_amount;

    // 7. Update bonding curve state with sell impact
    curve.update_reserves(amount, sol_output, false)?;
    curve.update_volume(amount, current_time);

    // 8. Log phase and price information
    if curve.curve_state == CurveState::PriceDiscovery {
        msg!("Sell in discovery phase: {} tokens at {} lamports per token", 
            amount, 
            curve.get_discovery_price(current_time)?);
    } else {
        msg!("Sell in bonding curve phase: {} tokens for {} SOL", 
            amount, 
            sol_output as f64 / 1_000_000_000.0);
        
        // Calculate price impact (optional)
        let virtual_token_ratio = curve.virtual_token_reserves
            .checked_div(curve.token_total_supply)
            .unwrap_or(0);
        
        msg!("Current token reserves ratio: {}/{}",
            curve.virtual_token_reserves,
            curve.token_total_supply);
    }

    Ok(())
}

#[derive(Accounts)]
pub struct Sell<'info> {
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