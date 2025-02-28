use anchor_lang::prelude::*;
use crate::{contexts::*, errors::HypeBondError};

pub fn create_milestone(
    ctx: Context<CreateMilestone>,
    milestone_id: u8,
    description: String,
    requirements: Vec<String>,
) -> Result<()> {
    let project = &ctx.accounts.project;
    let token_details = &ctx.accounts.token_details;
    let milestone = &mut ctx.accounts.milestone;

    // Validate milestone ID
    require!(
        milestone_id < project.total_milestones,
        HypeBondError::InvalidMilestoneCount
    );

    // Set milestone data
    milestone.project = project.key();
    milestone.milestone_id = milestone_id;
    milestone.description = description;
    milestone.requirements = requirements;
    milestone.completed = false;

    // Calculate token allocations for this milestone
    let milestone_tokens = token_details.total_supply * project.unlock_per_milestone / 10000;
    
    // Calculate token distribution based on project percentages
    let project_tokens = milestone_tokens * token_details.project_percentage as u64 / 100;
    let market_tokens = milestone_tokens - project_tokens;
    
    // Calculate team and treasury tokens from project tokens
    let team_tokens = project_tokens * project.team_allocation.percentage as u64 / 100;
    let treasury_tokens = project_tokens - team_tokens;

    // Set token allocations
    milestone.total_tokens = milestone_tokens;
    milestone.team_tokens = team_tokens;
    milestone.treasury_tokens = treasury_tokens;

    Ok(())
}

#[derive(Accounts)]
#[instruction(milestone_id: u8, description: String)]
pub struct CreateMilestone<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        mut,
        constraint = project.authority == authority.key() @ HypeBondError::NotAuthorized
    )]
    pub project: Account<'info, Project>,
    
    #[account(
        seeds = [b"token", project.key().as_ref()],
        bump
    )]
    pub token_details: Account<'info, TokenDetails>,
    
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 1 + 500 + 4 + 1 + 8 + 8 + 8, // Adjust for your needs
        seeds = [b"milestone", project.key().as_ref(), &[milestone_id]],
        bump
    )]
    pub milestone: Account<'info, Milestone>,
    
    pub system_program: Program<'info, System>,
}