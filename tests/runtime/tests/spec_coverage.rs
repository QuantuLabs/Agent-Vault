struct CoverageRow {
    id: &'static str,
    requirement: &'static str,
    runtime_tests: &'static [&'static str],
    formal_harnesses: &'static [&'static str],
}

const COVERAGE: &[CoverageRow] = &[
    CoverageRow {
        id: "identity.core-owner",
        requirement: "Protected instructions require the live Metaplex Core owner and expected collection.",
        runtime_tests: &[
            "protected_ops_follow_live_core_asset_owner_and_collection",
            "init_vault_config_rejects_malformed_registry_agent_account",
        ],
        formal_harnesses: &[],
    },
    CoverageRow {
        id: "identity.registry",
        requirement: "Vault initialization validates the 8004 registry AgentAccount PDA and collection.",
        runtime_tests: &[
            "initializes_global_config_from_devnet_manifest_constants",
            "init_vault_config_checks_treasury_registry_owner_pda_and_dusted_pdas",
        ],
        formal_harnesses: &[],
    },
    CoverageRow {
        id: "global-config.immutable",
        requirement: "Global config is initialized once, immutable, and matches manifest constants.",
        runtime_tests: &[
            "initialize_global_config_is_immutable_once_created",
            "rejects_non_manifest_global_config_fields_before_create",
        ],
        formal_harnesses: &["global_config_pack_unpack_roundtrip"],
    },
    CoverageRow {
        id: "wallet.indexed-pdas",
        requirement: "Wallets are indexed PDA accounts with stable seeds and bounded u16 count.",
        runtime_tests: &[
            "init_create_deposit_and_withdraw_sol_flow",
            "deposit_sol_is_permissionless_but_wallet_count_cannot_overflow",
        ],
        formal_harnesses: &[
            "wallet_pack_unpack_roundtrip_for_valid_v0_flags",
            "wallet_unpack_rejects_invalid_flag_combinations",
        ],
    },
    CoverageRow {
        id: "sol.flows",
        requirement: "SOL deposit, withdraw, transfer, close, rent floor, and recovery paths are enforced.",
        runtime_tests: &[
            "init_create_deposit_and_withdraw_sol_flow",
            "sol_paths_preserve_rent_floor_and_reject_duplicate_moves",
            "recovery_only_wallet_allows_constrained_cleanup_paths",
            "recovery_only_wallet_rejects_hot_path_operations",
        ],
        formal_harnesses: &[],
    },
    CoverageRow {
        id: "spl-token.basic",
        requirement: "Wallet-owned ATAs support create, transfer_checked, and close for Tokenkeg.",
        runtime_tests: &["tokenkeg_ata_transfer_and_close_paths_work"],
        formal_harnesses: &["token_program_kind_encoding_is_stable"],
    },
    CoverageRow {
        id: "token-2022.minimal",
        requirement: "Minimal Token-2022 support covers transfer-fee mints and supported account extensions.",
        runtime_tests: &[
            "token_2022_create_and_close_ata_paths_work",
            "token_2022_transfer_and_extension_rejections_are_checked",
            "execute_cpi_checked_token_2022_custody_equals_checks_extension_hash",
        ],
        formal_harnesses: &["token_transfer_fee_never_exceeds_amount_or_configured_max"],
    },
    CoverageRow {
        id: "wsol",
        requirement: "WSOL wrap and unwrap preserve wallet authority, rent floor, and native account invariants.",
        runtime_tests: &[
            "wsol_wrap_and_unwrap_preserve_wallet_authority_and_rent",
            "wsol_wrap_rejects_malformed_wallet_atas",
            "unwrap_sol_rejects_malformed_wsol_ata",
        ],
        formal_harnesses: &[],
    },
    CoverageRow {
        id: "checked-cpi.core",
        requirement: "Checked CPI inserts the wallet as readonly signer, rejects denied targets, and requires post-checks.",
        runtime_tests: &[
            "execute_cpi_checked_invokes_memo_with_only_wallet_meta",
            "execute_cpi_checked_rejects_denied_target_programs",
            "execute_cpi_checked_rejects_writable_wallet_account",
            "execute_cpi_checked_rejects_missing_economic_post_check",
        ],
        formal_harnesses: &[
            "cpi_final_account_count_is_target_plus_wallet",
            "cpi_wallet_meta_is_readonly_signer_and_indexed_exactly",
            "execute_cpi_plan_requires_at_least_one_economic_post_check",
        ],
    },
    CoverageRow {
        id: "checked-cpi.custody",
        requirement: "Writable wallet-controlled token accounts require economic and custody checks.",
        runtime_tests: &[
            "execute_cpi_checked_requires_custody_checks_for_writable_wallet_tokens",
            "execute_cpi_checked_validates_token_custody_equals_and_ata_status",
            "execute_cpi_checked_rejects_actual_token_custody_mutation",
            "execute_cpi_checked_token_custody_equals_supports_new_wallet_control",
        ],
        formal_harnesses: &[
            "cpi_plan_rejects_protected_or_wallet_remaining_accounts",
            "sol_balance_post_check_parser_is_total_for_valid_payloads",
        ],
    },
    CoverageRow {
        id: "checked-cpi.defi",
        requirement: "Swap-like CPI flows enforce max-input/min-output and account-state checks.",
        runtime_tests: &[
            "execute_cpi_checked_mock_swap_enforces_max_input_and_min_output",
            "execute_cpi_checked_requires_state_checks_for_writable_non_token_accounts",
        ],
        formal_harnesses: &["execute_cpi_checked_parser_accepts_bounded_single_sol_check"],
    },
    CoverageRow {
        id: "parsing.labels",
        requirement: "Instruction labels are fixed-width UTF-8 with canonical zero suffix.",
        runtime_tests: &["update_wallet_label_persists_utf8_label"],
        formal_harnesses: &[
            "empty_label_is_valid",
            "label_rejects_nonzero_suffix_after_nul",
        ],
    },
    CoverageRow {
        id: "release.verification",
        requirement: "Release verification runs format, unit, runtime, formal, SBF, and manifest hash checks.",
        runtime_tests: &["scripts/verify-devnet-release.sh"],
        formal_harnesses: &["scripts/verify-formal.sh"],
    },
];

#[test]
fn v0_spec_coverage_matrix_has_no_gaps() {
    assert!(
        COVERAGE.len() >= 12,
        "coverage matrix must track every V0 spec area"
    );

    for row in COVERAGE {
        assert!(!row.id.is_empty(), "coverage row has empty id");
        assert!(
            !row.requirement.is_empty(),
            "coverage row {} has empty requirement",
            row.id
        );
        assert!(
            !row.runtime_tests.is_empty() || !row.formal_harnesses.is_empty(),
            "coverage row {} has no tests or harnesses",
            row.id
        );
    }
}
