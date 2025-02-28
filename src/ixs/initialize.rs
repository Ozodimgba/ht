use anchor_lang::prelude::*;
use crate::contexts::*;

pub fn initialize(ctx: Context<Initialize>) -> anchor_lang::Result<()> {
    let global = &mut ctx.accounts.global;
    global.initialized = true;
    global.authority = ctx.accounts.user.key();
    Ok(())
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