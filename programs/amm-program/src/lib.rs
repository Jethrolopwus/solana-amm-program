#![cfg_attr(not(feature = "no-idl"), allow(unused))]
use anchor_lang::prelude::*;

pub mod state;
pub mod error;
pub mod events;
pub mod contexts;

pub use contexts::*;
pub use events::*;

declare_id!("6hUoxA8ETxkyuJWYBpyGcnho4VxBJGGuqeZ4wWYA7c3E");

#[program]
pub mod amm_program {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        seed: u64,
        fee: u16,
        authority: Option<Pubkey>,
    ) -> Result<()> {
        ctx.accounts.init(seed, fee, authority, &ctx.bumps)
    }

    pub fn deposit(
        ctx: Context<Deposit>,
        lp_amount: u64,
        max_x: u64,
        max_y: u64,
    ) -> Result<()> {
        ctx.accounts.deposit(lp_amount, max_x, max_y)
    }

    pub fn withdraw(
        ctx: Context<Withdraw>,
        lp_amount: u64,
        min_x: u64,
        min_y: u64,
    ) -> Result<()> {
        ctx.accounts.withdraw(lp_amount, min_x, min_y)
    }

    pub fn swap(ctx: Context<Swap>, args: SwapArgs) -> Result<()> {
        ctx.accounts.swap(args)
    }

    pub fn lock_pool(ctx: Context<LockPool>) -> Result<()> {
        ctx.accounts.lock()
    }

    pub fn unlock_pool(ctx: Context<LockPool>) -> Result<()> {
        ctx.accounts.unlock()
    }
}
