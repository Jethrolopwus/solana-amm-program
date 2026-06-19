use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{burn, transfer_checked, Burn, TransferChecked};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use constant_product_curve::ConstantProduct;

use crate::error::AmmError;
use crate::events::LiquidityRemoved;
use crate::state::Config;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub lp_provider: Signer<'info>,

    pub mint_x: Box<InterfaceAccount<'info, Mint>>,
    pub mint_y: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        has_one = mint_x,
        has_one = mint_y,
        seeds = [
            b"config",
            mint_x.key().to_bytes().as_ref(),
            mint_y.key().to_bytes().as_ref(),
            config.seed.to_le_bytes().as_ref(),
        ],
        bump = config.config_bump,
    )]
    pub config: Box<Account<'info, Config>>,

    #[account(
        mut,
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump,
        mint::decimals = 6,
        mint::authority = config,
    )]
    pub mint_lp: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_x: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_y: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = lp_provider,
        associated_token::token_program = token_program,
    )]
    pub lp_provider_ata_x: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = lp_provider,
        associated_token::token_program = token_program,
    )]
    pub lp_provider_ata_y: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_lp,
        associated_token::authority = lp_provider,
        associated_token::token_program = token_program,
    )]
    pub lp_provider_ata_lp: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, lp_amount: u64, min_x: u64, min_y: u64) -> Result<()> {
        require!(lp_amount > 0, AmmError::InvalidAmount);
        require!(!self.config.locked, AmmError::AMMLocked);

        let amounts = ConstantProduct::xy_withdraw_amounts_from_l(
            self.vault_x.amount,
            self.vault_y.amount,
            self.mint_lp.supply,
            lp_amount,
            6,
        )
        .map_err(AmmError::from)?;

        require!(min_x <= amounts.x, AmmError::InsufficientTokenX);
        require!(min_y <= amounts.y, AmmError::InsufficientTokenY);

        self.withdraw_tokens(true, amounts.x)?;
        self.withdraw_tokens(false, amounts.y)?;
        self.burn_lp_tokens(lp_amount)?;

        emit!(LiquidityRemoved {
            config: self.config.key(),
            lp_provider: self.lp_provider.key(),
            amount_x: amounts.x,
            amount_y: amounts.y,
            lp_burned: lp_amount,
        });

        Ok(())
    }

    fn withdraw_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let (cpi_accounts, decimals) = if is_x {
            (
                TransferChecked {
                    from: self.vault_x.to_account_info(),
                    mint: self.mint_x.to_account_info(),
                    to: self.lp_provider_ata_x.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                self.mint_x.decimals,
            )
        } else {
            (
                TransferChecked {
                    from: self.vault_y.to_account_info(),
                    mint: self.mint_y.to_account_info(),
                    to: self.lp_provider_ata_y.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                self.mint_y.decimals,
            )
        };

        let mint_x = self.mint_x.key();
        let mint_y = self.mint_y.key();
        let mint_x_bytes = mint_x.to_bytes();
        let mint_y_bytes = mint_y.to_bytes();
        let seed = self.config.seed.to_le_bytes();
        let bump = [self.config.config_bump];
        let seeds: [&[u8]; 5] = [
            b"config",
            mint_x_bytes.as_ref(),
            mint_y_bytes.as_ref(),
            seed.as_ref(),
            bump.as_ref(),
        ];

        transfer_checked(
            CpiContext::new_with_signer(cpi_program, cpi_accounts, &[&seeds]),
            amount,
            decimals,
        )?;
        Ok(())
    }

    fn burn_lp_tokens(&mut self, amount: u64) -> Result<()> {
        let cpi_accounts = Burn {
            mint: self.mint_lp.to_account_info(),
            from: self.lp_provider_ata_lp.to_account_info(),
            authority: self.lp_provider.to_account_info(),
        };
        burn(
            CpiContext::new(self.token_program.to_account_info(), cpi_accounts),
            amount,
        )?;
        Ok(())
    }
}
