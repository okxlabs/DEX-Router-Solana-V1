use anchor_lang::prelude::*;

#[constant]
pub const SEED_SA: &[u8] = b"okx_sa";
pub const SEED_TEMP_WSOL: &[u8] = b"temp_wsol";
pub const BUMP_SA: u8 = 251;

pub const COMMISSION_RATE_LIMIT: u16 = 1_000; // 10%
pub const COMMISSION_DENOMINATOR: u64 = 10_000;

pub const COMMISSION_RATE_LIMIT_V2: u32 = 100_000_000; // 10%
pub const COMMISSION_DENOMINATOR_V2: u64 = 1_000_000_000;

pub const PLATFORM_FEE_RATE_LIMIT_V2: u64 = 1_000_000_000; // 100%
pub const PLATFORM_FEE_DENOMINATOR_V2: u64 = 1_000_000_000;

pub const TRIM_RATE_LIMIT_V2: u8 = 100; // 10%
pub const TRIM_DENOMINATOR_V2: u16 = 1_000;

pub const PLATFORM_FEE_RATE_LIMIT_V3: u64 = 10_000; // 100%
pub const PLATFORM_FEE_DENOMINATOR_V3: u64 = 10_000;

pub const MAX_HOPS: usize = 3;
pub const TOTAL_WEIGHT: u8 = 100;
pub const SA_AUTHORITY_SEED: &[&[&[u8]]] = &[&[SEED_SA, &[BUMP_SA]]];
pub const TOKEN_ACCOUNT_RENT: u64 = 2039280; // Token account rent (165 bytes)
pub const MIN_SOL_ACCOUNT_RENT: u64 = 890880;
pub const SOL_DIFF_LIMIT: u64 = 8_100_000;

// Actual amount_in lower bound ratio for post swap check
pub const ACTUAL_IN_LOWER_BOUND_NUM: u128 = 95; // 95%
pub const ACTUAL_IN_LOWER_BOUND_DEN: u128 = 100; // denominator for percentage

pub const SWAP_SELECTOR: &[u8; 8] = &[248, 198, 158, 145, 225, 117, 135, 200];
pub const SWAP2_SELECTOR: &[u8; 8] = &[65, 75, 63, 76, 235, 91, 91, 136];
pub const CPSWAP_SELECTOR: &[u8; 8] = &[143, 190, 90, 218, 196, 30, 51, 222];
pub const SWAP_V2_SELECTOR: &[u8; 8] = &[43, 4, 237, 11, 26, 201, 30, 98];
pub const SWAP_EXACT_IN_SELECTOR: &[u8; 8] = &[104, 104, 131, 86, 161, 189, 180, 216];
pub const PLACE_TAKE_ORDER_SELECTOR: &[u8; 8] = &[3, 44, 71, 3, 26, 199, 203, 85];
pub const BRIDGE_TO_LOG_SELECTOR: &[u8; 8] = &[212, 189, 176, 218, 196, 135, 64, 122];
pub const ZERO_ADDRESS: Pubkey = Pubkey::new_from_array([0u8; 32]);
pub const HEAVEN_BUY_SELECTOR: &[u8; 8] = &[102, 6, 61, 18, 1, 218, 235, 234];
pub const HEAVEN_SELL_SELECTOR: &[u8; 8] = &[51, 230, 133, 164, 1, 127, 131, 173];

pub const PUMPFUN_BUY_SELECTOR: &[u8; 8] = &[102, 6, 61, 18, 1, 218, 235, 234];
pub const PUMPFUN_SELL_SELECTOR: &[u8; 8] = &[51, 230, 133, 164, 1, 127, 131, 173];
pub const MOONIT_BUY_SELECTOR: &[u8; 8] = &[102, 6, 61, 18, 1, 218, 235, 234];
pub const MOONIT_SELL_SELECTOR: &[u8; 8] = &[51, 230, 133, 164, 1, 127, 131, 173];
pub const STABBLE_SWAP_SELECTOR: &[u8; 8] = &[43, 4, 237, 11, 26, 201, 30, 98];
pub const DEPOSIT_SELECTOR: &[u8; 8] = &[242, 35, 198, 137, 82, 225, 242, 182];
pub const WITHDRAW_SELECTOR: &[u8; 8] = &[183, 18, 70, 156, 148, 109, 161, 34];

pub const WOOFI_SWAP_SELECTOR: &[u8; 8] = &[248, 198, 158, 145, 225, 117, 135, 200];
pub const VIRTUALS_BUY_SELECTOR: &[u8; 8] = &[102, 6, 61, 18, 1, 218, 235, 234];
pub const VIRTUALS_SELL_SELECTOR: &[u8; 8] = &[51, 230, 133, 164, 1, 127, 131, 173];
pub const PERPETUALS_ADDLIQ_SELECTOR: &[u8; 8] = &[0xe4, 0xa2, 0x4e, 0x1c, 0x46, 0xdb, 0x74, 0x73];
pub const PERPETUALS_REMOVELIQ_SELECTOR: &[u8; 8] =
    &[0xe6, 0xd7, 0x52, 0x7f, 0xf1, 0x65, 0xe3, 0x92];
pub const PERPETUALS_SWAP_SELECTOR: &[u8; 8] = &[0x41, 0x4b, 0x3f, 0x4c, 0xeb, 0x5b, 0x5b, 0x88];
pub const BUY_EXACT_IN_SELECTOR: &[u8; 8] = &[250, 234, 13, 123, 213, 156, 19, 236];
pub const SELL_EXACT_IN_SELECTOR: &[u8; 8] = &[149, 39, 222, 155, 211, 124, 152, 26];
pub const VERTIGO_BUY_SELECTOR: &[u8; 8] = &[102, 6, 61, 18, 1, 218, 235, 234];
pub const VERTIGO_SELL_SELECTOR: &[u8; 8] = &[51, 230, 133, 164, 1, 127, 131, 173];
pub const BOOPFUN_BUY_SELECTOR: &[u8; 8] = &[138, 127, 14, 91, 38, 87, 115, 105];
pub const BOOPFUN_SELL_SELECTOR: &[u8; 8] = &[109, 61, 40, 187, 230, 176, 135, 174];
pub const GAMMA_ORACLE_SWAP_SELECTOR: &[u8; 8] = &[239, 82, 192, 187, 160, 26, 223, 223];
pub const SABER_DECIMAL_DEPOSIT_SELECTOR: &[u8; 8] =
    &[0xf2, 0x23, 0xc6, 0x89, 0x52, 0xe1, 0xf2, 0xb6];
pub const SABER_DECIMAL_WITHDRAW_SELECTOR: &[u8; 8] =
    &[0xb7, 0x12, 0x46, 0x9c, 0x94, 0x6d, 0xa1, 0x22];
pub const ONE_DEX_SWAP_SELECTOR: &[u8; 8] = &[8, 151, 245, 76, 172, 203, 144, 39];
pub const MANIFEST_SWAP_SELECTOR: &[u8; 1] = &[4];
pub const TESSERA_SWAP_SELECTOR: &[u8; 1] = &[16];
pub const SOL_RFQ_FILL_ORDER_SELECTOR: &[u8; 8] = &[232, 122, 115, 25, 199, 143, 136, 162];
pub const GOONFI_SWAP_SELECTOR: &[u8; 1] = &[2];
pub const SPOT_SWAP_SELECTOR: &[u8; 8] = &[167, 97, 12, 231, 237, 78, 166, 251];

pub const HUMIDIFI_SWAP_SELECTOR: u8 = 0x4;
const HUMIDIFI_IX_DATA_KEY_SEED: [u8; 32] = [
    58, 255, 47, 255, 226, 186, 235, 195, 123, 131, 245, 8, 11, 233, 132, 219, 225, 40, 79, 119,
    169, 121, 169, 58, 197, 1, 122, 9, 216, 164, 149, 97,
];
pub const HUMIDIFI_IX_DATA_KEY: u64 = u64::from_le_bytes([
    HUMIDIFI_IX_DATA_KEY_SEED[0],
    HUMIDIFI_IX_DATA_KEY_SEED[1],
    HUMIDIFI_IX_DATA_KEY_SEED[2],
    HUMIDIFI_IX_DATA_KEY_SEED[3],
    HUMIDIFI_IX_DATA_KEY_SEED[4],
    HUMIDIFI_IX_DATA_KEY_SEED[5],
    HUMIDIFI_IX_DATA_KEY_SEED[6],
    HUMIDIFI_IX_DATA_KEY_SEED[7],
]);
pub const ALPHAQ_SWAP_SELECTOR: &[u8; 1] = &[0x0c];

pub const BUY_EXACT_SOL_IN_SELECTOR: &[u8; 8] = &[56, 252, 116, 8, 158, 223, 205, 95]; // 38 fc 74 08 9e df cd 5f
pub const BUY_EXACT_QUOTE_IN_SELECTOR: &[u8; 8] = &[198, 46, 21, 82, 180, 217, 232, 112]; // c6 2e 15 52 b4 d9 e8 70

// ******************** Limit Order ******************** //
pub const GLOBAL_CONFIG_SEED: &str = "global_config";
pub const ORDER_V1_SEED: &str = "order_v1";
pub const ESCROW_TOKEN_SEED: &str = "escrow_token";
pub const MIN_DEADLINE: u64 = 300; //min order deadline: 5 minutes
pub const SIGNATURE_FEE: u64 = 5000;
pub const ORDER_MIN_RENT: u64 = 3563520; //needs to be changed when order account size is changed
pub const DEFAULT_COMPUTE_UNIT_LIMIT: u32 = 200_000;
pub const FEE_MULTIPLIER_DENOMINATOR: u64 = 10;

#[cfg(feature = "staging")]
pub mod authority_pda {
    use anchor_lang::declare_id;
    declare_id!("4DwLmWvMyWPPKa8jhmW6AZKGctUMe7GxAWrb2Wcw8ZUa"); //pre_deploy
}

#[cfg(not(feature = "staging"))]
pub mod authority_pda {
    use anchor_lang::declare_id;
    declare_id!("HV1KXxWFaSeriyFvXyx48FqG9BoFbfinB8njCJonqP7K");
}

#[cfg(feature = "staging")]
pub mod okx_bridge_program {
    use anchor_lang::declare_id;
    declare_id!("preMfqJcNX4xdbGz7LGaNiUj9Ej5Qg2a4ymfbuG5R5k"); //pre_deploy
}

#[cfg(not(feature = "staging"))]
pub mod okx_bridge_program {
    use anchor_lang::declare_id;
    declare_id!("okxBd18urPbBi2vsExxUDArzQNcju2DugV9Mt46BxYE");
}

#[cfg(feature = "staging")]
pub mod wsol_sa {
    use anchor_lang::declare_id;
    declare_id!("5RWt14cufVyp4URS5hfoCczqSATxFH4AW6XAN8yyJtTg"); //pre_deploy
}

#[cfg(not(feature = "staging"))]
pub mod wsol_sa {
    use anchor_lang::declare_id;
    declare_id!("2rikd7tzPbmowhUJzPNVtX7fuUGcnBa8jqJnx6HbtHeE");
}

#[cfg(feature = "staging")]
pub mod sol_rfq_program {
    use anchor_lang::declare_id;
    declare_id!("preNqJotnzt2tUaeGX4FsQEU3dUsopsraZHVwNUwUAZ"); //pre_deploy
}

#[cfg(not(feature = "staging"))]
pub mod sol_rfq_program {
    use anchor_lang::declare_id;
    declare_id!("RFQ1uATMXfRXemLnbYCF8JZhVfELp2K53jSEAGbsAKX");
}

pub mod claim_authority {
    use anchor_lang::declare_id;
    declare_id!("CjoV5B96reuCfPh2rRK11G1QptG97jZdyZArTn3EN1Mj");
}

pub mod compute_budget_program {
    use anchor_lang::declare_id;
    declare_id!("ComputeBudget111111111111111111111111111111");
}

pub mod token_program {
    use anchor_lang::declare_id;
    declare_id!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
}

pub mod token_2022_program {
    use anchor_lang::declare_id;
    declare_id!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
}

pub mod wsol_program {
    use anchor_lang::declare_id;
    declare_id!("So11111111111111111111111111111111111111112");
}

//system program address, backend uses this address to represent native sol
pub mod system_program {
    use anchor_lang::declare_id;
    declare_id!("11111111111111111111111111111111");
}

// ******************** dex program ids ******************** //

pub mod spl_token_swap_program {
    use anchor_lang::declare_id;
    declare_id!("SwaPpA9LAaLfeLi3a68M4DjnLqgtticKg6CnyNwgAC8");
}
pub mod orca_swap_program {
    use anchor_lang::declare_id;
    declare_id!("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP");
}

pub mod one_moon_swap_program {
    use anchor_lang::declare_id;
    declare_id!("1MooN32fuBBgApc8ujknKJw5sef3BVwPGgz3pto1BAh");
}

pub mod step_swap_program {
    use anchor_lang::declare_id;
    declare_id!("SSwpMgqNDsyV7mAgN9ady4bDVu5ySjmmXejXvy2vLt1");
}

pub mod saber_stable_program {
    use anchor_lang::declare_id;
    declare_id!("SSwpkEEcbUqx4vtoEByFjSkhKdCT862DNVb52nZg1UZ");
}

pub mod raydium_swap_program {
    use anchor_lang::declare_id;
    declare_id!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
}

pub mod raydium_stable_program {
    use anchor_lang::declare_id;
    declare_id!("5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h");
}

pub mod raydium_clmm_program {
    use anchor_lang::declare_id;
    declare_id!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
}

pub mod raydium_cpmm_program {
    use anchor_lang::declare_id;
    declare_id!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
}

pub mod aldrin_v1_program {
    use anchor_lang::declare_id;
    declare_id!("AMM55ShdkoGRB5jVYPjWziwk8m5MpwyDgsMWHaMSQWH6");
}

pub mod aldrin_v2_program {
    use anchor_lang::declare_id;
    declare_id!("CURVGoZn8zycx6FXwwevgBTB2gVvdbGTEpvMJDbgs2t4");
}

pub mod whirlpool_program {
    use anchor_lang::declare_id;
    declare_id!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
}

pub mod meteora_dynamicpool_program {
    use anchor_lang::declare_id;
    declare_id!("Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB");
}

pub mod meteora_dlmm_program {
    use anchor_lang::declare_id;
    declare_id!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo");
}

pub mod meteora_damm_v2_program {
    use anchor_lang::declare_id;
    declare_id!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");
}

pub mod lifinity_v1pool_program {
    use anchor_lang::declare_id;
    declare_id!("EewxydAPCCVuNEyrVN68PuSYdQ7wKn27V9Gjeoi8dy3S");
}

pub mod lifinity_v2pool_program {
    use anchor_lang::declare_id;
    declare_id!("2wT8Yq49kHgDzXuPxZSaeLaH1qbmGXtEyPy64bL7aD3c");
}

pub mod flux_beam_program {
    use anchor_lang::declare_id;
    declare_id!("FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X");
}

pub mod openbookv2_program {
    use anchor_lang::declare_id;
    declare_id!("opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb");
}

pub mod phoenix_program {
    use anchor_lang::declare_id;
    declare_id!("PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY");
}

pub mod obric_v2_program {
    use anchor_lang::declare_id;
    declare_id!("obriQD1zbpyLz95G5n7nJe6a4DPjpFwa5XYPoNm113y");
}

pub mod sanctum_program {
    use anchor_lang::declare_id;
    declare_id!("5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx");
}

pub mod pumpfun_program {
    use anchor_lang::declare_id;
    declare_id!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
}

pub mod saros_program {
    use anchor_lang::declare_id;
    declare_id!("SSwapUtytfBdBn1b9NUGG6foMVPtcWgpRU32HToDUZr");
}

pub mod saros_dlmm_program {
    use anchor_lang::declare_id;
    declare_id!("1qbkdrr3z4ryLA7pZykqxvxWPoeifcVKo6ZG9CfkvVE");
}
pub mod stabble_stable_program {
    use anchor_lang::declare_id;
    declare_id!("swapNyd8XiQwJ6ianp9snpu4brUqFxadzvHebnAXjJZ");
}
pub mod stabble_weighted_program {
    use anchor_lang::declare_id;
    declare_id!("swapFpHZwjELNnjvThjajtiVmkz3yPQEHjLtka2fwHW");
}
pub mod sanctum_router_program {
    use anchor_lang::declare_id;
    declare_id!("stkitrT1Uoy18Dk1fTrgPw8W6MVzoCfYoAFT4MLsmhq");
}

pub mod lido_sol_mint {
    use anchor_lang::declare_id;
    declare_id!("7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj");
}

pub mod marinade_sol_mint {
    use anchor_lang::declare_id;
    declare_id!("mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So");
}

pub mod meteora_vault_program {
    use anchor_lang::declare_id;
    declare_id!("24Uqj9JCLxUeoC3hGfh5W3s9FM9uCHDS2SG3LYwBpyTi");
}

pub mod solfi_program {
    use anchor_lang::declare_id;
    declare_id!("SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe");
}

pub mod solfi_v2_program {
    use anchor_lang::declare_id;
    declare_id!("SV2EYYJyRz2YhfXwXnhNAevDEui5Q6yrfyo13WtupPF");
}

pub mod qualia_program {
    use anchor_lang::declare_id;
    declare_id!("RBCNJvXMmrcSbX6Tc9dYySLR13Vs6kjVUVXK6qJ4Lf4");
}

pub mod zerofi_program {
    use anchor_lang::declare_id;
    declare_id!("ZERor4xhbUycZ6gb9ntrhqscUcZmAbQDjEAtCf4hbZY");
}

pub mod pumpfunamm_program {
    use anchor_lang::declare_id;
    declare_id!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");
}

pub mod virtuals_program {
    use anchor_lang::declare_id;
    declare_id!("5U3EU2ubXtK84QcRjWVmYt9RaDyA8gKxdUrPFXmZyaki");
}

pub mod virtual_token_mint {
    use anchor_lang::declare_id;
    declare_id!("3iQL8BFS2vE7mww4ehAqQHAsbmRNCrPxizWAT2Zfyr9y");
}

pub mod price_update_solusd {
    use anchor_lang::declare_id;
    declare_id!("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE");
}

pub mod vertigo_program {
    use anchor_lang::declare_id;
    declare_id!("vrTGoBuy5rYSxAfV3jaRJWHH6nN9WK4NRExGxsk1bCJ"); // mainnet
    // declare_id!("AVY2KtGjkHE4J93A1y9Ds6RekC3kTKasgazEeDXB4DXX");     // devnet
}

pub mod perpetuals_program {
    use anchor_lang::declare_id;
    declare_id!("PERPHjGBqRHArX4DySjwM6UJHiR3sWAatqfdBS2qQJu");
}

pub mod raydium_launchpad_program {
    use anchor_lang::declare_id;
    declare_id!("LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj");
}

pub mod woofi_program {
    use anchor_lang::declare_id;
    declare_id!("WooFif76YGRNjk1pA8wCsN67aQsD9f9iLsz4NcJ1AVb");
}

pub mod meteora_dbc_program {
    use anchor_lang::declare_id;
    declare_id!("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");
}

pub mod gavel_program {
    use anchor_lang::declare_id;
    declare_id!("srAMMzfVHVAtgSJc8iH6CfKzuWuUTzLHVCE81QU1rgi");
}

pub mod boopfun_program {
    use anchor_lang::declare_id;
    declare_id!("boop8hVGQGqehUK2iVEMEnMrL5RbjywRzHKBmBE7ry4");
}

pub mod goosefx_gamma_program {
    use anchor_lang::declare_id;
    declare_id!("GAMMA7meSFWaBXF25oSUgmGRwaW6sCMFLmBNiMSdbHVT");
}

pub mod dooar_program {
    use anchor_lang::declare_id;
    declare_id!("Dooar9JkhdZ7J3LHN3A7YCuoGRUggXhQaG4kijfLGU2j");
}

pub mod numeraire_program {
    use anchor_lang::declare_id;
    declare_id!("NUMERUNsFCP3kuNmWZuXtm1AaQCPj9uw6Guv2Ekoi5P");
}

pub mod numeraire_usdstar_mint {
    use anchor_lang::declare_id;
    declare_id!("BenJy1n3WTx9mTjEvy63e8Q1j4RqUc6E4VBMz3ir4Wo6");
}

pub mod saber_decimal_wrapper_program {
    use anchor_lang::declare_id;
    declare_id!("DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB");
}

pub mod one_dex_program {
    use anchor_lang::declare_id;
    declare_id!("DEXYosS6oEGvk8uCDayvwEZz4qEyDJRf9nFgYCaqPMTm");
}

pub mod manifest_program {
    use anchor_lang::declare_id;
    // Manifest orderbook DEX program - supports limit and market orders
    declare_id!("MNFSTqtC93rEfYHB6hF82sKdZpUDFWkViLByLd1k1Ms");
}

pub mod byreal_clmm_program {
    use anchor_lang::declare_id;
    declare_id!("REALQqNEomY6cQGZJUGwywTBD2UmDT32rZcNnfxQ5N2");
}

pub mod pancake_swap_v3_program {
    use anchor_lang::declare_id;
    declare_id!("HpNfyc2Saw7RKkQd8nEL4khUcuPhQ7WwY1B2qjx8jxFq");
}

pub mod tessera_program {
    use anchor_lang::declare_id;
    declare_id!("TessVdML9pBGgG9yGks7o4HewRaXVAMuoVj4x83GLQH");
}

pub mod humidifi_program {
    use anchor_lang::declare_id;
    declare_id!("9H6tua7jkLhdm3w8BvgpTn5LZNU7g4ZynDmCiNN3q6Rp");
}

pub mod heaven_program {
    use anchor_lang::declare_id;
    declare_id!("HEAVENoP2qxoeuF8Dj2oT1GHEnu49U5mJYkdeC8BAX2o");
}

pub mod goonfi_program {
    use anchor_lang::declare_id;
    declare_id!("goonERTdGsjnkZqWuVjs73BZ3Pb9qoCUdBUL17BnS5j");
}
pub mod moonit_program {
    use anchor_lang::declare_id;
    declare_id!("MoonCVVNZFSYkqNXP6bxHLPL6QQJiMagDL3qcqUQTrG");
}

pub mod whalestreet_program {
    use anchor_lang::declare_id;
    declare_id!("5FyWAoG8V6hxgY6XM9hZStNxSW4D6mkv8HmYrxuPPDhv");
}

pub mod sugar_money_program {
    use anchor_lang::declare_id;
    declare_id!("deus4Bvftd5QKcEkE5muQaWGWDoma8GrySvPFrBPjhS");
}

pub mod alphaq_program {
    use anchor_lang::declare_id;
    declare_id!("ALPHAQmeA7bjrVuccPsYPiCvsi428SNwte66Srvs4pHA");
}

pub mod futarchy_amm_program {
    use anchor_lang::declare_id;
    declare_id!("FUTARELBfJfQ8RDGhg1wdhddq1odMAJUePHFuBYfUxKq");
}
