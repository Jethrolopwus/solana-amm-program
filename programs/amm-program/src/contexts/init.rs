use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{create as ata_create, AssociatedToken, Create},
    token_interface::{Mint, TokenInterface},
};
use crate::events::PoolInitialized;
use crate::state::Config;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,

    pub mint_x: Box<InterfaceAccount<'info, Mint>>,
    pub mint_y: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        payer = initializer,
        seeds = [b"lp", config.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = config,
    )]
    pub mint_lp: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: created via CPI in handler
    #[account(mut)]
    pub vault_x: UncheckedAccount<'info>,

    /// CHECK: created via CPI in handler
    #[account(mut)]
    pub vault_y: UncheckedAccount<'info>,

    #[account(
        init,
        payer = initializer,
        space = Config::INIT_SPACE,
        seeds = [
            b"config",
            mint_x.key().to_bytes().as_ref(),
            mint_y.key().to_bytes().as_ref(),
            seed.to_le_bytes().as_ref(),
        ],
        bump,
    )]
    pub config: Box<Account<'info, Config>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Initialize<'info> {
    pub fn init(
        &mut self,
        seed: u64,
        fee: u16,
        authority: Option<Pubkey>,
        bumps: &InitializeBumps,
    ) -> Result<()> {
        // Create vault_x ATA
        ata_create(CpiContext::new(
            self.associated_token_program.to_account_info(),
            Create {
                payer: self.initializer.to_account_info(),
                associated_token: self.vault_x.to_account_info(),
                authority: self.config.to_account_info(),
                mint: self.mint_x.to_account_info(),
                system_program: self.system_program.to_account_info(),
                token_program: self.token_program.to_account_info(),
            },
        ))?;

        // Create vault_y ATA
        ata_create(CpiContext::new(
            self.associated_token_program.to_account_info(),
            Create {
                payer: self.initializer.to_account_info(),
                associated_token: self.vault_y.to_account_info(),
                authority: self.config.to_account_info(),
                mint: self.mint_y.to_account_info(),
                system_program: self.system_program.to_account_info(),
                token_program: self.token_program.to_account_info(),
            },
        ))?;

        self.config.set_inner(Config {
            seed,
            authority,
            mint_x: self.mint_x.key(),
            mint_y: self.mint_y.key(),
            fee,
            locked: false,
            config_bump: bumps.config,
            lp_bump: bumps.mint_lp,
        });

        emit!(PoolInitialized {
            config: self.config.key(),
            mint_x: self.mint_x.key(),
            mint_y: self.mint_y.key(),
            fee,
        });

        Ok(())
    }
}
