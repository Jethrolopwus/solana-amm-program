use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SwapArgs {
    pub amount: u64,
    pub is_x: bool,
    pub min_out: u64,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    pub system_program: Program<'info, System>,
}

impl<'info> Swap<'info> {
    pub fn swap(&mut self, _args: SwapArgs) -> Result<()> {
        Ok(())
    }
}