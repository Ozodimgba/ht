use anchor_lang::prelude::*;

#[account]
pub struct Milestone {
    pub project: Pubkey,
    pub milestone_id: u8,
    pub description: String,
    pub requirements: Vec<String>,
    pub completed: bool,
    pub total_tokens: u64,
    pub team_tokens: u64,
    pub treasury_tokens: u64,
}