use anchor_lang::prelude::*;

use crate::error::AmmError;
use crate::state::Config;

#[derive(Accounts)]
pub struct LockPool<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        constraint = config.authority == Some(authority.key()) @ AmmError::InvalidConfig,
        seeds = [
            b"config",
            config.mint_x.to_bytes().as_ref(),
            config.mint_y.to_bytes().as_ref(),
            config.seed.to_le_bytes().as_ref(),
        ],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,
}

impl<'info> LockPool<'info> {
    pub fn lock(&mut self) -> Result<()> {
        self.config.locked = true;
        Ok(())
    }

    pub fn unlock(&mut self) -> Result<()> {
        self.config.locked = false;
        Ok(())
    }
}
