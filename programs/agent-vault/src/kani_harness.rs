use crate::{
    constants::{LABEL_LEN, MAX_CPI_ACCOUNTS, WALLET_FLAG_ACTIVE, WALLET_FLAG_RECOVERY_ONLY},
    cpi_plan::{
        final_account_meta_at, final_cpi_account_count, planned_account_at,
        validate_duplicate_policy, validate_execute_cpi_plan, CpiAccountMeta, PlannedAccount,
        ProtectedAccounts,
    },
    instruction::{
        parse_instruction, parse_post_check, validate_label, ExecuteCpiChecked, Instruction,
        PostCheck, TokenProgramKind, TAG_EXECUTE_CPI_CHECKED,
    },
    state::{
        pack_global_config, pack_vault_config, pack_wallet, unpack_global_config,
        unpack_vault_config, unpack_wallet, AgentWallet, GlobalConfig, VaultConfig,
        GLOBAL_CONFIG_LEN, VAULT_CONFIG_LEN, WALLET_LEN,
    },
    token_state::TransferFee,
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
    let transfer_fee_basis_points = kani::any::<u8>() as u16;
    kani::assume(transfer_fee_basis_points <= 100);
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
