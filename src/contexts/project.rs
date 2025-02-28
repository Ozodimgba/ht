use anchor_lang::prelude::*;
use super::team_member::TeamAllocation;

#[account]
pub struct Project {
    pub authority: Pubkey,
    pub name: String,
    pub ticker: String,
    pub description: String,
    pub total_milestones: u8,
    pub completed_milestones: u8,
    pub unlock_per_milestone: u64,  // In basis points (100.00%)
    pub team_allocation: TeamAllocation,
}