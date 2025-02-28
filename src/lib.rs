use anchor_lang::prelude::*;

mod errors;
mod curve;
mod constants;
mod ixs;
mod contexts;

// No need to use imports here at the top level
// The problem is these imports don't propagate into the program module
use ixs::*;
use contexts::*;

declare_id!("BiGyz1fq35QxV357XKBUxVHXaHim9MnEk51J9aRB9FBZ");

#[program]
pub mod hype_bond {
    use super::*; // This imports everything from the parent scope including anchor_lang::prelude::*
          
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ixs::initialize(ctx)
    }
    
    pub fn set_params(
        ctx: Context<SetParams>,
        fee_recipient: Pubkey,
        initial_virtual_token_reserves: u64,
        initial_virtual_sol_reserves: u64,
        initial_real_token_reserves: u64,
        token_total_supply: u64,
        fee_basis_points: u64,
        discovery_duration: i64,
        discovery_start_price: u64,
        discovery_end_price: u64,
    ) -> Result<()> {
        ixs::set_params(
            ctx,
            fee_recipient,
            initial_virtual_token_reserves,
            initial_virtual_sol_reserves,
            initial_real_token_reserves,
            token_total_supply,
            fee_basis_points,
            discovery_duration,
            discovery_start_price,
            discovery_end_price,
        )
    }
    
    pub fn create_project(
        ctx: Context<CreateProject>,
        name: String,
        ticker: String,
        description: String,
        total_milestones: u8,
        team_percentage: u8,
        treasury_percentage: u8,
        team_allocation: TeamAllocation,
    ) -> Result<()> {
        ixs::create_project(
            ctx,
            name,
            ticker,
            description,
            total_milestones,
            team_percentage,
            treasury_percentage,
            team_allocation,
        )
    }
    
    pub fn create_milestone(
        ctx: Context<CreateMilestone>,
        milestone_id: u8,
        description: String,
        requirements: Vec<String>,
    ) -> Result<()> {
        ixs::create_milestone(ctx, milestone_id, description, requirements)
    }
    
    pub fn complete_milestone(ctx: Context<CompleteMilestone>, milestone_id: u8) -> Result<()> {
        ixs::complete_milestone(ctx, milestone_id)
    }
    
    pub fn buy(ctx: Context<Buy>, amount: u64, max_sol_cost: u64) -> Result<()> {
        ixs::buy(ctx, amount, max_sol_cost)
    }
    
    pub fn sell(ctx: Context<Sell>, amount: u64, min_sol_output: u64) -> Result<()> {
        ixs::sell(ctx, amount, min_sol_output)
    }
}