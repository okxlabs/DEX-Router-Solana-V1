use super::common::DexProcessor;
use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{
    HopAccounts, PERPETUALS_ADDLIQ_SELECTOR, PERPETUALS_REMOVELIQ_SELECTOR,
    PERPETUALS_SWAP_SELECTOR, perpetuals_program,
};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::{token::Token, token_interface::TokenAccount};
use arrayref::array_ref;

pub struct PerpetualsProcessor;
impl DexProcessor for PerpetualsProcessor {}

pub const PERPETUALS_SWAP_ACCOUNTS_LEN: usize = 17;

// for perpetuals addLiquidity2/removeLiquidity2
pub const PERPETUALS_LIQUIDITY_ACCOUNTS_LEN: usize = 14;
pub const PEEPETUALS_REMAINING_ACCOUNTS_LEN: usize = 5 * 3; // 5 custody accounts + 5 dove price accounts + 5 pythnet price accounts

pub struct PerpetualsAccount<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub owner: &'info AccountInfo<'info>,
    pub funding_or_receiving_account: InterfaceAccount<'info, TokenAccount>, // funding/receiving token account
    pub jlp_token_account: InterfaceAccount<'info, TokenAccount>,            // JLP token account

    pub perpetuals_vault_authority: &'info AccountInfo<'info>, // transfer authority
    pub perpetuals_state: &'info AccountInfo<'info>,
    pub perpetuals_pool: &'info AccountInfo<'info>,
    pub collateral_custody: &'info AccountInfo<'info>,
    pub doves_price_account: &'info AccountInfo<'info>,
    pub pythnet_price_account: &'info AccountInfo<'info>,
    pub custody_token_account: InterfaceAccount<'info, TokenAccount>,
    pub jlp_mint: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub event_authority: &'info AccountInfo<'info>,
    // remaining accounts
    pub remaining_accounts: &'info [AccountInfo<'info>; PEEPETUALS_REMAINING_ACCOUNTS_LEN],
}

impl<'info> PerpetualsAccount<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            owner,
            funding_or_receiving_account,
            jlp_token_account,
            perpetuals_vault_authority,
            perpetuals_state,
            perpetuals_pool,
            collateral_custody,
            doves_price_account,
            pythnet_price_account,
            custody_token_account,
            jlp_mint,
            token_program,
            event_authority,
        ]: &[AccountInfo<'info>; PERPETUALS_LIQUIDITY_ACCOUNTS_LEN] =
            array_ref![accounts, offset, PERPETUALS_LIQUIDITY_ACCOUNTS_LEN];
        let remaining_accounts = array_ref![
            accounts,
            offset + PERPETUALS_LIQUIDITY_ACCOUNTS_LEN,
            PEEPETUALS_REMAINING_ACCOUNTS_LEN
        ];

        Ok(Self {
            dex_program_id,
            owner,
            funding_or_receiving_account: InterfaceAccount::try_from(funding_or_receiving_account)?,
            jlp_token_account: InterfaceAccount::try_from(jlp_token_account)?,
            perpetuals_vault_authority,
            perpetuals_state,
            perpetuals_pool,
            collateral_custody,
            doves_price_account,
            pythnet_price_account,
            custody_token_account: InterfaceAccount::try_from(custody_token_account)?,
            jlp_mint,
            token_program: Program::try_from(token_program)?,
            event_authority,
            remaining_accounts,
        })
    }

    fn get_accountmetas(&self) -> Vec<AccountMeta> {
        let mut metas = Vec::with_capacity(
            PERPETUALS_LIQUIDITY_ACCOUNTS_LEN + PEEPETUALS_REMAINING_ACCOUNTS_LEN,
        );
        metas.extend([
            AccountMeta::new_readonly(self.owner.key(), true),
            AccountMeta::new(self.funding_or_receiving_account.key(), false),
            AccountMeta::new(self.jlp_token_account.key(), false),
            AccountMeta::new_readonly(self.perpetuals_vault_authority.key(), false),
            AccountMeta::new_readonly(self.perpetuals_state.key(), false),
            AccountMeta::new(self.perpetuals_pool.key(), false),
            AccountMeta::new(self.collateral_custody.key(), false),
            AccountMeta::new_readonly(self.doves_price_account.key(), false),
            AccountMeta::new_readonly(self.pythnet_price_account.key(), false),
            AccountMeta::new(self.custody_token_account.key(), false),
            AccountMeta::new(self.jlp_mint.key(), false),
            AccountMeta::new_readonly(self.token_program.key(), false),
            AccountMeta::new_readonly(self.event_authority.key(), false),
            AccountMeta::new_readonly(self.dex_program_id.key(), false),
        ]);

        metas.extend(self.remaining_accounts.iter().map(|account| {
            if account.key.eq(self.collateral_custody.key) {
                AccountMeta::new(account.key(), false)
            } else {
                AccountMeta::new_readonly(account.key(), false)
            }
        }));
        metas
    }

    fn get_accountinfos(&self) -> Vec<AccountInfo<'info>> {
        let mut account_infos = Vec::with_capacity(
            PERPETUALS_LIQUIDITY_ACCOUNTS_LEN + PEEPETUALS_REMAINING_ACCOUNTS_LEN,
        );
        account_infos.extend([
            self.owner.to_account_info(),
            self.funding_or_receiving_account.to_account_info(),
            self.jlp_token_account.to_account_info(),
            self.perpetuals_vault_authority.to_account_info(),
            self.perpetuals_state.to_account_info(),
            self.perpetuals_pool.to_account_info(),
            self.collateral_custody.to_account_info(),
            self.doves_price_account.to_account_info(),
            self.pythnet_price_account.to_account_info(),
            self.custody_token_account.to_account_info(),
            self.jlp_mint.to_account_info(),
            self.token_program.to_account_info(),
            self.event_authority.to_account_info(),
            self.dex_program_id.to_account_info(),
        ]);
        account_infos
            .extend(self.remaining_accounts.iter().map(|account| account.to_account_info()));
        account_infos
    }
}

pub struct PerpetualsSwapAccount<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub owner: &'info AccountInfo<'info>,
    pub funding_account: InterfaceAccount<'info, TokenAccount>,
    pub receiving_account: InterfaceAccount<'info, TokenAccount>,

    pub perpetuals_vault_authority: &'info AccountInfo<'info>,
    pub perpetuals_state: &'info AccountInfo<'info>,
    pub perpetuals_pool: &'info AccountInfo<'info>,
    pub receiving_custody: &'info AccountInfo<'info>,
    pub receiving_custody_doves_price_account: &'info AccountInfo<'info>,
    pub receiving_custody_pythnet_price_account: &'info AccountInfo<'info>,
    pub receiving_custody_token_account: InterfaceAccount<'info, TokenAccount>,
    pub dispensing_custody: &'info AccountInfo<'info>,
    pub dispensing_custody_doves_price_account: &'info AccountInfo<'info>,
    pub dispensing_custody_pythnet_price_account: &'info AccountInfo<'info>,
    pub dispensing_custody_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub event_authority: &'info AccountInfo<'info>,
    // pub perpes_program:  &'info AccountInfo<'info>,              // same to dex_program_id
}

impl<'info> PerpetualsSwapAccount<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            owner,
            funding_account,
            receiving_account,
            perpetuals_vault_authority,
            perpetuals_state,
            perpetuals_pool,
            receiving_custody,
            receiving_custody_doves_price_account,
            receiving_custody_pythnet_price_account,
            receiving_custody_token_account,
            dispensing_custody,
            dispensing_custody_doves_price_account,
            dispensing_custody_pythnet_price_account,
            dispensing_custody_token_account,
            token_program,
            event_authority,
        ]: &[AccountInfo<'info>; PERPETUALS_SWAP_ACCOUNTS_LEN] =
            array_ref![accounts, offset, PERPETUALS_SWAP_ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            owner,
            funding_account: InterfaceAccount::try_from(funding_account)?,
            receiving_account: InterfaceAccount::try_from(receiving_account)?,
            perpetuals_vault_authority,
            perpetuals_state,
            perpetuals_pool,
            receiving_custody,
            receiving_custody_doves_price_account,
            receiving_custody_pythnet_price_account,
            receiving_custody_token_account: InterfaceAccount::try_from(
                receiving_custody_token_account,
            )?,
            dispensing_custody,
            dispensing_custody_doves_price_account,
            dispensing_custody_pythnet_price_account,
            dispensing_custody_token_account: InterfaceAccount::try_from(
                dispensing_custody_token_account,
            )?,
            token_program: Program::try_from(token_program)?,
            event_authority,
        })
    }

    fn get_accountmetas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new_readonly(self.owner.key(), true),
            AccountMeta::new(self.funding_account.key(), false),
            AccountMeta::new(self.receiving_account.key(), false),
            AccountMeta::new_readonly(self.perpetuals_vault_authority.key(), false),
            AccountMeta::new_readonly(self.perpetuals_state.key(), false),
            AccountMeta::new(self.perpetuals_pool.key(), false),
            AccountMeta::new(self.receiving_custody.key(), false),
            AccountMeta::new_readonly(self.receiving_custody_doves_price_account.key(), false),
            AccountMeta::new_readonly(self.receiving_custody_pythnet_price_account.key(), false),
            AccountMeta::new(self.receiving_custody_token_account.key(), false),
            AccountMeta::new(self.dispensing_custody.key(), false),
            AccountMeta::new_readonly(self.dispensing_custody_doves_price_account.key(), false),
            AccountMeta::new_readonly(self.dispensing_custody_pythnet_price_account.key(), false),
            AccountMeta::new(self.dispensing_custody_token_account.key(), false),
            AccountMeta::new_readonly(self.token_program.key(), false),
            AccountMeta::new_readonly(self.event_authority.key(), false),
            AccountMeta::new_readonly(self.dex_program_id.key(), false),
        ]
    }

    fn get_accountinfos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.owner.to_account_info(),
            self.funding_account.to_account_info(),
            self.receiving_account.to_account_info(),
            self.perpetuals_vault_authority.to_account_info(),
            self.perpetuals_state.to_account_info(),
            self.perpetuals_pool.to_account_info(),
            self.receiving_custody.to_account_info(),
            self.receiving_custody_doves_price_account.to_account_info(),
            self.receiving_custody_pythnet_price_account.to_account_info(),
            self.receiving_custody_token_account.to_account_info(),
            self.dispensing_custody.to_account_info(),
            self.dispensing_custody_doves_price_account.to_account_info(),
            self.dispensing_custody_pythnet_price_account.to_account_info(),
            self.dispensing_custody_token_account.to_account_info(),
            self.token_program.to_account_info(),
            self.event_authority.to_account_info(),
            self.dex_program_id.to_account_info(),
        ]
    }
}

pub fn perpetuals_swap_handler<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::PerpetualsSwap amount in: {}, offset: {}", amount_in, offset);

    let mut swap_accounts = PerpetualsSwapAccount::parse_accounts(remaining_accounts, *offset)?;

    if swap_accounts.dex_program_id.key != &perpetuals_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    swap_accounts.perpetuals_pool.key.log();

    before_check(
        &swap_accounts.owner,
        &swap_accounts.funding_account,
        swap_accounts.receiving_account.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let mut ix_data = Vec::with_capacity(24);
    ix_data.extend_from_slice(PERPETUALS_SWAP_SELECTOR); //discriminator
    ix_data.extend_from_slice(&amount_in.to_le_bytes()); // amount in
    ix_data.extend_from_slice(&1u64.to_le_bytes()); // min amount out

    swap_accounts.dex_program_id.key().log();

    let instruction = Instruction {
        program_id: swap_accounts.dex_program_id.key(),
        accounts: swap_accounts.get_accountmetas(),
        data: ix_data,
    };

    let amount_out = invoke_process(
        amount_in,
        &PerpetualsProcessor,
        &swap_accounts.get_accountinfos(),
        &mut swap_accounts.funding_account,
        &mut swap_accounts.receiving_account,
        hop_accounts,
        instruction,
        hop,
        offset,
        PERPETUALS_SWAP_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;

    Ok(amount_out)
}

pub fn liquidity_handler<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    is_add_liquidity: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::PerpetualsLiquidityHandler amount in: {}, offset: {}", amount_in, offset);
    let mut handle_liquidity_accounts =
        PerpetualsAccount::parse_accounts(remaining_accounts, *offset)?;

    if handle_liquidity_accounts.dex_program_id.key != &perpetuals_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    handle_liquidity_accounts.perpetuals_pool.key.log();

    before_check(
        &handle_liquidity_accounts.owner,
        &handle_liquidity_accounts.funding_or_receiving_account,
        handle_liquidity_accounts.jlp_token_account.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    handle_liquidity_accounts.dex_program_id.key.log();

    let mut ix_data;
    if is_add_liquidity {
        ix_data = Vec::with_capacity(25);
        ix_data.extend_from_slice(PERPETUALS_ADDLIQ_SELECTOR); //discriminator
        ix_data.extend_from_slice(&amount_in.to_le_bytes()); // amount in
        ix_data.extend_from_slice(&1u64.to_le_bytes()); // minLpAmountOut
        ix_data.push(0); // tokenAmountPreSwap

        let instruction = Instruction {
            program_id: handle_liquidity_accounts.dex_program_id.key(),
            accounts: handle_liquidity_accounts.get_accountmetas(),
            data: ix_data,
        };

        invoke_process(
            amount_in,
            &PerpetualsProcessor,
            &handle_liquidity_accounts.get_accountinfos(),
            &mut handle_liquidity_accounts.funding_or_receiving_account,
            &mut handle_liquidity_accounts.jlp_token_account,
            hop_accounts,
            instruction,
            hop,
            offset,
            PERPETUALS_LIQUIDITY_ACCOUNTS_LEN + PEEPETUALS_REMAINING_ACCOUNTS_LEN,
            proxy_swap,
            owner_seeds,
        )
    } else {
        ix_data = Vec::with_capacity(24);
        ix_data.extend_from_slice(PERPETUALS_REMOVELIQ_SELECTOR); //discriminator
        ix_data.extend_from_slice(&amount_in.to_le_bytes()); // lp amount in
        ix_data.extend_from_slice(&1u64.to_le_bytes()); // minAmountOut

        let instruction = Instruction {
            program_id: handle_liquidity_accounts.dex_program_id.key(),
            accounts: handle_liquidity_accounts.get_accountmetas(),
            data: ix_data,
        };

        invoke_process(
            amount_in,
            &PerpetualsProcessor,
            &handle_liquidity_accounts.get_accountinfos(),
            &mut handle_liquidity_accounts.jlp_token_account,
            &mut handle_liquidity_accounts.funding_or_receiving_account,
            hop_accounts,
            instruction,
            hop,
            offset,
            PERPETUALS_LIQUIDITY_ACCOUNTS_LEN + PEEPETUALS_REMAINING_ACCOUNTS_LEN,
            proxy_swap,
            owner_seeds,
        )
    }
}
