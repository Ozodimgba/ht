use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::{contexts::*, errors::HypeBondError};

pub fn complete_milestone(ctx: Context<CompleteMilestone>, milestone_id: u8) -> Result<()> {
    let project = &mut ctx.accounts.project;
    let milestone = &mut ctx.accounts.milestone;
    let token_details = &mut ctx.accounts.token_details;
    
    // Verify milestone ID
    require!(
        milestone_id == milestone.milestone_id,
        HypeBondError::InvalidMilestoneCount
    );
    
    // Verify milestone belongs to project
    require!(
        milestone.project == project.key(),
        HypeBondError::NotAuthorized
    );
    
    // Verify milestone is not already completed
    require!(
        !milestone.completed,
        HypeBondError::AlreadyInitialized // Could create a more specific error
    );
    
    // Mark milestone as completed
    milestone.completed = true;
    
    // Update project completed milestones
    project.completed_milestones += 1;
    
    // Update token details
    token_details.tokens_unlocked += milestone.total_tokens;
    
    let project_key = project.key();
    // Transfer team tokens
    if milestone.team_tokens > 0 {
        let seeds = &[
            b"vault".as_ref(),
            project_key.as_ref(),
            &[ctx.bumps.vault]
        ];
        let signer = &[&seeds[..]];
        
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.team_account.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                },
                signer
            ),
            milestone.team_tokens
        )?;
    }
    
    // Transfer treasury tokens
    if milestone.treasury_tokens > 0 {
        let seeds = &[
            b"vault".as_ref(),
            project_key.as_ref(),
            &[ctx.bumps.vault]
        ];
        let signer = &[&seeds[..]];
        
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.treasury_account.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                },
                signer
            ),
            milestone.treasury_tokens
        )?;
    }
    
    Ok(())
}

#[derive(Accounts)]
#[instruction(milestone_id: u8)]
pub struct CompleteMilestone<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        mut,
        constraint = project.authority == authority.key() @ HypeBondError::NotAuthorized
    )]
    pub project: Account<'info, Project>,
    
    #[account(
        mut,
        seeds = [b"token", project.key().as_ref()],
        bump
    )]
    pub token_details: Account<'info, TokenDetails>,
    
    #[account(
        mut,
        seeds = [b"milestone", project.key().as_ref(), &[milestone_id]],
        bump,
        constraint = milestone.project == project.key() @ HypeBondError::NotAuthorized
    )]
    pub milestone: Account<'info, Milestone>,
    
    #[account(
        mut,
        seeds = [b"vault", project.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = team_account.owner == project.team_allocation.wallet @ HypeBondError::NotAuthorized
    )]
    pub team_account: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub treasury_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}