use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer_checked, TransferChecked},
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use constant_product_curve::{ConstantProduct, LiquidityPair};

use crate::error::AmmError;
use crate::events::Swapped;
use crate::state::Config;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SwapArgs {
    pub is_x: bool,
    pub amount: u64,
    pub min: u64,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    pub user: Signer<'info>,

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
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump,
        mint::decimals = 6,
        mint::authority = config,
    )]
    pub mint_lp: Box<InterfaceAccount<'info, Mint>>,

    pub mint_x: Box<InterfaceAccount<'info, Mint>>,
    pub mint_y: Box<InterfaceAccount<'info, Mint>>,

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
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_ata_x: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_ata_y: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Swap<'info> {
    pub fn swap(&mut self, args: SwapArgs) -> Result<()> {
        require!(args.amount > 0, AmmError::InvalidAmount);
        require!(!self.config.locked, AmmError::AMMLocked);

        let mut curve = ConstantProduct::init(
            self.vault_x.amount,
            self.vault_y.amount,
            self.mint_lp.supply,
            self.config.fee,
            None,
        )
        .map_err(AmmError::from)?;

        let pair = if args.is_x { LiquidityPair::X } else { LiquidityPair::Y };

        let res = curve.swap(pair, args.amount, args.min).map_err(AmmError::from)?;

        require_neq!(res.deposit, 0, AmmError::InvalidAmount);
        require_neq!(res.withdraw, 0, AmmError::InvalidAmount);

        self.transfer_to_vault(&args, res.deposit)?;
        self.withdraw_from_vault(&args, res.withdraw)?;

        emit!(Swapped {
            config: self.config.key(),
            user: self.user.key(),
            is_x: args.is_x,
            amount_in: res.deposit,
            amount_out: res.withdraw,
        });

        Ok(())
    }

    fn transfer_to_vault(&mut self, args: &SwapArgs, amount: u64) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let (cpi_accounts, decimals) = if args.is_x {
            (
                TransferChecked {
                    from: self.user_ata_x.to_account_info(),
                    mint: self.mint_x.to_account_info(),
                    to: self.vault_x.to_account_info(),
                    authority: self.user.to_account_info(),
                },
                self.mint_x.decimals,
            )
        } else {
            (
                TransferChecked {
                    from: self.user_ata_y.to_account_info(),
                    mint: self.mint_y.to_account_info(),
                    to: self.vault_y.to_account_info(),
                    authority: self.user.to_account_info(),
                },
                self.mint_y.decimals,
            )
        };

        transfer_checked(CpiContext::new(cpi_program, cpi_accounts), amount, decimals)?;
        Ok(())
    }

    fn withdraw_from_vault(&mut self, args: &SwapArgs, amount: u64) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let (cpi_accounts, decimals) = if args.is_x {
            (
                TransferChecked {
                    from: self.vault_y.to_account_info(),
                    mint: self.mint_y.to_account_info(),
                    to: self.user_ata_y.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                self.mint_y.decimals,
            )
        } else {
            (
                TransferChecked {
                    from: self.vault_x.to_account_info(),
                    mint: self.mint_x.to_account_info(),
                    to: self.user_ata_x.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                self.mint_x.decimals,
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
}
