
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, _lp_amount: u64, _min_x: u64, _min_y: u64) -> Result<()> {
        Ok(())
    }
}