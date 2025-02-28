use anchor_lang::prelude::*;

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