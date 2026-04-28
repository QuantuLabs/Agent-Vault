struct CoverageRow {
    id: &'static str,
    requirement: &'static str,
    runtime_tests: &'static [&'static str],
    formal_harnesses: &'static [&'static str],
}

const RUNTIME_TEST_SOURCE: &str = include_str!("global_config.rs");
const KANI_HARNESS_SOURCE: &str =
    include_str!("../../../programs/agent-vault/src/kani_harness.rs");
const VERIFY_DEVNET_RELEASE_SCRIPT: &str =
    include_str!("../../../scripts/verify-devnet-release.sh");
const VERIFY_FORMAL_SCRIPT: &str = include_str!("../../../scripts/verify-formal.sh");

const COVERAGE: &[CoverageRow] = &[
    CoverageRow {
        id: "identity.core-owner",
        requirement: "Protected instructions require the live Metaplex Core owner and expected collection.",
        runtime_tests: &[
            "protected_ops_follow_live_core_asset_owner_and_collection",
            "init_vault_config_rejects_malformed_registry_agent_account",
        ],
        formal_harnesses: &["core_asset_parser_uses_spec_offsets"],
    },
    CoverageRow {
        id: "identity.registry",
        requirement: "Vault initialization validates the 8004 registry AgentAccount PDA and collection.",
        runtime_tests: &[
            "initializes_global_config_from_devnet_manifest_constants",
            "init_vault_config_checks_treasury_registry_owner_pda_and_dusted_pdas",
        ],
        formal_harnesses: &["agent_account_parser_uses_8004_header_offsets"],
    },
    CoverageRow {
        id: "global-config.immutable",
        requirement: "Global config is initialized once, immutable, and matches manifest constants.",
        runtime_tests: &[
            "initialize_global_config_is_immutable_once_created",
            "rejects_non_manifest_global_config_fields_before_create",
            "config_loads_reject_valid_data_at_wrong_pda_addresses",
        ],
        formal_harnesses: &[
            "global_config_pack_unpack_roundtrip",
            "v0_constants_match_spec_limits_and_tags",
        ],
    },
    CoverageRow {
        id: "fees.activation-only",
        requirement: "V0 charges only the one-time vault activation fee and no routine protocol fees.",
        runtime_tests: &[
            "init_vault_config_is_one_time_fee",
            "routine_v0_instructions_do_not_charge_protocol_fees",
        ],
        formal_harnesses: &["v0_constants_match_spec_limits_and_tags"],
    },
    CoverageRow {
        id: "account-layouts",
        requirement: "Fixed account sizes, reserved bytes, wallet flags, and little-endian indexes match V0.",
        runtime_tests: &[
            "init_create_deposit_and_withdraw_sol_flow",
            "dusted_system_owned_wallet_pda_can_be_created_and_reopened",
        ],
        formal_harnesses: &[
            "vault_config_pack_unpack_roundtrip_with_reserved_v0_flags",
            "vault_config_unpack_rejects_nonzero_reserved_flags",
            "agent_wallet_index_seed_is_u16_little_endian",
        ],
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
        formal_harnesses: &["lamport_move_result_preserves_sum_and_rent_floor"],
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
        formal_harnesses: &["token_program_kind_encoding_is_stable"],
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
            "cpi_duplicate_policy_allows_identical_unprotected_duplicates",
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
        requirement: "Release verification runs format, Clippy, unit, runtime, formal, SBF, and manifest hash checks.",
        runtime_tests: &[
            "scripts/verify-devnet-release.sh",
            "devnet_release_cost_report",
        ],
        formal_harnesses: &["scripts/verify-formal.sh"],
    },
    CoverageRow {
        id: "performance.budgets",
        requirement: "Runtime tests keep CU gates for routine V0 instructions and release verification.",
        runtime_tests: &[
            "init_create_deposit_and_withdraw_sol_flow",
            "routine_v0_instructions_do_not_charge_protocol_fees",
            "execute_cpi_checked_invokes_memo_with_only_wallet_meta",
            "devnet_release_cost_report",
        ],
        formal_harnesses: &["v0_constants_match_spec_limits_and_tags"],
    },
    CoverageRow {
        id: "rent.snapshots",
        requirement: "Runtime tests snapshot active rent for global config, vault config, wallet, and token accounts.",
        runtime_tests: &["rent_snapshots_match_active_rent"],
        formal_harnesses: &["v0_constants_match_spec_limits_and_tags"],
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
            !row.runtime_tests.is_empty(),
            "coverage row {} has no LiteSVM/runtime coverage",
            row.id
        );
        assert!(
            !row.formal_harnesses.is_empty(),
            "coverage row {} has no Kani/formal coverage",
            row.id
        );

        for runtime_test in row.runtime_tests {
            assert!(
                runtime_item_exists(runtime_test),
                "coverage row {} references missing runtime test or script {}",
                row.id,
                runtime_test
            );
        }

        for harness in row.formal_harnesses {
            assert!(
                formal_item_exists(harness),
                "coverage row {} references missing Kani harness or script {}",
                row.id,
                harness
            );
        }
    }
}

fn runtime_item_exists(name: &str) -> bool {
    if name == "scripts/verify-devnet-release.sh" {
        return VERIFY_DEVNET_RELEASE_SCRIPT.contains("verify-formal.sh")
            && VERIFY_DEVNET_RELEASE_SCRIPT.contains("cargo clippy")
            && VERIFY_DEVNET_RELEASE_SCRIPT.contains("--test-threads=1")
            && VERIFY_DEVNET_RELEASE_SCRIPT.contains("devnet_release_cost_report");
    }

    RUNTIME_TEST_SOURCE.contains(&format!("fn {name}("))
}

fn formal_item_exists(name: &str) -> bool {
    if name == "scripts/verify-formal.sh" {
        return VERIFY_FORMAL_SCRIPT.contains("cargo kani");
    }

    KANI_HARNESS_SOURCE.contains(&format!("fn {name}("))
}
