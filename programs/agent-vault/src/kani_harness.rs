use crate::{
    agent_account::{
        parse_agent_account, AGENT_ACCOUNT_ASSET_OFFSET, AGENT_ACCOUNT_BUMP_OFFSET,
        AGENT_ACCOUNT_COLLECTION_OFFSET, AGENT_ACCOUNT_CREATOR_OFFSET, AGENT_ACCOUNT_DISCRIMINATOR,
        AGENT_ACCOUNT_MIN_LEN, AGENT_ACCOUNT_OWNER_OFFSET,
    },
    constants::{
        EXPECTED_ACTIVATION_FEE_LAMPORTS, LABEL_LEN, MAX_CPI_ACCOUNTS, MAX_CPI_IX_DATA_LEN,
        MAX_POST_CHECKS, MAX_WALLETS, VERSION_V0, WALLET_FLAG_ACTIVE, WALLET_FLAG_RECOVERY_ONLY,
    },
    core_asset::{
        parse_core_asset, CORE_ASSET_COLLECTION_OFFSET, CORE_ASSET_COLLECTION_TAG,
        CORE_ASSET_COLLECTION_TAG_OFFSET, CORE_ASSET_KEY_OFFSET, CORE_ASSET_MIN_LEN,
        CORE_ASSET_OWNER_OFFSET, CORE_ASSET_V1_KEY,
    },
    cpi_plan::{
        final_account_meta_at, final_cpi_account_count, planned_account_at,
        validate_duplicate_policy, validate_execute_cpi_plan, CpiAccountMeta, PlannedAccount,
        ProtectedAccounts,
    },
    instruction::{
        parse_instruction, parse_post_check, validate_label, validate_post_checks,
        ExecuteCpiChecked, Instruction, PostCheck, TokenProgramKind, TAG_CLOSE_WALLET,
        TAG_CLOSE_WALLET_ATA, TAG_CREATE_WALLET, TAG_CREATE_WALLET_ATA, TAG_DEPOSIT_SOL,
        TAG_EXECUTE_CPI_CHECKED, TAG_INITIALIZE_GLOBAL_CONFIG, TAG_INIT_VAULT_CONFIG,
        TAG_REOPEN_WALLET_FOR_RECOVERY, TAG_TRANSFER_SOL, TAG_TRANSFER_SPL, TAG_UNWRAP_SOL,
        TAG_UPDATE_WALLET_LABEL, TAG_WITHDRAW_SOL, TAG_WRAP_SOL,
    },
    pda::agent_wallet_index_seed,
    processor::checked_lamport_move_result,
    state::{
        pack_global_config, pack_vault_config, pack_wallet, unpack_global_config,
        unpack_vault_config, unpack_wallet, AgentWallet, GlobalConfig, VaultConfig,
        GLOBAL_CONFIG_LEN, VAULT_CONFIG_LEN, WALLET_LEN,
    },
    token_state::{TransferFee, MAX_FEE_BASIS_POINTS},
};

fn any_key() -> [u8; 32] {
    kani::any()
}

fn distinct_key(seed: u8) -> [u8; 32] {
    [seed; 32]
}

#[kani::proof]
fn global_config_pack_unpack_roundtrip() {
    let config = GlobalConfig {
        bump: kani::any(),
        initializer: any_key(),
        registry_program: any_key(),
        collection: any_key(),
        fee_treasury: any_key(),
        vault_activation_fee_lamports: kani::any(),
    };
    let mut data = [0u8; GLOBAL_CONFIG_LEN];

    assert!(pack_global_config(&config, &mut data).is_ok());
    assert_eq!(unpack_global_config(&data), Ok(config));
}

#[kani::proof]
fn vault_config_pack_unpack_roundtrip_with_reserved_v0_flags() {
    let config = VaultConfig {
        bump: kani::any(),
        wallet_count: kani::any(),
        flags: 0,
        created_at: kani::any(),
    };
    let mut data = [0u8; VAULT_CONFIG_LEN];

    assert!(pack_vault_config(&config, &mut data).is_ok());
    assert_eq!(unpack_vault_config(&data), Ok(config));
}

#[kani::proof]
fn wallet_pack_unpack_roundtrip_for_valid_v0_flags() {
    let recovery_only: bool = kani::any();
    let flags = if recovery_only {
        WALLET_FLAG_RECOVERY_ONLY
    } else {
        WALLET_FLAG_ACTIVE
    };
    let wallet = AgentWallet {
        bump: kani::any(),
        index: kani::any(),
        flags,
        label: kani::any(),
    };
    let mut data = [0u8; WALLET_LEN];

    assert!(pack_wallet(&wallet, &mut data).is_ok());
    assert_eq!(unpack_wallet(&data), Ok(wallet));
}

#[kani::proof]
fn wallet_unpack_rejects_invalid_flag_combinations() {
    let mut data = [0u8; WALLET_LEN];
    let both_flags = WALLET_FLAG_ACTIVE | WALLET_FLAG_RECOVERY_ONLY;
    let wallet = AgentWallet {
        bump: kani::any(),
        index: kani::any(),
        flags: both_flags,
        label: kani::any(),
    };

    assert!(pack_wallet(&wallet, &mut data).is_ok());
    assert!(unpack_wallet(&data).is_err());
}

#[kani::proof]
fn v0_constants_match_spec_limits_and_tags() {
    assert_eq!(VERSION_V0, 0);
    assert_eq!(MAX_WALLETS, u16::MAX);
    assert_eq!(LABEL_LEN, 16);
    assert_eq!(MAX_CPI_ACCOUNTS, 64);
    assert_eq!(MAX_CPI_IX_DATA_LEN, 1024);
    assert_eq!(MAX_POST_CHECKS, 8);
    assert_eq!(EXPECTED_ACTIVATION_FEE_LAMPORTS, 500_000);

    assert_eq!(TAG_INITIALIZE_GLOBAL_CONFIG, 0);
    assert_eq!(TAG_INIT_VAULT_CONFIG, 1);
    assert_eq!(TAG_CREATE_WALLET, 2);
    assert_eq!(TAG_UPDATE_WALLET_LABEL, 3);
    assert_eq!(TAG_DEPOSIT_SOL, 4);
    assert_eq!(TAG_WITHDRAW_SOL, 5);
    assert_eq!(TAG_TRANSFER_SOL, 6);
    assert_eq!(TAG_CLOSE_WALLET, 7);
    assert_eq!(TAG_REOPEN_WALLET_FOR_RECOVERY, 8);
    assert_eq!(TAG_CREATE_WALLET_ATA, 32);
    assert_eq!(TAG_TRANSFER_SPL, 33);
    assert_eq!(TAG_WRAP_SOL, 34);
    assert_eq!(TAG_UNWRAP_SOL, 35);
    assert_eq!(TAG_CLOSE_WALLET_ATA, 36);
    assert_eq!(TAG_EXECUTE_CPI_CHECKED, 64);
}

#[kani::proof]
fn reserved_instruction_discriminators_reject_in_v0() {
    let tag: u8 = kani::any();
    kani::assume(tag >= 65);
    kani::assume(tag <= 127);
    let data = [tag];

    assert!(parse_instruction(&data).is_err());
}

#[kani::proof]
fn agent_wallet_index_seed_is_u16_little_endian() {
    let index: u16 = kani::any();

    assert_eq!(agent_wallet_index_seed(index), index.to_le_bytes());
}

#[kani::proof]
fn vault_config_unpack_rejects_nonzero_reserved_flags() {
    let flags: u16 = kani::any();
    kani::assume(flags != 0);
    let config = VaultConfig {
        bump: kani::any(),
        wallet_count: kani::any(),
        flags,
        created_at: kani::any(),
    };
    let mut data = [0u8; VAULT_CONFIG_LEN];

    assert!(pack_vault_config(&config, &mut data).is_ok());
    assert!(unpack_vault_config(&data).is_err());
}

#[kani::proof]
fn core_asset_parser_uses_spec_offsets() {
    let owner = any_key();
    let collection = any_key();
    let mut data = [0u8; CORE_ASSET_MIN_LEN];
    data[CORE_ASSET_KEY_OFFSET] = CORE_ASSET_V1_KEY;
    data[CORE_ASSET_COLLECTION_TAG_OFFSET] = CORE_ASSET_COLLECTION_TAG;
    data[CORE_ASSET_OWNER_OFFSET..CORE_ASSET_OWNER_OFFSET + 32].copy_from_slice(&owner);
    data[CORE_ASSET_COLLECTION_OFFSET..CORE_ASSET_COLLECTION_OFFSET + 32]
        .copy_from_slice(&collection);

    let parsed = parse_core_asset(&data).unwrap();
    assert_eq!(parsed.owner, owner);
    assert_eq!(parsed.collection, collection);
}

#[kani::proof]
fn agent_account_parser_uses_8004_header_offsets() {
    let collection = any_key();
    let creator = any_key();
    let owner = any_key();
    let asset = any_key();
    let bump: u8 = kani::any();
    let mut data = [0u8; AGENT_ACCOUNT_MIN_LEN];
    data[0..8].copy_from_slice(&AGENT_ACCOUNT_DISCRIMINATOR);
    data[AGENT_ACCOUNT_COLLECTION_OFFSET..AGENT_ACCOUNT_COLLECTION_OFFSET + 32]
        .copy_from_slice(&collection);
    data[AGENT_ACCOUNT_CREATOR_OFFSET..AGENT_ACCOUNT_CREATOR_OFFSET + 32].copy_from_slice(&creator);
    data[AGENT_ACCOUNT_OWNER_OFFSET..AGENT_ACCOUNT_OWNER_OFFSET + 32].copy_from_slice(&owner);
    data[AGENT_ACCOUNT_ASSET_OFFSET..AGENT_ACCOUNT_ASSET_OFFSET + 32].copy_from_slice(&asset);
    data[AGENT_ACCOUNT_BUMP_OFFSET] = bump;

    let parsed = parse_agent_account(&data).unwrap();
    assert_eq!(parsed.collection, collection);
    assert_eq!(parsed.creator, creator);
    assert_eq!(parsed.owner, owner);
    assert_eq!(parsed.asset, asset);
    assert_eq!(parsed.bump, bump);
}

#[kani::proof]
fn empty_label_is_valid() {
    let label = [0u8; LABEL_LEN];

    assert!(validate_label(&label).is_ok());
}

#[kani::proof]
fn label_rejects_nonzero_suffix_after_nul() {
    let mut label = [0u8; LABEL_LEN];
    label[0] = 0;
    label[1] = 1;

    assert!(validate_label(&label).is_err());
}

#[kani::proof]
fn sol_balance_post_check_parser_is_total_for_valid_payloads() {
    let account_index: u8 = kani::any();
    let lamports: u64 = kani::any();
    let mut data = [0u8; 10];
    data[0] = 0;
    data[1] = account_index;
    data[2..10].copy_from_slice(&lamports.to_le_bytes());

    assert_eq!(
        parse_post_check(&data),
        Ok((
            PostCheck::SolBalanceMin {
                account_index,
                min_lamports: lamports,
            },
            10,
        ))
    );
}

#[kani::proof]
fn post_check_tags_have_spec_serialized_sizes() {
    let mut sol = [0u8; 10];
    let mut tag = 0u8;
    while tag <= 3 {
        sol[0] = tag;
        assert_eq!(parse_post_check(&sol).unwrap().1, 10);
        tag += 1;
    }

    let mut token_amount = [0u8; 43];
    tag = 4;
    while tag <= 7 {
        token_amount[0] = tag;
        assert_eq!(parse_post_check(&token_amount).unwrap().1, 43);
        tag += 1;
    }

    let mut token_authority = [0u8; 34];
    token_authority[0] = 8;
    assert_eq!(parse_post_check(&token_authority).unwrap().1, 34);

    let mut token_custody_unchanged = [0u8; 3];
    token_custody_unchanged[0] = 9;
    assert_eq!(parse_post_check(&token_custody_unchanged).unwrap().1, 3);

    let mut token_custody_equals = [0u8; 167];
    token_custody_equals[0] = 10;
    token_custody_equals[3] = TokenProgramKind::Tokenkeg.as_u8();
    assert_eq!(parse_post_check(&token_custody_equals).unwrap().1, 167);

    let mut account_owner = [0u8; 34];
    account_owner[0] = 11;
    assert_eq!(parse_post_check(&account_owner).unwrap().1, 34);

    let mut account_state = [0u8; 78];
    account_state[0] = 12;
    assert_eq!(parse_post_check(&account_state).unwrap().1, 78);
}

#[kani::proof]
fn optional_pubkey_none_requires_zero_bytes() {
    let mut data = [0u8; 167];
    data[0] = 10;
    data[3] = TokenProgramKind::Tokenkeg.as_u8();
    data[68] = 0;
    data[69] = 1;
    data[101] = 0;

    assert!(parse_post_check(&data).is_err());
}

#[kani::proof]
fn validate_post_checks_rejects_trailing_bytes_after_declared_checks() {
    let mut data = [0u8; 11];
    data[0] = 0;
    data[1] = 0;
    data[2..10].copy_from_slice(&1u64.to_le_bytes());
    data[10] = 1;

    assert!(validate_post_checks(1, &data).is_err());
}

#[kani::proof]
fn execute_cpi_checked_parser_accepts_bounded_single_sol_check() {
    let index: u16 = kani::any();
    let target_account_count: u8 = kani::any();
    kani::assume(target_account_count <= 4);
    let wallet_meta_index: u8 = kani::any();
    kani::assume(wallet_meta_index <= target_account_count);
    let min_lamports: u64 = kani::any();
    let mut data = [0u8; 1 + 2 + 1 + 1 + 2 + 4 + 1 + 10];

    data[0] = TAG_EXECUTE_CPI_CHECKED;
    data[1..3].copy_from_slice(&index.to_le_bytes());
    data[3] = wallet_meta_index;
    data[4] = target_account_count;
    data[5..7].copy_from_slice(&4u16.to_le_bytes());
    data[7..11].copy_from_slice(&[1, 2, 3, 4]);
    data[11] = 1;
    data[12] = 0;
    data[13] = wallet_meta_index;
    data[14..22].copy_from_slice(&min_lamports.to_le_bytes());

    match parse_instruction(&data) {
        Ok(Instruction::ExecuteCpiChecked(ix)) => {
            assert_eq!(ix.index, index);
            assert_eq!(ix.wallet_meta_index, wallet_meta_index);
            assert_eq!(ix.target_account_count, target_account_count);
            assert_eq!(ix.target_ix_data, &[1, 2, 3, 4]);
            assert_eq!(ix.post_check_count, 1);
        }
        _ => unreachable!(),
    }
}

fn protected() -> ProtectedAccounts {
    ProtectedAccounts {
        holder: distinct_key(1),
        global_config: distinct_key(2),
        vault_config: distinct_key(3),
        agent_asset: distinct_key(4),
        target_program: distinct_key(5),
    }
}

#[kani::proof]
fn cpi_final_account_count_is_target_plus_wallet() {
    let target_account_count: u8 = kani::any();
    if target_account_count <= MAX_CPI_ACCOUNTS {
        assert_eq!(
            final_cpi_account_count(target_account_count),
            Ok(target_account_count as usize + 1)
        );
    } else {
        assert!(final_cpi_account_count(target_account_count).is_err());
    }
}

#[kani::proof]
fn cpi_wallet_meta_is_readonly_signer_and_indexed_exactly() {
    let wallet = distinct_key(9);
    let accounts = [
        CpiAccountMeta::new(distinct_key(10), false, true),
        CpiAccountMeta::new(distinct_key(11), true, false),
        CpiAccountMeta::new(distinct_key(12), false, false),
    ];

    assert_eq!(planned_account_at(1, 1, 3), Ok(PlannedAccount::Wallet));
    assert_eq!(
        final_account_meta_at(1, 1, 3, &accounts, &wallet),
        Ok(CpiAccountMeta::new(wallet, true, false))
    );
    assert_eq!(
        final_account_meta_at(0, 1, 3, &accounts, &wallet),
        Ok(accounts[0])
    );
    assert_eq!(
        final_account_meta_at(2, 1, 3, &accounts, &wallet),
        Ok(accounts[1])
    );
}

#[kani::proof]
fn cpi_plan_rejects_protected_or_wallet_remaining_accounts() {
    let wallet = distinct_key(9);
    let protected = protected();

    assert!(validate_duplicate_policy(
        &[CpiAccountMeta::new(wallet, false, false)],
        &protected,
        &wallet
    )
    .is_err());
    assert!(validate_duplicate_policy(
        &[CpiAccountMeta::new(protected.holder, false, false)],
        &protected,
        &wallet
    )
    .is_err());
}

#[kani::proof]
fn cpi_duplicate_policy_allows_identical_unprotected_duplicates() {
    let wallet = distinct_key(9);
    let protected = protected();
    let duplicate = CpiAccountMeta::new(distinct_key(10), false, true);

    assert!(validate_duplicate_policy(&[duplicate, duplicate], &protected, &wallet).is_ok());
}

#[kani::proof]
fn lamport_move_result_preserves_sum_and_rent_floor() {
    let from_lamports: u64 = kani::any();
    let to_lamports: u64 = kani::any();
    let lamports: u64 = kani::any();
    let from_floor: u64 = kani::any();

    if let Ok((from_after, to_after)) =
        checked_lamport_move_result(from_lamports, to_lamports, lamports, from_floor)
    {
        assert!(from_after >= from_floor);
        assert_eq!(from_after.checked_add(lamports), Some(from_lamports));
        assert_eq!(to_lamports.checked_add(lamports), Some(to_after));
    }
}

#[kani::proof]
fn execute_cpi_plan_requires_at_least_one_economic_post_check() {
    let wallet = distinct_key(9);
    let target_ix_data = [1u8, 2, 3];
    let mut post_check_data = [0u8; 34];
    post_check_data[0] = 11;
    post_check_data[1] = 0;
    post_check_data[2..34].copy_from_slice(&distinct_key(10));
    let ix = ExecuteCpiChecked {
        index: 0,
        wallet_meta_index: 0,
        target_account_count: 0,
        target_ix_data: &target_ix_data,
        post_check_count: 1,
        post_check_data: &post_check_data,
    };

    assert!(validate_execute_cpi_plan(&ix, &[], &protected(), &wallet).is_err());
}

#[kani::proof]
fn token_transfer_fee_never_exceeds_amount_or_configured_max() {
    let transfer_fee_basis_points = kani::any::<u16>();
    kani::assume(transfer_fee_basis_points <= MAX_FEE_BASIS_POINTS);
    let maximum_fee = kani::any::<u16>() as u64;
    let amount = kani::any::<u16>() as u64;
    let fee_config = TransferFee {
        epoch: kani::any(),
        maximum_fee,
        transfer_fee_basis_points,
    };

    let fee = match fee_config.calculate_fee(amount) {
        Some(fee) => fee,
        None => unreachable!(),
    };
    assert!(fee <= amount);
    assert!(fee <= maximum_fee);
}

#[kani::proof]
fn token_transfer_fee_handles_u64_extreme_values() {
    assert_fee_bounds(u64::MAX, u64::MAX, MAX_FEE_BASIS_POINTS);
    assert_fee_bounds(u64::MAX, 42, MAX_FEE_BASIS_POINTS);
    assert_fee_bounds(u64::MAX, u64::MAX, MAX_FEE_BASIS_POINTS - 1);
    assert_fee_bounds(u64::MAX - 1, u64::MAX, 1);
    assert_fee_bounds(0, u64::MAX, MAX_FEE_BASIS_POINTS);
}

fn assert_fee_bounds(amount: u64, maximum_fee: u64, transfer_fee_basis_points: u16) {
    let fee_config = TransferFee {
        epoch: 0,
        maximum_fee,
        transfer_fee_basis_points,
    };
    let fee = match fee_config.calculate_fee(amount) {
        Some(fee) => fee,
        None => unreachable!(),
    };
    assert!(fee <= amount);
    assert!(fee <= maximum_fee);
}

#[kani::proof]
fn token_program_kind_encoding_is_stable() {
    assert_eq!(TokenProgramKind::Tokenkeg.as_u8(), 0);
    assert_eq!(TokenProgramKind::Token2022.as_u8(), 1);
    assert_eq!(TokenProgramKind::from_u8(0), Ok(TokenProgramKind::Tokenkeg));
    assert_eq!(
        TokenProgramKind::from_u8(1),
        Ok(TokenProgramKind::Token2022)
    );
    assert!(TokenProgramKind::from_u8(2).is_err());
}
