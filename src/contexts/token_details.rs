use anchor_lang::prelude::*;

#[account]
pub struct TokenDetails {
    pub mint: Pubkey,
    pub project: Pubkey,
    pub project_percentage: u8,
    pub total_supply: u64,
    pub tokens_unlocked: u64,
}