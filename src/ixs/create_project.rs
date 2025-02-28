use anchor_lang::prelude::*;
use anchor_spl::token::{spl_token::instruction::AuthorityType, Mint, MintTo, SetAuthority, Token, TokenAccount};
use crate::{contexts::*, errors::HypeBondError};

#[derive(Accounts)]
#[instruction(name: String, ticker: String)]
pub struct CreateProject<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 100 + 10 + 500 + 1 + 1 + 8 + 32 + 1, // Adjust for TeamAllocation size
        seeds = [b"project", name.as_bytes(), ticker.as_bytes()],
        bump
    )]
    pub project: Account<'info, Project>,
    
    // Fix: Don't use project.key() in seeds because project is being initialized
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 32 + 1 + 1 + 8 + 8,
        seeds = [b"token", name.as_bytes(), ticker.as_bytes()],
        bump
    )]
    pub token_details: Account<'info, TokenDetails>,
    
    // Fix: Don't use mint as its own authority
    #[account(
        init,
        payer = authority,
        seeds = [b"mint", name.as_bytes(), ticker.as_bytes()],
        bump,
        mint::decimals = 6,
        mint::authority = authority,
    )]
    pub mint: Account<'info, Mint>,
    
    // Fix: Don't use vault as its own authority and don't reference project.key()
    #[account(
        init,
        payer = authority,
        seeds = [b"vault", name.as_bytes(), ticker.as_bytes()],
        bump,
        token::mint = mint,
        token::authority = authority,
    )]
    pub vault: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}


pub fn create_project(
    ctx: Context<CreateProject>,
    name: String,
    ticker: String,
    description: String,
    total_milestones: u8,
    team_percentage: u8,    // Percentage of non-market tokens that go to team
    treasury_percentage: u8, // Percentage of non-market tokens that go to treasury
    team_allocation: TeamAllocation, // should optionally be a squads multisig
) -> Result<()> {
    require!(team_percentage + treasury_percentage < 100, HypeBondError::InvalidPercentages);
    require!(total_milestones > 3, HypeBondError::InvalidMilestoneCount);

    let project = &mut ctx.accounts.project;
    let token_details = &mut ctx.accounts.token_details;

    project.name = name;
    project.ticker = ticker;
    project.description = description;
    project.authority = ctx.accounts.authority.key(); // should be a metadao Dao
    project.total_milestones = total_milestones;
    project.completed_milestones = 0;
    project.team_allocation = team_allocation;

    let unlock_per_milestone = 10000 / total_milestones as u64;
    project.unlock_per_milestone = unlock_per_milestone;

    token_details.mint = ctx.accounts.mint.key();
    token_details.project = project.key();
    token_details.project_percentage = team_percentage + treasury_percentage;
    token_details.total_supply = 1_000_000_000; // 1 billion (hard coded)
    token_details.tokens_unlocked = 0; // milestone zero has to be defined

    // Mint total supply to vault
    let mint_to_vault = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(), // Changed from mint to authority
        },
    );

    anchor_spl::token::mint_to(
        mint_to_vault, 
        token_details.total_supply
    )?;

    // Set up the mint authority to null
    let set_authority = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        SetAuthority {
            current_authority: ctx.accounts.authority.to_account_info(), // Changed from mint to authority
            account_or_mint: ctx.accounts.mint.to_account_info(),
        },
    );
        
    anchor_spl::token::set_authority(
        set_authority,
        AuthorityType::MintTokens,
        None,  // Setting to None removes the authority
    )?;
        
    let set_freeze_authority = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        SetAuthority {
            current_authority: ctx.accounts.authority.to_account_info(), // Changed from mint to authority
            account_or_mint: ctx.accounts.mint.to_account_info(),
        },
    );
        
    anchor_spl::token::set_authority(
        set_freeze_authority,
        AuthorityType::FreezeAccount,
        None,  // Setting to None removes the freeze authority
    )?;

   Ok(())
}