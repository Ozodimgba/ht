use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Mint, Token, TokenAccount},
    associated_token::AssociatedToken,
};
use curve::BondingCurve;

mod errors;
mod curve;
mod constants;

declare_id!("BiGyz1fq35QxV357XKBUxVHXaHim9MnEk51J9aRB9FBZ");

#[program]
mod hype_terminal {
    use errors::HypeterminalError;

    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let global = &mut ctx.accounts.global;
        global.initialized = true;
        global.authority = ctx.accounts.user.key();
        Ok(())
    }

    pub fn set_params(
        ctx: Context<SetParams>,
        fee_recipient: Pubkey,
        initial_virtual_token_reserves: u64,
        initial_virtual_sol_reserves: u64,
        initial_real_token_reserves: u64,
        token_total_supply: u64,
        fee_basis_points: u64,
    ) -> Result<()> {
        require!(ctx.accounts.global.initialized, HypeterminalError::NotInitialized);
        require!(
            ctx.accounts.user.key() == ctx.accounts.global.authority,
            HypeterminalError::NotAuthorized
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

        Ok(())
    }

    pub fn buy(ctx: Context<Trade>, amount: u64, max_sol_cost: u64) -> Result<()> {
        // 1. Access key accounts
        let curve = &mut ctx.accounts.bonding_curve;
        let user = &ctx.accounts.user;
        let global = &ctx.accounts.global;
        
        // 2. Calculate price using bonding curve formula
        let sol_required = curve.calculate_buy_price(amount)?;
        require!(sol_required <= max_sol_cost, HypeterminalError::TooMuchSolRequired);
    
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
        curve.update_volume(amount, Clock::get()?.unix_timestamp);
    
        // 8. Emit trade event
    
        Ok(())
    }

    pub fn sell(ctx: Context<Trade>, amount: u64, min_sol_output: u64) -> Result<()> {
        // 1. Access key accounts & validate
        let curve = &mut ctx.accounts.bonding_curve;
        let user = &ctx.accounts.user;
        let global = &ctx.accounts.global;
        
        require!(!curve.complete, HypeterminalError::BondingCurveComplete);
        require!(
            amount >= curve.min_trade_amount && amount <= curve.max_trade_amount,
            HypeterminalError::InvalidTradeSize
        );
    
        // 2. Calculate SOL output using bonding curve with sell protections
        let sol_output = curve.calculate_sell_price(amount)?;
        require!(sol_output >= min_sol_output, HypeterminalError::TooLittleSolReceived);
    
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
        curve.update_volume(amount, Clock::get()?.unix_timestamp);
    
        // Calculate price impact (optional)
        let virtual_token_ratio = curve.virtual_token_reserves
            .checked_div(curve.token_total_supply)
            .unwrap_or(0);
    
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = user,
        space = 8 + std::mem::size_of::<Global>(),
        seeds = [b"global"],
        bump
    )]
    pub global: Account<'info, Global>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
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

    pub system_program: Program<'info, System>,

    /// CHECK: Event authority
    pub event_authority: UncheckedAccount<'info>,

    /// CHECK: Program
    pub program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Trade<'info> {
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
        // constraint = !bonding_curve.complete @ ErrorCode::BondingCurveComplete <- Implement better constraint
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

#[account]
#[derive(Default)]
pub struct Global {
    pub initialized: bool,
    pub authority: Pubkey,
    pub fee_recipient: Pubkey,
    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_real_token_reserves: u64,
    pub token_total_supply: u64,
    pub fee_basis_points: u64,
    // Market health parameters
    pub min_trade_amount: u64,
    pub max_trade_amount: u64,
    pub base_slippage: u64,
    pub volume_multiplier: u64,
}