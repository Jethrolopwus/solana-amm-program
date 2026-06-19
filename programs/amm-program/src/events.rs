use anchor_lang::prelude::*;

#[event]
pub struct PoolInitialized {
    pub config: Pubkey,
    pub mint_x: Pubkey,
    pub mint_y: Pubkey,
    pub fee: u16,
}

#[event]
pub struct LiquidityAdded {
    pub config: Pubkey,
    pub lp_provider: Pubkey,
    pub amount_x: u64,
    pub amount_y: u64,
    pub lp_minted: u64,
}

#[event]
pub struct LiquidityRemoved {
    pub config: Pubkey,
    pub lp_provider: Pubkey,
    pub amount_x: u64,
    pub amount_y: u64,
    pub lp_burned: u64,
}

#[event]
pub struct Swapped {
    pub config: Pubkey,
    pub user: Pubkey,
    pub is_x: bool,
    pub amount_in: u64,
    pub amount_out: u64,
}
