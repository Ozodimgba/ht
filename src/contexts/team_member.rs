use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TeamAllocation {
    pub wallet: Pubkey,
    pub percentage: u8,    // Percentage of team allocation
}