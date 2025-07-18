use crate::error::LimitOrderError;
use anchor_lang::prelude::*;

#[account(zero_copy(unsafe))]
#[derive(Debug)]
pub struct GlobalConfig {
    /// Bump to identify PDA.
    pub bump: u8,

    /// The admin of the program.
    pub admin: Pubkey,

    /// Only the resolver can trigger order filled.
    pub resolvers: [Pubkey; 5],

    /// Prepaid trade fee, the remaining amount will be refunded to the user
    pub trade_fee: u64,

    /// Indicate whether to pause trading.
    pub paused: bool,

    /// Fee multiplier for the trade fee
    pub fee_multiplier: u8,

    /// padding for upgrade
    pub padding: [u8; 127],
}

impl Default for GlobalConfig {
    fn default() -> Self {
        GlobalConfig {
            bump: 0,
            admin: Pubkey::default(),
            resolvers: [Pubkey::default(); 5],
            trade_fee: 0,
            paused: false,
            fee_multiplier: 10,
            padding: [0u8; 127],
        }
    }
}

impl GlobalConfig {
    pub const LEN: usize = 8 + std::mem::size_of::<GlobalConfig>();

    pub fn set_admin(&mut self, admin: Pubkey) -> Result<()> {
        require_keys_neq!(admin, Pubkey::default(), LimitOrderError::InvalidAccount);
        self.admin = admin;
        Ok(())
    }

    pub fn add_resolver(&mut self, resolver: Pubkey) -> Result<()> {
        require_keys_neq!(resolver, Pubkey::default(), LimitOrderError::InvalidAccount);
        for item in self.resolvers {
            require_keys_neq!(resolver, item, LimitOrderError::ResolverIsExist);
        }
        for item in &mut self.resolvers {
            if *item == Pubkey::default() {
                *item = resolver;
                return Ok(());
            }
        }
        return Err(LimitOrderError::ExceedResolverLimit.into());
    }

    pub fn remove_resolver(&mut self, resolver: Pubkey) -> Result<()> {
        require_keys_neq!(resolver, Pubkey::default(), LimitOrderError::InvalidAccount);
        for item in &mut self.resolvers {
            if *item == resolver {
                *item = Pubkey::default();
                return Ok(());
            }
        }
        return Err(LimitOrderError::ResolverIsNotExist.into());
    }

    pub fn is_resolver(&self, resolver: Pubkey) -> bool {
        if resolver == Pubkey::default() {
            return false;
        }
        for item in &self.resolvers {
            if *item == resolver {
                return true;
            }
        }
        return false;
    }

    pub fn set_trade_fee(&mut self, trade_fee: u64) -> Result<()> {
        require!(trade_fee > 0, LimitOrderError::InvalidTradeFee);
        self.trade_fee = trade_fee;
        Ok(())
    }

    pub fn set_paused(&mut self, paused: bool) -> Result<()> {
        self.paused = paused;
        Ok(())
    }

    pub fn set_fee_multiplier(&mut self, fee_multiplier: u8) -> Result<()> {
        require!(fee_multiplier >= 10, LimitOrderError::InvalidFeeMultiplier);
        self.fee_multiplier = fee_multiplier;
        Ok(())
    }
}
