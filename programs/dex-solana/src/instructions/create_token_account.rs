use crate::error::ErrorCode;
use crate::utils::is_token_account_initialized;
use crate::{token_program, wsol_program};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::program_pack::Pack;
use anchor_lang::solana_program::{system_instruction, system_program};
use anchor_spl::token::TokenAccount;
use anchor_spl::token::spl_token::state::Account as SplTokenAccount;
use anchor_spl::{
    token::Token,
    token_interface::{self, Mint},
};

#[derive(Accounts)]
pub struct CreateTokenAccountAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: This is the owner of the token account
    pub owner: AccountInfo<'info>,

    /// CHECK: This is the wsol token account to be created
    #[account(
        mut,
        constraint = token_account.owner == &system_program::id() || token_account.owner == &token_program::id(),
        seeds = [token_mint.key().as_ref(), owner.key().as_ref()],
        bump,
    )]
    pub token_account: AccountInfo<'info>,

    #[account(address = wsol_program::id())]
    pub token_mint: InterfaceAccount<'info, Mint>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

pub fn create_token_account_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CreateTokenAccountAccounts<'a>>,
    bump: u8,
) -> Result<()> {
    let token_account = ctx.accounts.token_account.to_account_info();
    let owner = ctx.accounts.owner.to_account_info();
    let token_mint = ctx.accounts.token_mint.to_account_info();
    let token_program = ctx.accounts.token_program.to_account_info();

    // Allocate/assign/initialize only. Funding (lamports) is done by external instructions.
    let space = TokenAccount::LEN as u64; // SPL Token account size

    // PDA seeds for the token account
    let seeds: &[&[u8]] = &[token_mint.key.as_ref(), owner.key.as_ref(), &[bump]];
    let signer_seeds: &[&[&[u8]]] = &[seeds];

    // before allocate, check if the account is already initialized
    if is_token_account_initialized(&token_account) {
        let data = token_account.try_borrow_data()?;
        let ta = SplTokenAccount::unpack(&data)?;

        require_keys_eq!(ta.owner, owner.key(), ErrorCode::InvalidTokenAccount);

        msg!("Token account already initialized");
        return Ok(());
    }

    // 1) allocate space for the account (allocate)
    let ix_allocate = system_instruction::allocate(token_account.key, space);
    invoke_signed(&ix_allocate, &[token_account.clone()], signer_seeds)?;

    // 2) assign ownership to the token program (assign)
    let ix_assign = system_instruction::assign(token_account.key, token_program.key);
    invoke_signed(&ix_assign, &[token_account.clone()], signer_seeds)?;

    // 3) initialize as token account (initializeAccount3)
    token_interface::initialize_account3(CpiContext::new(
        token_program,
        token_interface::InitializeAccount3 {
            account: token_account,
            mint: token_mint,
            authority: owner,
        },
    ))?;

    Ok(())
}
