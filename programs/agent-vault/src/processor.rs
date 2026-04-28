use crate::{
    agent_account::parse_agent_account,
    constants::{
        is_loader_program, ASSOCIATED_TOKEN_PROGRAM_ID, EXPECTED_ACTIVATION_FEE_LAMPORTS,
        EXPECTED_COLLECTION, EXPECTED_FEE_TREASURY, EXPECTED_INITIALIZER,
        EXPECTED_REGISTRY_PROGRAM, MAX_CPI_ACCOUNTS, MAX_WALLETS, NATIVE_MINT_ID,
        SEED_AGENT_WALLET, SEED_GLOBAL_CONFIG, SEED_VAULT_CONFIG, TOKEN_2022_PROGRAM_ID,
        TOKEN_PROGRAM_ID, WALLET_FLAG_ACTIVE, WALLET_FLAG_RECOVERY_ONLY,
    },
    core_asset::assert_core_asset_owner_and_collection,
    cpi_plan::{
        validate_execute_cpi_plan, validate_execute_cpi_post_check_indexes, CpiAccountMeta,
        ProtectedAccounts,
    },
    error::AgentVaultError,
    instruction::{
        parse_instruction, CreateWallet, CreateWalletAta, DepositSol, IndexedWallet, Instruction,
        OptionalPubkey, PostCheck, ReopenWalletForRecovery, TokenProgramKind, TransferSol,
        TransferSpl, UpdateWalletLabel, WithdrawSol, WrapSol,
    },
    pda::{
        agent_wallet_index_seed, derive_agent_wallet, derive_associated_token_account,
        derive_global_config, derive_registry_agent_account, derive_vault_config,
        validate_agent_wallet_pda, validate_global_config_pda, validate_vault_config_pda,
    },
    state::{
        pack_global_config, pack_vault_config, pack_wallet, read_global_config_bump,
        read_vault_config_bump, unpack_global_config_after_header,
        unpack_vault_config_after_header, unpack_wallet, AgentWallet, GlobalConfig, VaultConfig,
        GLOBAL_CONFIG_LEN, PUBKEY_LEN, VAULT_CONFIG_LEN, WALLET_LABEL_OFFSET, WALLET_LEN,
    },
    token_state::{parse_mint, parse_token_account_for_mint, TokenAccount, TokenMint},
    validation::{
        assert_associated_token_program, assert_clock_sysvar, assert_native_mint, assert_owned_by,
        assert_rent_sysvar, assert_signer, assert_system_program, assert_token_program_any,
        assert_uninitialized_pda, assert_writable,
    },
};
use alloc::vec::Vec;
use pinocchio::{
    cpi::{invoke_signed, invoke_signed_with_slice, Seed, Signer},
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    AccountView, Address, ProgramResult,
};

#[inline(never)]
pub fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    data: &[u8],
) -> ProgramResult {
    match parse_instruction(data)? {
        Instruction::InitializeGlobalConfig(ix) => {
            process_initialize_global_config(program_id, accounts, &ix)
        }
        Instruction::InitVaultConfig => process_init_vault_config(program_id, accounts),
        Instruction::CreateWallet(ix) => process_create_wallet(program_id, accounts, &ix),
        Instruction::UpdateWalletLabel(ix) => {
            process_update_wallet_label(program_id, accounts, &ix)
        }
        Instruction::DepositSol(ix) => process_deposit_sol(program_id, accounts, &ix),
        Instruction::WithdrawSol(ix) => process_withdraw_sol(program_id, accounts, &ix),
        Instruction::TransferSol(ix) => process_transfer_sol(program_id, accounts, &ix),
        Instruction::CloseWallet => process_close_wallet(program_id, accounts),
        Instruction::ReopenWalletForRecovery(ix) => {
            process_reopen_wallet_for_recovery(program_id, accounts, &ix)
        }
        Instruction::CreateWalletAta(ix) => process_create_wallet_ata(program_id, accounts, &ix),
        Instruction::TransferSpl(ix) => process_transfer_spl(program_id, accounts, &ix),
        Instruction::WrapSol(ix) => process_wrap_sol(program_id, accounts, &ix),
        Instruction::UnwrapSol(ix) => process_unwrap_sol(program_id, accounts, &ix),
        Instruction::CloseWalletAta(ix) => process_close_wallet_ata(program_id, accounts, &ix),
        Instruction::ExecuteCpiChecked(ix) => {
            process_execute_cpi_checked(program_id, accounts, &ix)
        }
    }
}

#[inline(never)]
fn process_initialize_global_config(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &crate::instruction::InitializeGlobalConfig,
) -> ProgramResult {
    require_account_count(accounts, 3)?;
    let initializer = account(accounts, 0)?;
    let global_config = account(accounts, 1)?;
    let system_program = account(accounts, 2)?;

    assert_signer(initializer)?;
    assert_writable(initializer)?;
    assert_writable(global_config)?;
    assert_system_program(system_program)?;
    if initializer.address() != &EXPECTED_INITIALIZER {
        return Err(AgentVaultError::InvalidSigner.into());
    }
    if ix.registry_program != address_bytes(&EXPECTED_REGISTRY_PROGRAM)
        || ix.collection != address_bytes(&EXPECTED_COLLECTION)
        || ix.fee_treasury != address_bytes(&EXPECTED_FEE_TREASURY)
        || ix.vault_activation_fee_lamports != EXPECTED_ACTIVATION_FEE_LAMPORTS
    {
        return Err(AgentVaultError::InvalidGlobalConfig.into());
    }

    let pda = derive_global_config(program_id)?;
    if global_config.address() != &pda.address {
        return Err(AgentVaultError::InvalidPda.into());
    }
    assert_uninitialized_pda(global_config)?;

    let bump_seed = [pda.bump];
    let seeds = [Seed::from(SEED_GLOBAL_CONFIG), Seed::from(&bump_seed)];
    let signer = Signer::from(&seeds);
    pinocchio_system::create_account_with_minimum_balance_signed(
        global_config,
        GLOBAL_CONFIG_LEN,
        program_id,
        initializer,
        None,
        &[signer],
    )?;

    let config = GlobalConfig {
        bump: pda.bump,
        initializer: address_bytes(initializer.address()),
        registry_program: ix.registry_program,
        collection: ix.collection,
        fee_treasury: ix.fee_treasury,
        vault_activation_fee_lamports: ix.vault_activation_fee_lamports,
    };
    let mut data = global_config.try_borrow_mut()?;
    pack_global_config(&config, &mut data)
}

#[inline(never)]
fn process_init_vault_config(program_id: &Address, accounts: &[AccountView]) -> ProgramResult {
    require_account_count(accounts, 8)?;
    let holder = account(accounts, 0)?;
    let global_config_account = account(accounts, 1)?;
    let vault_config_account = account(accounts, 2)?;
    let agent_asset = account(accounts, 3)?;
    let agent_account = account(accounts, 4)?;
    let fee_treasury = account(accounts, 5)?;
    let clock_sysvar = account(accounts, 6)?;
    let system_program = account(accounts, 7)?;

    assert_signer(holder)?;
    assert_writable(holder)?;
    assert_writable(vault_config_account)?;
    assert_writable(fee_treasury)?;
    assert_clock_sysvar(clock_sysvar)?;
    assert_system_program(system_program)?;

    let global = load_global_config(program_id, global_config_account)?;
    assert_core_asset_owner_and_collection(holder, agent_asset, &global.collection)?;
    if fee_treasury.address().as_ref() != global.fee_treasury {
        return Err(AgentVaultError::InvalidTreasury.into());
    }

    let registry_program = Address::new_from_array(global.registry_program);
    assert_owned_by(agent_account, &registry_program)?;
    let registry_pda = derive_registry_agent_account(&registry_program, agent_asset.address())?;
    if agent_account.address() != &registry_pda.address {
        return Err(AgentVaultError::InvalidAgentAccount.into());
    }
    let agent_data = agent_account.try_borrow()?;
    let parsed_agent = parse_agent_account(&agent_data)?;
    if parsed_agent.collection != global.collection
        || parsed_agent.asset != address_bytes(agent_asset.address())
        || parsed_agent.bump != registry_pda.bump
    {
        return Err(AgentVaultError::InvalidAgentAccount.into());
    }

    let vault_pda = derive_vault_config(program_id, agent_asset.address())?;
    if vault_config_account.address() != &vault_pda.address {
        return Err(AgentVaultError::InvalidPda.into());
    }
    assert_uninitialized_pda(vault_config_account)?;

    if global.vault_activation_fee_lamports > 0 {
        checked_system_transfer(holder, fee_treasury, global.vault_activation_fee_lamports)?;
        let mut logger = pinocchio_log::logger::Logger::<64>::default();
        logger.append("AgentVaultActivationFeePaid lamports=");
        logger.append(global.vault_activation_fee_lamports);
        logger.log();
    }

    let bump_seed = [vault_pda.bump];
    let seeds = [
        Seed::from(SEED_VAULT_CONFIG),
        Seed::from(agent_asset.address().as_ref()),
        Seed::from(&bump_seed),
    ];
    let signer = Signer::from(&seeds);
    pinocchio_system::create_account_with_minimum_balance_signed(
        vault_config_account,
        VAULT_CONFIG_LEN,
        program_id,
        holder,
        None,
        &[signer],
    )?;

    let clock = Clock::from_account_view(clock_sysvar)?;
    let config = VaultConfig {
        bump: vault_pda.bump,
        wallet_count: 0,
        flags: 0,
        created_at: clock.unix_timestamp,
    };
    let mut data = vault_config_account.try_borrow_mut()?;
    pack_vault_config(&config, &mut data)
}

#[inline(never)]
fn process_create_wallet(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &CreateWallet,
) -> ProgramResult {
    require_account_count(accounts, 5)?;
    let holder = account(accounts, 0)?;
    let vault_config_account = account(accounts, 1)?;
    let wallet_account = account(accounts, 2)?;
    let agent_asset = account(accounts, 3)?;
    let system_program = account(accounts, 4)?;

    assert_signer(holder)?;
    assert_writable(holder)?;
    assert_writable(vault_config_account)?;
    assert_writable(wallet_account)?;
    assert_system_program(system_program)?;

    let expected_collection = address_bytes(&EXPECTED_COLLECTION);
    assert_core_asset_owner_and_collection(holder, agent_asset, &expected_collection)?;
    let mut vault = load_vault_config(program_id, vault_config_account, agent_asset.address())?;
    if vault.wallet_count == MAX_WALLETS {
        return Err(AgentVaultError::WalletCountOverflow.into());
    }
    let index = vault.wallet_count;
    let wallet_pda = derive_agent_wallet(program_id, agent_asset.address(), index)?;
    if wallet_account.address() != &wallet_pda.address {
        return Err(AgentVaultError::InvalidPda.into());
    }
    assert_uninitialized_pda(wallet_account)?;

    create_wallet_account(
        program_id,
        holder,
        wallet_account,
        agent_asset.address(),
        index,
        wallet_pda.bump,
    )?;

    let wallet = AgentWallet {
        bump: wallet_pda.bump,
        index,
        flags: WALLET_FLAG_ACTIVE,
        label: ix.label,
    };
    {
        let mut data = wallet_account.try_borrow_mut()?;
        pack_wallet(&wallet, &mut data)?;
    }

    vault.wallet_count = vault
        .wallet_count
        .checked_add(1)
        .ok_or(AgentVaultError::WalletCountOverflow)?;
    let mut vault_data = vault_config_account.try_borrow_mut()?;
    pack_vault_config(&vault, &mut vault_data)
}

#[inline(never)]
fn process_update_wallet_label(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &UpdateWalletLabel,
) -> ProgramResult {
    require_account_count(accounts, 5)?;
    let holder = account(accounts, 0)?;
    let global_config_account = account(accounts, 1)?;
    let vault_config_account = account(accounts, 2)?;
    let wallet_account = account(accounts, 3)?;
    let agent_asset = account(accounts, 4)?;

    assert_signer(holder)?;
    assert_writable(wallet_account)?;
    let global = load_global_config(program_id, global_config_account)?;
    assert_core_asset_owner_and_collection(holder, agent_asset, &global.collection)?;
    let _vault = load_vault_config(program_id, vault_config_account, agent_asset.address())?;
    let wallet = load_wallet(program_id, wallet_account, agent_asset.address())?;
    if !wallet.is_active() || wallet.index != ix.index {
        return Err(AgentVaultError::InvalidWallet.into());
    }
    let mut data = wallet_account.try_borrow_mut()?;
    data[WALLET_LABEL_OFFSET..WALLET_LABEL_OFFSET + ix.label.len()].copy_from_slice(&ix.label);
    Ok(())
}

#[inline(never)]
fn process_deposit_sol(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &DepositSol,
) -> ProgramResult {
    require_account_count(accounts, 4)?;
    let funder = account(accounts, 0)?;
    let wallet_account = account(accounts, 1)?;
    let agent_asset = account(accounts, 2)?;
    let system_program = account(accounts, 3)?;

    assert_signer(funder)?;
    assert_writable(funder)?;
    assert_writable(wallet_account)?;
    assert_system_program(system_program)?;
    let wallet = load_wallet(program_id, wallet_account, agent_asset.address())?;
    if wallet.is_recovery_only() {
        return Err(AgentVaultError::WalletRecoveryOnly.into());
    }
    if !wallet.is_active() {
        return Err(AgentVaultError::WalletInactive.into());
    }
    checked_system_transfer(funder, wallet_account, ix.amount)
}

#[inline(never)]
fn process_withdraw_sol(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &WithdrawSol,
) -> ProgramResult {
    require_account_count(accounts, 5)?;
    let holder = account(accounts, 0)?;
    let wallet_account = account(accounts, 1)?;
    let destination = account(accounts, 2)?;
    let agent_asset = account(accounts, 3)?;
    let rent_sysvar = account(accounts, 4)?;

    assert_signer(holder)?;
    assert_writable(wallet_account)?;
    assert_writable(destination)?;
    assert_rent_sysvar(rent_sysvar)?;
    let expected_collection = address_bytes(&EXPECTED_COLLECTION);
    assert_core_asset_owner_and_collection(holder, agent_asset, &expected_collection)?;
    let wallet = load_wallet(program_id, wallet_account, agent_asset.address())?;
    if wallet.index != ix.index {
        return Err(AgentVaultError::InvalidWallet.into());
    }
    if !wallet.is_active() && !wallet.is_recovery_only() {
        return Err(AgentVaultError::WalletInactive.into());
    }
    let rent_floor = rent_minimum(rent_sysvar, WALLET_LEN)?;
    checked_lamport_move_preserving_floor(wallet_account, destination, ix.amount, rent_floor)
}

#[inline(never)]
fn process_transfer_sol(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &TransferSol,
) -> ProgramResult {
    require_account_count(accounts, 5)?;
    let holder = account(accounts, 0)?;
    let from_wallet_account = account(accounts, 1)?;
    let to_wallet_account = account(accounts, 2)?;
    let agent_asset = account(accounts, 3)?;
    let rent_sysvar = account(accounts, 4)?;

    assert_signer(holder)?;
    assert_writable(from_wallet_account)?;
    assert_writable(to_wallet_account)?;
    assert_rent_sysvar(rent_sysvar)?;
    let expected_collection = address_bytes(&EXPECTED_COLLECTION);
    assert_core_asset_owner_and_collection(holder, agent_asset, &expected_collection)?;
    let from_wallet = load_wallet(program_id, from_wallet_account, agent_asset.address())?;
    let to_wallet = load_wallet(program_id, to_wallet_account, agent_asset.address())?;
    if from_wallet.index != ix.from_index || to_wallet.index != ix.to_index {
        return Err(AgentVaultError::InvalidWallet.into());
    }
    if !from_wallet.is_active() || !to_wallet.is_active() {
        return Err(AgentVaultError::WalletInactive.into());
    }
    let rent_floor = rent_minimum(rent_sysvar, WALLET_LEN)?;
    checked_lamport_move_preserving_floor(
        from_wallet_account,
        to_wallet_account,
        ix.amount,
        rent_floor,
    )
}

#[inline(never)]
fn process_close_wallet(program_id: &Address, accounts: &[AccountView]) -> ProgramResult {
    require_account_count(accounts, 7)?;
    let holder = account(accounts, 0)?;
    let global_config_account = account(accounts, 1)?;
    let vault_config_account = account(accounts, 2)?;
    let wallet_account = account(accounts, 3)?;
    let rent_receiver = account(accounts, 4)?;
    let agent_asset = account(accounts, 5)?;
    let rent_sysvar = account(accounts, 6)?;

    assert_signer(holder)?;
    assert_writable(wallet_account)?;
    assert_writable(rent_receiver)?;
    assert_rent_sysvar(rent_sysvar)?;
    let global = load_global_config(program_id, global_config_account)?;
    assert_core_asset_owner_and_collection(holder, agent_asset, &global.collection)?;
    let vault = load_vault_config(program_id, vault_config_account, agent_asset.address())?;
    let wallet = load_wallet(program_id, wallet_account, agent_asset.address())?;
    if !wallet.is_active() && !wallet.is_recovery_only() {
        return Err(AgentVaultError::WalletInactive.into());
    }
    if wallet.index >= vault.wallet_count {
        return Err(AgentVaultError::InvalidWallet.into());
    }
    let rent_floor = rent_minimum(rent_sysvar, WALLET_LEN)?;
    if wallet_account.lamports() > rent_floor {
        return Err(AgentVaultError::InsufficientLamports.into());
    }

    {
        let mut data = wallet_account.try_borrow_mut()?;
        data.fill(0);
    }
    checked_close_account(wallet_account, rent_receiver)
}

#[inline(never)]
fn process_reopen_wallet_for_recovery(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &ReopenWalletForRecovery,
) -> ProgramResult {
    require_account_count(accounts, 6)?;
    let holder = account(accounts, 0)?;
    let global_config_account = account(accounts, 1)?;
    let vault_config_account = account(accounts, 2)?;
    let wallet_account = account(accounts, 3)?;
    let agent_asset = account(accounts, 4)?;
    let system_program = account(accounts, 5)?;

    assert_signer(holder)?;
    assert_writable(holder)?;
    assert_writable(wallet_account)?;
    assert_system_program(system_program)?;
    let global = load_global_config(program_id, global_config_account)?;
    assert_core_asset_owner_and_collection(holder, agent_asset, &global.collection)?;
    let vault = load_vault_config(program_id, vault_config_account, agent_asset.address())?;
    if ix.index >= vault.wallet_count {
        return Err(AgentVaultError::InvalidWallet.into());
    }
    let wallet_pda = derive_agent_wallet(program_id, agent_asset.address(), ix.index)?;
    if wallet_account.address() != &wallet_pda.address {
        return Err(AgentVaultError::InvalidPda.into());
    }
    assert_uninitialized_pda(wallet_account)?;
    create_wallet_account(
        program_id,
        holder,
        wallet_account,
        agent_asset.address(),
        ix.index,
        wallet_pda.bump,
    )?;
    let wallet = AgentWallet {
        bump: wallet_pda.bump,
        index: ix.index,
        flags: WALLET_FLAG_RECOVERY_ONLY,
        label: ix.label,
    };
    let mut data = wallet_account.try_borrow_mut()?;
    pack_wallet(&wallet, &mut data)
}

#[inline(never)]
fn process_create_wallet_ata(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &CreateWalletAta,
) -> ProgramResult {
    require_account_count(accounts, 10)?;
    let holder = account(accounts, 0)?;
    let global_config_account = account(accounts, 1)?;
    let vault_config_account = account(accounts, 2)?;
    let wallet_account = account(accounts, 3)?;
    let agent_asset = account(accounts, 4)?;
    let mint = account(accounts, 5)?;
    let wallet_ata = account(accounts, 6)?;
    let associated_token_program = account(accounts, 7)?;
    let token_program = account(accounts, 8)?;
    let system_program = account(accounts, 9)?;

    assert_signer(holder)?;
    assert_writable(holder)?;
    assert_writable(wallet_ata)?;
    assert_associated_token_program(associated_token_program)?;
    assert_system_program(system_program)?;
    assert_token_program_kind(token_program, ix.token_program_kind)?;

    let global = load_global_config(program_id, global_config_account)?;
    assert_core_asset_owner_and_collection(holder, agent_asset, &global.collection)?;
    let _vault = load_vault_config(program_id, vault_config_account, agent_asset.address())?;
    let wallet = load_wallet(program_id, wallet_account, agent_asset.address())?;
    if wallet.index != ix.index {
        return Err(AgentVaultError::InvalidWallet.into());
    }
    if wallet.is_recovery_only() {
        return Err(AgentVaultError::WalletRecoveryOnly.into());
    }
    if !wallet.is_active() {
        return Err(AgentVaultError::WalletInactive.into());
    }
    assert_owned_by(mint, token_program.address())?;
    {
        let mint_data = mint.try_borrow()?;
        parse_mint(&mint_data, ix.token_program_kind)?;
    }
    assert_associated_token_address(
        wallet_ata,
        wallet_account.address(),
        mint.address(),
        token_program.address(),
    )?;

    invoke_create_ata(
        holder,
        wallet_ata,
        wallet_account,
        mint,
        system_program,
        token_program,
    )
}

#[inline(never)]
fn process_transfer_spl(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &TransferSpl,
) -> ProgramResult {
    require_account_count(accounts, 9)?;
    let holder = account(accounts, 0)?;
    let global_config_account = account(accounts, 1)?;
    let vault_config_account = account(accounts, 2)?;
    let wallet_account = account(accounts, 3)?;
    let agent_asset = account(accounts, 4)?;
    let mint = account(accounts, 5)?;
    let source_token_account = account(accounts, 6)?;
    let destination_token_account = account(accounts, 7)?;
    let token_program = account(accounts, 8)?;

    assert_signer(holder)?;
    assert_writable(source_token_account)?;
    assert_writable(destination_token_account)?;
    assert_token_program_any(token_program)?;

    let token_program_kind = token_program_kind_from_account(token_program)?;
    let global = load_global_config(program_id, global_config_account)?;
    assert_core_asset_owner_and_collection(holder, agent_asset, &global.collection)?;
    let _vault = load_vault_config(program_id, vault_config_account, agent_asset.address())?;
    let wallet = load_wallet(program_id, wallet_account, agent_asset.address())?;
    if wallet.index != ix.index {
        return Err(AgentVaultError::InvalidWallet.into());
    }
    if !wallet.is_active() && !wallet.is_recovery_only() {
        return Err(AgentVaultError::WalletInactive.into());
    }

    let wallet_key = address_bytes(wallet_account.address());
    let use_transfer_fee = {
        assert_owned_by(mint, token_program.address())?;
        assert_owned_by(source_token_account, token_program.address())?;
        assert_owned_by(destination_token_account, token_program.address())?;
        assert_associated_token_address(
            source_token_account,
            wallet_account.address(),
            mint.address(),
            token_program.address(),
        )?;

        let mint_data = mint.try_borrow()?;
        let parsed_mint = parse_mint(&mint_data, token_program_kind)?;
        if parsed_mint.decimals != ix.decimals {
            return Err(AgentVaultError::InvalidTokenAccount.into());
        }

        let source_data = source_token_account.try_borrow()?;
        let source = parse_token_account_for_mint(&source_data, token_program_kind, &parsed_mint)?;
        validate_source_wallet_token_account(&source, &wallet_key, mint.address())?;

        let destination_data = destination_token_account.try_borrow()?;
        let destination =
            parse_token_account_for_mint(&destination_data, token_program_kind, &parsed_mint)?;
        if destination.mint != address_bytes(mint.address()) {
            return Err(AgentVaultError::InvalidTokenAccount.into());
        }
        if wallet.is_recovery_only() && token_account_has_wallet_custody(&destination, &wallet_key)
        {
            return Err(AgentVaultError::WalletRecoveryOnly.into());
        }
        if token_account_has_wallet_custody(&destination, &wallet_key) {
            assert_associated_token_address(
                destination_token_account,
                wallet_account.address(),
                mint.address(),
                token_program.address(),
            )?;
            validate_destination_wallet_token_account(&destination, &wallet_key)?;
        }

        match token_program_kind {
            TokenProgramKind::Tokenkeg => {
                if ix.expected_fee != 0 {
                    return Err(AgentVaultError::InvalidTokenAccount.into());
                }
                false
            }
            TokenProgramKind::Token2022 => {
                if let Some(config) = parsed_mint.extensions.transfer_fee_config {
                    let clock = Clock::get()?;
                    let expected_fee = config
                        .calculate_epoch_fee(clock.epoch, ix.amount)
                        .ok_or(AgentVaultError::ArithmeticOverflow)?;
                    if ix.expected_fee != expected_fee {
                        return Err(AgentVaultError::InvalidTokenAccount.into());
                    }
                    true
                } else {
                    if ix.expected_fee != 0 {
                        return Err(AgentVaultError::InvalidTokenAccount.into());
                    }
                    false
                }
            }
        }
    };

    invoke_token_transfer_checked(
        token_program,
        source_token_account,
        mint,
        destination_token_account,
        wallet_account,
        agent_asset.address(),
        &wallet,
        ix.amount,
        ix.decimals,
        ix.expected_fee,
        use_transfer_fee,
    )
}

#[inline(never)]
fn process_wrap_sol(program_id: &Address, accounts: &[AccountView], ix: &WrapSol) -> ProgramResult {
    require_account_count(accounts, 9)?;
    let holder = account(accounts, 0)?;
    let global_config_account = account(accounts, 1)?;
    let vault_config_account = account(accounts, 2)?;
    let wallet_account = account(accounts, 3)?;
    let agent_asset = account(accounts, 4)?;
    let wallet_wsol_ata = account(accounts, 5)?;
    let native_mint = account(accounts, 6)?;
    let token_program = account(accounts, 7)?;
    let rent_sysvar = account(accounts, 8)?;

    assert_signer(holder)?;
    assert_writable(wallet_account)?;
    assert_writable(wallet_wsol_ata)?;
    assert_native_mint(native_mint)?;
    assert_token_program_kind(token_program, TokenProgramKind::Tokenkeg)?;
    assert_rent_sysvar(rent_sysvar)?;

    let global = load_global_config(program_id, global_config_account)?;
    assert_core_asset_owner_and_collection(holder, agent_asset, &global.collection)?;
    let _vault = load_vault_config(program_id, vault_config_account, agent_asset.address())?;
    let wallet = load_wallet(program_id, wallet_account, agent_asset.address())?;
    if wallet.index != ix.index {
        return Err(AgentVaultError::InvalidWallet.into());
    }
    if wallet.is_recovery_only() {
        return Err(AgentVaultError::WalletRecoveryOnly.into());
    }
    if !wallet.is_active() {
        return Err(AgentVaultError::WalletInactive.into());
    }
    validate_wsol_ata(
        wallet_wsol_ata,
        wallet_account.address(),
        token_program.address(),
    )?;
    let rent_floor = rent_minimum(rent_sysvar, WALLET_LEN)?;
    checked_lamport_move_preserving_floor(wallet_account, wallet_wsol_ata, ix.amount, rent_floor)
}

#[inline(never)]
fn process_unwrap_sol(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &IndexedWallet,
) -> ProgramResult {
    require_account_count(accounts, 7)?;
    let holder = account(accounts, 0)?;
    let global_config_account = account(accounts, 1)?;
    let vault_config_account = account(accounts, 2)?;
    let wallet_account = account(accounts, 3)?;
    let agent_asset = account(accounts, 4)?;
    let wallet_wsol_ata = account(accounts, 5)?;
    let token_program = account(accounts, 6)?;

    assert_signer(holder)?;
    assert_writable(wallet_account)?;
    assert_writable(wallet_wsol_ata)?;
    assert_token_program_kind(token_program, TokenProgramKind::Tokenkeg)?;
    let global = load_global_config(program_id, global_config_account)?;
    assert_core_asset_owner_and_collection(holder, agent_asset, &global.collection)?;
    let _vault = load_vault_config(program_id, vault_config_account, agent_asset.address())?;
    let wallet = load_wallet(program_id, wallet_account, agent_asset.address())?;
    if wallet.index != ix.index {
        return Err(AgentVaultError::InvalidWallet.into());
    }
    if !wallet.is_active() && !wallet.is_recovery_only() {
        return Err(AgentVaultError::WalletInactive.into());
    }
    validate_wsol_ata(
        wallet_wsol_ata,
        wallet_account.address(),
        token_program.address(),
    )?;
    invoke_token_close_account(
        token_program,
        wallet_wsol_ata,
        wallet_account,
        wallet_account,
        agent_asset.address(),
        &wallet,
    )
}

#[inline(never)]
fn process_close_wallet_ata(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &IndexedWallet,
) -> ProgramResult {
    require_account_count(accounts, 10)?;
    let holder = account(accounts, 0)?;
    let global_config_account = account(accounts, 1)?;
    let vault_config_account = account(accounts, 2)?;
    let wallet_account = account(accounts, 3)?;
    let agent_asset = account(accounts, 4)?;
    let mint = account(accounts, 5)?;
    let wallet_ata = account(accounts, 6)?;
    let rent_receiver = account(accounts, 7)?;
    let associated_token_program = account(accounts, 8)?;
    let token_program = account(accounts, 9)?;

    assert_signer(holder)?;
    assert_writable(wallet_ata)?;
    assert_writable(rent_receiver)?;
    assert_associated_token_program(associated_token_program)?;
    assert_token_program_any(token_program)?;

    let token_program_kind = token_program_kind_from_account(token_program)?;
    let global = load_global_config(program_id, global_config_account)?;
    assert_core_asset_owner_and_collection(holder, agent_asset, &global.collection)?;
    let _vault = load_vault_config(program_id, vault_config_account, agent_asset.address())?;
    let wallet = load_wallet(program_id, wallet_account, agent_asset.address())?;
    if wallet.index != ix.index {
        return Err(AgentVaultError::InvalidWallet.into());
    }
    if !wallet.is_active() && !wallet.is_recovery_only() {
        return Err(AgentVaultError::WalletInactive.into());
    }

    {
        assert_owned_by(mint, token_program.address())?;
        assert_owned_by(wallet_ata, token_program.address())?;
        if mint.address() == &NATIVE_MINT_ID {
            return Err(AgentVaultError::InvalidWsolAccount.into());
        }
        assert_associated_token_address(
            wallet_ata,
            wallet_account.address(),
            mint.address(),
            token_program.address(),
        )?;
        let mint_data = mint.try_borrow()?;
        let parsed_mint = parse_mint(&mint_data, token_program_kind)?;
        let ata_data = wallet_ata.try_borrow()?;
        let ata = parse_token_account_for_mint(&ata_data, token_program_kind, &parsed_mint)?;
        validate_source_wallet_token_account(
            &ata,
            &address_bytes(wallet_account.address()),
            mint.address(),
        )?;
        if ata.amount != 0 || !ata.extensions.is_closable() {
            return Err(AgentVaultError::InvalidTokenAccount.into());
        }
    }

    invoke_token_close_account(
        token_program,
        wallet_ata,
        rent_receiver,
        wallet_account,
        agent_asset.address(),
        &wallet,
    )
}

#[inline(never)]
fn process_execute_cpi_checked(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &crate::instruction::ExecuteCpiChecked<'_>,
) -> ProgramResult {
    require_account_count(accounts, 6)?;
    let holder = account(accounts, 0)?;
    let global_config_account = account(accounts, 1)?;
    let vault_config_account = account(accounts, 2)?;
    let wallet_account = account(accounts, 3)?;
    let agent_asset = account(accounts, 4)?;
    let target_program = account(accounts, 5)?;
    let remaining_accounts = &accounts[6..];

    assert_signer(holder)?;
    if wallet_account.is_writable() {
        return Err(AgentVaultError::InvalidWritable.into());
    }
    if !target_program.executable() {
        return Err(AgentVaultError::InvalidCpiTarget.into());
    }
    if target_program.address() == program_id
        || is_loader_program(target_program.address())
        || target_program.address() == &TOKEN_PROGRAM_ID
        || target_program.address() == &TOKEN_2022_PROGRAM_ID
        || target_program.address() == &ASSOCIATED_TOKEN_PROGRAM_ID
    {
        return Err(AgentVaultError::InvalidCpiTarget.into());
    }

    let global = load_global_config(program_id, global_config_account)?;
    assert_core_asset_owner_and_collection(holder, agent_asset, &global.collection)?;
    let _vault = load_vault_config(program_id, vault_config_account, agent_asset.address())?;
    let wallet = load_wallet(program_id, wallet_account, agent_asset.address())?;
    if wallet.index != ix.index {
        return Err(AgentVaultError::InvalidWallet.into());
    }
    if wallet.is_recovery_only() {
        return Err(AgentVaultError::WalletRecoveryOnly.into());
    }
    if !wallet.is_active() {
        return Err(AgentVaultError::WalletInactive.into());
    }
    if remaining_accounts.len() != ix.target_account_count as usize {
        return Err(AgentVaultError::InvalidCpiAccounts.into());
    }

    let wallet_key = address_bytes(wallet_account.address());

    if ix.target_account_count == 0 {
        validate_execute_cpi_post_check_indexes(ix, &[], &wallet_key)?;
        let final_views = [wallet_account];
        let final_metas = [InstructionAccount::readonly_signer(
            wallet_account.address(),
        )];
        return execute_checked_cpi_with_final_accounts(
            CheckedCpiContext {
                program_id,
                ix,
                wallet_account,
                wallet_key: &wallet_key,
                agent_asset: agent_asset.address(),
                target_program,
                wallet: &wallet,
            },
            &final_views,
            &final_metas,
        );
    }

    let remaining_metas = build_cpi_plan_metas(remaining_accounts)?;
    let protected = ProtectedAccounts {
        holder: address_bytes(holder.address()),
        global_config: address_bytes(global_config_account.address()),
        vault_config: address_bytes(vault_config_account.address()),
        agent_asset: address_bytes(agent_asset.address()),
        target_program: address_bytes(target_program.address()),
    };
    validate_execute_cpi_plan(ix, &remaining_metas, &protected, &wallet_key)?;

    let final_count = ix.target_account_count as usize + 1;
    let mut final_views: Vec<&AccountView> = Vec::new();
    final_views
        .try_reserve_exact(final_count)
        .map_err(|_| AgentVaultError::AccountLimitExceeded)?;
    let mut final_metas: Vec<InstructionAccount> = Vec::new();
    final_metas
        .try_reserve_exact(final_count)
        .map_err(|_| AgentVaultError::AccountLimitExceeded)?;
    build_final_cpi_accounts(
        ix,
        wallet_account,
        remaining_accounts,
        &mut final_views,
        &mut final_metas,
    )?;
    execute_checked_cpi_with_final_accounts(
        CheckedCpiContext {
            program_id,
            ix,
            wallet_account,
            wallet_key: &wallet_key,
            agent_asset: agent_asset.address(),
            target_program,
            wallet: &wallet,
        },
        &final_views[..final_count],
        &final_metas[..final_count],
    )
}

struct CheckedCpiContext<'a, 'ix, 'data> {
    program_id: &'a Address,
    ix: &'ix crate::instruction::ExecuteCpiChecked<'data>,
    wallet_account: &'a AccountView,
    wallet_key: &'a [u8; PUBKEY_LEN],
    agent_asset: &'a Address,
    target_program: &'a AccountView,
    wallet: &'a AgentWallet,
}

fn execute_checked_cpi_with_final_accounts<'a>(
    ctx: CheckedCpiContext<'a, '_, '_>,
    final_views: &[&'a AccountView],
    final_metas: &[InstructionAccount<'a>],
) -> ProgramResult {
    let mut pre_values = [0u64; crate::constants::MAX_POST_CHECKS as usize];
    let mut custody_snapshots =
        [CustodySnapshot::EMPTY; crate::constants::MAX_POST_CHECKS as usize];
    snapshot_post_checks(ctx.ix, final_views, &mut pre_values, &mut custody_snapshots)?;
    enforce_wallet_controlled_token_checks(ctx.ix, final_views, final_metas, ctx.wallet_key)?;
    enforce_writable_non_token_owner_checks(ctx.ix, final_views, final_metas)?;

    let index_seed = agent_wallet_index_seed(ctx.wallet.index);
    let bump_seed = [ctx.wallet.bump];
    let seeds = [
        Seed::from(SEED_AGENT_WALLET),
        Seed::from(ctx.agent_asset.as_ref()),
        Seed::from(&index_seed),
        Seed::from(&bump_seed),
    ];
    let signer = Signer::from(&seeds);
    let instruction = InstructionView {
        program_id: ctx.target_program.address(),
        accounts: final_metas,
        data: ctx.ix.target_ix_data,
    };
    invoke_signed_with_slice(&instruction, final_views, &[signer])?;

    assert_owned_by(ctx.wallet_account, ctx.program_id)?;
    if ctx.wallet_account.data_len() != WALLET_LEN {
        return Err(AgentVaultError::InvalidWallet.into());
    }
    let rent_floor = Rent::get()?.try_minimum_balance(WALLET_LEN)?;
    if ctx.wallet_account.lamports() < rent_floor {
        return Err(AgentVaultError::RentFloorViolation.into());
    }
    enforce_wallet_controlled_token_checks(ctx.ix, final_views, final_metas, ctx.wallet_key)?;
    evaluate_post_checks(ctx.ix, final_views, &pre_values, &custody_snapshots)
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct CustodySnapshot {
    exists: bool,
    token_program_kind: TokenProgramKind,
    mint: [u8; PUBKEY_LEN],
    authority: [u8; PUBKEY_LEN],
    close_authority: OptionalPubkey,
    delegate: OptionalPubkey,
    delegated_amount: u64,
    state: u8,
    extension_data_hash: [u8; 32],
}

impl CustodySnapshot {
    const EMPTY: Self = Self {
        exists: false,
        token_program_kind: TokenProgramKind::Tokenkeg,
        mint: [0u8; PUBKEY_LEN],
        authority: [0u8; PUBKEY_LEN],
        close_authority: OptionalPubkey::None,
        delegate: OptionalPubkey::None,
        delegated_amount: 0,
        state: 0,
        extension_data_hash: [0u8; 32],
    };
}

fn final_account<'a>(
    final_accounts: &'a [&'a AccountView],
    index: u8,
) -> Result<&'a AccountView, ProgramError> {
    final_accounts
        .get(index as usize)
        .copied()
        .ok_or(AgentVaultError::InvalidPostCheck.into())
}

fn read_checked_token_amount(
    token_account: &AccountView,
    mint_account: &AccountView,
    expected_mint: &[u8; PUBKEY_LEN],
) -> Result<u64, ProgramError> {
    let kind = token_program_kind_from_owner(token_account)?;
    if !mint_account.owned_by(token_program_address(kind)) {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }
    if mint_account.address().as_ref() != expected_mint {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }
    let mint_data = mint_account.try_borrow()?;
    let mint = parse_mint(&mint_data, kind)?;
    let token_data = token_account.try_borrow()?;
    let token = parse_token_account_for_mint(&token_data, kind, &mint)?;
    if token.mint != *expected_mint {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }
    let native_mint = address_bytes(&NATIVE_MINT_ID);
    if token.mint == native_mint {
        let reserve = token
            .native_reserve
            .value()
            .ok_or(AgentVaultError::InvalidTokenAccount)?;
        return token_account
            .lamports()
            .checked_sub(reserve)
            .ok_or(AgentVaultError::InvalidTokenAccount.into());
    }
    if token.native_reserve.is_some() {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }
    Ok(token.amount)
}

fn snapshot_token_custody(
    token_account: &AccountView,
    mint_account: &AccountView,
) -> Result<CustodySnapshot, ProgramError> {
    let kind = token_program_kind_from_owner(token_account)?;
    snapshot_token_custody_with_kind(token_account, mint_account, kind)
}

fn snapshot_token_custody_with_kind(
    token_account: &AccountView,
    mint_account: &AccountView,
    kind: TokenProgramKind,
) -> Result<CustodySnapshot, ProgramError> {
    if !token_account.owned_by(token_program_address(kind))
        || !mint_account.owned_by(token_program_address(kind))
    {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }
    let mint_data = mint_account.try_borrow()?;
    let mint = parse_mint(&mint_data, kind)?;
    let token_data = token_account.try_borrow()?;
    let token = parse_token_account_for_mint(&token_data, kind, &mint)?;
    if token.mint != address_bytes(mint_account.address()) {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }
    Ok(CustodySnapshot {
        exists: true,
        token_program_kind: kind,
        mint: token.mint,
        authority: token.authority,
        close_authority: token.close_authority,
        delegate: token.delegate,
        delegated_amount: token.delegated_amount,
        state: token.state,
        extension_data_hash: canonical_token_extension_hash(&token.extensions),
    })
}

fn canonical_token_extension_hash(
    policy: &crate::token_state::TokenAccountExtensionPolicy<'_>,
) -> [u8; 32] {
    let mut inputs: [&[u8]; 6] = [&[]; 6];
    let mut input_count = 0usize;

    let entry0 = policy.canonical_entry(0);
    let entry1 = policy.canonical_entry(1);
    let type0 = entry0
        .map(|entry| entry.extension_type.to_le_bytes())
        .unwrap_or([0u8; 2]);
    let len0 = entry0
        .map(|entry| (entry.payload.len() as u16).to_le_bytes())
        .unwrap_or([0u8; 2]);
    let type1 = entry1
        .map(|entry| entry.extension_type.to_le_bytes())
        .unwrap_or([0u8; 2]);
    let len1 = entry1
        .map(|entry| (entry.payload.len() as u16).to_le_bytes())
        .unwrap_or([0u8; 2]);

    if let Some(entry) = entry0 {
        inputs[input_count] = &type0;
        input_count += 1;
        inputs[input_count] = &len0;
        input_count += 1;
        inputs[input_count] = entry.payload;
        input_count += 1;
    }
    if let Some(entry) = entry1 {
        inputs[input_count] = &type1;
        input_count += 1;
        inputs[input_count] = &len1;
        input_count += 1;
        inputs[input_count] = entry.payload;
        input_count += 1;
    }

    let hash = solana_sha256_hasher::hashv(&inputs[..input_count]);
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_ref());
    out
}

fn token_program_kind_from_owner(account: &AccountView) -> Result<TokenProgramKind, ProgramError> {
    if account.owned_by(&TOKEN_PROGRAM_ID) {
        Ok(TokenProgramKind::Tokenkeg)
    } else if account.owned_by(&TOKEN_2022_PROGRAM_ID) {
        Ok(TokenProgramKind::Token2022)
    } else {
        Err(AgentVaultError::InvalidTokenAccount.into())
    }
}

fn build_cpi_plan_metas(accounts: &[AccountView]) -> Result<Vec<CpiAccountMeta>, ProgramError> {
    if accounts.len() > MAX_CPI_ACCOUNTS as usize {
        return Err(AgentVaultError::AccountLimitExceeded.into());
    }
    let mut metas = Vec::new();
    metas
        .try_reserve_exact(accounts.len())
        .map_err(|_| AgentVaultError::AccountLimitExceeded)?;
    let mut i = 0;
    while i < accounts.len() {
        metas.push(CpiAccountMeta::new(
            address_bytes(accounts[i].address()),
            accounts[i].is_signer(),
            accounts[i].is_writable(),
        ));
        i += 1;
    }
    Ok(metas)
}

fn build_final_cpi_accounts<'a>(
    ix: &crate::instruction::ExecuteCpiChecked<'_>,
    wallet: &'a AccountView,
    remaining_accounts: &'a [AccountView],
    final_views: &mut Vec<&'a AccountView>,
    final_metas: &mut Vec<InstructionAccount<'a>>,
) -> ProgramResult {
    let final_count = ix.target_account_count as usize + 1;
    let mut final_index = 0usize;
    while final_index < final_count {
        if final_index == ix.wallet_meta_index as usize {
            final_views.push(wallet);
            final_metas.push(InstructionAccount::readonly_signer(wallet.address()));
        } else {
            let remaining_index = if final_index < ix.wallet_meta_index as usize {
                final_index
            } else {
                final_index - 1
            };
            let account = remaining_accounts
                .get(remaining_index)
                .ok_or(AgentVaultError::InvalidCpiAccounts)?;
            final_views.push(account);
            final_metas.push(InstructionAccount::new(
                account.address(),
                account.is_writable(),
                account.is_signer(),
            ));
        }
        final_index += 1;
    }
    Ok(())
}

fn enforce_wallet_controlled_token_checks(
    ix: &crate::instruction::ExecuteCpiChecked<'_>,
    final_accounts: &[&AccountView],
    final_metas: &[InstructionAccount<'_>],
    wallet_key: &[u8; PUBKEY_LEN],
) -> ProgramResult {
    let mut i = 0usize;
    let mut coverage = WalletTokenPostCheckCoverage::EMPTY;
    let mut coverage_loaded = false;
    while i < final_accounts.len() {
        if final_metas[i].is_writable {
            if let Some((mint, kind)) = read_wallet_controlled_token(final_accounts[i], wallet_key)?
            {
                let token_index = i as u8;
                if !coverage_loaded {
                    coverage = wallet_token_post_check_coverage(ix)?;
                    coverage_loaded = true;
                }
                if !coverage.has_economic(token_index) || !coverage.has_custody(token_index) {
                    return Err(AgentVaultError::MissingCustodyPostCheck.into());
                }
                let expected_ata = derive_associated_token_account(
                    &Address::new_from_array(*wallet_key),
                    &Address::new_from_array(mint),
                    token_program_address(kind),
                )?;
                if final_accounts[i].address() != &expected_ata.address {
                    return Err(AgentVaultError::InvalidAta.into());
                }
            }
        }
        i += 1;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct WalletTokenPostCheckCoverage {
    economic_mask: u128,
    custody_mask: u128,
}

impl WalletTokenPostCheckCoverage {
    const EMPTY: Self = Self {
        economic_mask: 0,
        custody_mask: 0,
    };

    #[inline(always)]
    fn has_economic(&self, token_index: u8) -> bool {
        self.economic_mask & account_index_mask(token_index) != 0
    }

    #[inline(always)]
    fn has_custody(&self, token_index: u8) -> bool {
        self.custody_mask & account_index_mask(token_index) != 0
    }
}

fn wallet_token_post_check_coverage(
    ix: &crate::instruction::ExecuteCpiChecked<'_>,
) -> Result<WalletTokenPostCheckCoverage, ProgramError> {
    let mut coverage = WalletTokenPostCheckCoverage::EMPTY;
    let mut checks = ix.post_checks();
    while let Some(check) = checks.next_check()? {
        match check {
            PostCheck::TokenBalanceMin {
                token_account_index,
                ..
            }
            | PostCheck::TokenBalanceMax {
                token_account_index,
                ..
            }
            | PostCheck::TokenIncreaseMin {
                token_account_index,
                ..
            }
            | PostCheck::TokenDecreaseMax {
                token_account_index,
                ..
            } => {
                coverage.economic_mask |= account_index_mask(token_account_index);
            }
            PostCheck::TokenCustodyUnchanged {
                token_account_index,
                ..
            }
            | PostCheck::TokenCustodyEquals {
                token_account_index,
                ..
            } => {
                coverage.custody_mask |= account_index_mask(token_account_index);
            }
            _ => {}
        }
    }
    Ok(coverage)
}

fn enforce_writable_non_token_owner_checks(
    ix: &crate::instruction::ExecuteCpiChecked<'_>,
    final_accounts: &[&AccountView],
    final_metas: &[InstructionAccount<'_>],
) -> ProgramResult {
    let mut i = 0usize;
    while i < final_accounts.len() {
        if final_metas[i].is_writable
            && !final_accounts[i].owned_by(&TOKEN_PROGRAM_ID)
            && !final_accounts[i].owned_by(&TOKEN_2022_PROGRAM_ID)
        {
            let owner = account_owner_bytes(final_accounts[i]);
            if !post_checks_cover_account_state(ix, i as u8, &owner)? {
                return Err(AgentVaultError::MissingCustodyPostCheck.into());
            }
        }
        i += 1;
    }
    Ok(())
}

fn read_wallet_controlled_token(
    account: &AccountView,
    wallet_key: &[u8; PUBKEY_LEN],
) -> Result<Option<([u8; PUBKEY_LEN], TokenProgramKind)>, ProgramError> {
    let kind = if account.owned_by(&TOKEN_PROGRAM_ID) {
        TokenProgramKind::Tokenkeg
    } else if account.owned_by(&TOKEN_2022_PROGRAM_ID) {
        TokenProgramKind::Token2022
    } else {
        return Ok(None);
    };
    let data = account.try_borrow()?;
    let token = crate::token_state::parse_token_account(&data, kind, true)?;
    if token_account_has_wallet_custody(&token, wallet_key) {
        Ok(Some((token.mint, kind)))
    } else {
        Ok(None)
    }
}

fn post_checks_cover_account_state(
    ix: &crate::instruction::ExecuteCpiChecked<'_>,
    account_index: u8,
    owner: &[u8; PUBKEY_LEN],
) -> Result<bool, ProgramError> {
    let mut checks = ix.post_checks();
    while let Some(check) = checks.next_check()? {
        if let PostCheck::AccountStateEquals {
            account_index: checked_index,
            owner: checked_owner,
            ..
        } = check
        {
            if checked_index == account_index && checked_owner == *owner {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[inline(always)]
fn account_index_mask(index: u8) -> u128 {
    1u128 << (index as u32)
}

fn snapshot_post_checks(
    ix: &crate::instruction::ExecuteCpiChecked<'_>,
    final_accounts: &[&AccountView],
    pre_values: &mut [u64; crate::constants::MAX_POST_CHECKS as usize],
    custody_snapshots: &mut [CustodySnapshot; crate::constants::MAX_POST_CHECKS as usize],
) -> ProgramResult {
    let mut checks = ix.post_checks();
    let mut i = 0usize;
    while let Some(check) = checks.next_check()? {
        pre_values[i] = match check {
            PostCheck::SolIncreaseMin { account_index, .. }
            | PostCheck::SolDecreaseMax { account_index, .. } => {
                final_account(final_accounts, account_index)?.lamports()
            }
            PostCheck::TokenIncreaseMin {
                token_account_index,
                mint_account_index,
                mint,
                ..
            }
            | PostCheck::TokenDecreaseMax {
                token_account_index,
                mint_account_index,
                mint,
                ..
            } => read_checked_token_amount(
                final_account(final_accounts, token_account_index)?,
                final_account(final_accounts, mint_account_index)?,
                &mint,
            )?,
            _ => 0,
        };

        if let PostCheck::TokenCustodyUnchanged {
            token_account_index,
            mint_account_index,
        } = check
        {
            custody_snapshots[i] = snapshot_token_custody(
                final_account(final_accounts, token_account_index)?,
                final_account(final_accounts, mint_account_index)?,
            )?;
        }
        i += 1;
    }
    Ok(())
}

fn evaluate_post_checks(
    ix: &crate::instruction::ExecuteCpiChecked<'_>,
    final_accounts: &[&AccountView],
    pre_values: &[u64; crate::constants::MAX_POST_CHECKS as usize],
    custody_snapshots: &[CustodySnapshot; crate::constants::MAX_POST_CHECKS as usize],
) -> ProgramResult {
    let mut checks = ix.post_checks();
    let mut i = 0usize;
    while let Some(check) = checks.next_check()? {
        match check {
            PostCheck::SolBalanceMin {
                account_index,
                min_lamports,
            } => {
                if final_account(final_accounts, account_index)?.lamports() < min_lamports {
                    return Err(AgentVaultError::PostCheckFailed.into());
                }
            }
            PostCheck::SolBalanceMax {
                account_index,
                max_lamports,
            } => {
                if final_account(final_accounts, account_index)?.lamports() > max_lamports {
                    return Err(AgentVaultError::PostCheckFailed.into());
                }
            }
            PostCheck::SolIncreaseMin {
                account_index,
                min_lamports_increase,
            } => {
                let post = final_account(final_accounts, account_index)?.lamports();
                let increase = post
                    .checked_sub(pre_values[i])
                    .ok_or(AgentVaultError::PostCheckFailed)?;
                if increase < min_lamports_increase {
                    return Err(AgentVaultError::PostCheckFailed.into());
                }
            }
            PostCheck::SolDecreaseMax {
                account_index,
                max_lamports_decrease,
            } => {
                let post = final_account(final_accounts, account_index)?.lamports();
                let decrease = pre_values[i]
                    .checked_sub(post)
                    .ok_or(AgentVaultError::PostCheckFailed)?;
                if decrease > max_lamports_decrease {
                    return Err(AgentVaultError::PostCheckFailed.into());
                }
            }
            PostCheck::TokenBalanceMin {
                token_account_index,
                mint_account_index,
                mint,
                min_amount,
            } => {
                let amount = read_checked_token_amount(
                    final_account(final_accounts, token_account_index)?,
                    final_account(final_accounts, mint_account_index)?,
                    &mint,
                )?;
                if amount < min_amount {
                    return Err(AgentVaultError::PostCheckFailed.into());
                }
            }
            PostCheck::TokenBalanceMax {
                token_account_index,
                mint_account_index,
                mint,
                max_amount,
            } => {
                let amount = read_checked_token_amount(
                    final_account(final_accounts, token_account_index)?,
                    final_account(final_accounts, mint_account_index)?,
                    &mint,
                )?;
                if amount > max_amount {
                    return Err(AgentVaultError::PostCheckFailed.into());
                }
            }
            PostCheck::TokenIncreaseMin {
                token_account_index,
                mint_account_index,
                mint,
                min_amount_increase,
            } => {
                let post = read_checked_token_amount(
                    final_account(final_accounts, token_account_index)?,
                    final_account(final_accounts, mint_account_index)?,
                    &mint,
                )?;
                let increase = post
                    .checked_sub(pre_values[i])
                    .ok_or(AgentVaultError::PostCheckFailed)?;
                if increase < min_amount_increase {
                    return Err(AgentVaultError::PostCheckFailed.into());
                }
            }
            PostCheck::TokenDecreaseMax {
                token_account_index,
                mint_account_index,
                mint,
                max_amount_decrease,
            } => {
                let post = read_checked_token_amount(
                    final_account(final_accounts, token_account_index)?,
                    final_account(final_accounts, mint_account_index)?,
                    &mint,
                )?;
                let decrease = pre_values[i]
                    .checked_sub(post)
                    .ok_or(AgentVaultError::PostCheckFailed)?;
                if decrease > max_amount_decrease {
                    return Err(AgentVaultError::PostCheckFailed.into());
                }
            }
            PostCheck::TokenAuthorityEquals {
                token_account_index,
                authority,
            } => {
                let token_account = final_account(final_accounts, token_account_index)?;
                let kind = token_program_kind_from_owner(token_account)?;
                let data = token_account.try_borrow()?;
                let token = crate::token_state::parse_token_account(&data, kind, true)?;
                if token.authority != authority {
                    return Err(AgentVaultError::PostCheckFailed.into());
                }
            }
            PostCheck::TokenCustodyUnchanged {
                token_account_index,
                mint_account_index,
            } => {
                let snapshot = snapshot_token_custody(
                    final_account(final_accounts, token_account_index)?,
                    final_account(final_accounts, mint_account_index)?,
                )?;
                if snapshot != custody_snapshots[i] {
                    return Err(AgentVaultError::CustodyChanged.into());
                }
            }
            PostCheck::TokenCustodyEquals {
                token_account_index,
                mint_account_index,
                token_program_kind,
                mint,
                authority,
                close_authority,
                delegate,
                state,
                extension_data_hash,
            } => {
                let snapshot = snapshot_token_custody_with_kind(
                    final_account(final_accounts, token_account_index)?,
                    final_account(final_accounts, mint_account_index)?,
                    token_program_kind,
                )?;
                if snapshot.mint != mint
                    || snapshot.authority != authority
                    || snapshot.close_authority != close_authority
                    || snapshot.delegate != delegate
                    || snapshot.delegated_amount != 0
                    || snapshot.state != state
                    || snapshot.extension_data_hash != extension_data_hash
                {
                    return Err(AgentVaultError::PostCheckFailed.into());
                }
            }
            PostCheck::AccountOwnerEquals {
                account_index,
                owner,
            } => {
                if account_owner_bytes(final_account(final_accounts, account_index)?) != owner {
                    return Err(AgentVaultError::PostCheckFailed.into());
                }
            }
            PostCheck::AccountStateEquals {
                account_index,
                owner,
                lamports,
                data_len,
                data_hash,
            } => {
                let account = final_account(final_accounts, account_index)?;
                if account_owner_bytes(account) != owner
                    || account.lamports() != lamports
                    || account.data_len() as u32 != data_len
                    || account_data_hash(account)? != data_hash
                {
                    return Err(AgentVaultError::PostCheckFailed.into());
                }
            }
        }
        i += 1;
    }
    Ok(())
}

fn account_data_hash(account: &AccountView) -> Result<[u8; 32], ProgramError> {
    let data = account.try_borrow()?;
    let hash = solana_sha256_hasher::hashv(&[&data[..]]);
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_ref());
    Ok(out)
}

fn validate_source_wallet_token_account(
    account: &TokenAccount<'_>,
    wallet_key: &[u8; PUBKEY_LEN],
    mint: &Address,
) -> ProgramResult {
    if account.mint != address_bytes(mint) || account.authority != *wallet_key {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }
    if !optional_pubkey_is_none_or_equals(&account.close_authority, wallet_key) {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }
    if !account.delegate.is_none() {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }
    Ok(())
}

fn validate_destination_wallet_token_account(
    account: &TokenAccount<'_>,
    wallet_key: &[u8; PUBKEY_LEN],
) -> ProgramResult {
    if account.authority != *wallet_key {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }
    if !optional_pubkey_is_none_or_equals(&account.close_authority, wallet_key) {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }
    if !account.delegate.is_none() {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }
    Ok(())
}

fn token_account_has_wallet_custody(
    account: &TokenAccount<'_>,
    wallet_key: &[u8; PUBKEY_LEN],
) -> bool {
    account.authority == *wallet_key
        || optional_pubkey_equals(&account.close_authority, wallet_key)
        || optional_pubkey_equals(&account.delegate, wallet_key)
}

fn validate_wsol_ata(
    wallet_wsol_ata: &AccountView,
    wallet: &Address,
    token_program: &Address,
) -> ProgramResult {
    assert_owned_by(wallet_wsol_ata, token_program)?;
    assert_associated_token_address(wallet_wsol_ata, wallet, &NATIVE_MINT_ID, token_program)?;

    let data = wallet_wsol_ata.try_borrow()?;
    let account = parse_token_account_for_mint(
        &data,
        TokenProgramKind::Tokenkeg,
        &TokenMint {
            mint_authority: OptionalPubkey::None,
            supply: 0,
            decimals: 9,
            freeze_authority: OptionalPubkey::None,
            extensions: crate::token_state::MintExtensionPolicy::none(),
        },
    )?;
    let wallet_key = address_bytes(wallet);
    if account.mint != address_bytes(&NATIVE_MINT_ID) || account.authority != wallet_key {
        return Err(AgentVaultError::InvalidWsolAccount.into());
    }
    if !optional_pubkey_is_none_or_equals(&account.close_authority, &wallet_key)
        || !account.delegate.is_none()
        || account.native_reserve.is_none()
    {
        return Err(AgentVaultError::InvalidWsolAccount.into());
    }
    Ok(())
}

fn assert_associated_token_address(
    account: &AccountView,
    wallet: &Address,
    mint: &Address,
    token_program: &Address,
) -> ProgramResult {
    let ata = derive_associated_token_account(wallet, mint, token_program)?;
    if account.address() != &ata.address {
        return Err(AgentVaultError::InvalidAta.into());
    }
    Ok(())
}

fn assert_token_program_kind(token_program: &AccountView, kind: TokenProgramKind) -> ProgramResult {
    let expected = token_program_address(kind);
    if token_program.address() != expected {
        return Err(AgentVaultError::InvalidTokenProgram.into());
    }
    Ok(())
}

fn token_program_kind_from_account(
    token_program: &AccountView,
) -> Result<TokenProgramKind, ProgramError> {
    if token_program.address() == &TOKEN_PROGRAM_ID {
        Ok(TokenProgramKind::Tokenkeg)
    } else if token_program.address() == &TOKEN_2022_PROGRAM_ID {
        Ok(TokenProgramKind::Token2022)
    } else {
        Err(AgentVaultError::InvalidTokenProgram.into())
    }
}

#[inline(always)]
fn token_program_address(kind: TokenProgramKind) -> &'static Address {
    match kind {
        TokenProgramKind::Tokenkeg => &TOKEN_PROGRAM_ID,
        TokenProgramKind::Token2022 => &TOKEN_2022_PROGRAM_ID,
    }
}

fn optional_pubkey_equals(value: &OptionalPubkey, key: &[u8; PUBKEY_LEN]) -> bool {
    matches!(value, OptionalPubkey::Some(value) if value == key)
}

fn optional_pubkey_is_none_or_equals(value: &OptionalPubkey, key: &[u8; PUBKEY_LEN]) -> bool {
    matches!(value, OptionalPubkey::None)
        || matches!(value, OptionalPubkey::Some(value) if value == key)
}

fn invoke_create_ata(
    payer: &AccountView,
    wallet_ata: &AccountView,
    wallet: &AccountView,
    mint: &AccountView,
    system_program: &AccountView,
    token_program: &AccountView,
) -> ProgramResult {
    let metas = [
        InstructionAccount::writable_signer(payer.address()),
        InstructionAccount::writable(wallet_ata.address()),
        InstructionAccount::readonly(wallet.address()),
        InstructionAccount::readonly(mint.address()),
        InstructionAccount::readonly(system_program.address()),
        InstructionAccount::readonly(token_program.address()),
    ];
    let data = [0u8];
    let instruction = InstructionView {
        program_id: &ASSOCIATED_TOKEN_PROGRAM_ID,
        accounts: &metas,
        data: &data,
    };
    invoke_signed(
        &instruction,
        &[
            payer,
            wallet_ata,
            wallet,
            mint,
            system_program,
            token_program,
        ],
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
fn invoke_token_transfer_checked(
    token_program: &AccountView,
    source: &AccountView,
    mint: &AccountView,
    destination: &AccountView,
    wallet: &AccountView,
    agent_asset: &Address,
    wallet_state: &AgentWallet,
    amount: u64,
    decimals: u8,
    expected_fee: u64,
    use_transfer_fee: bool,
) -> ProgramResult {
    let index_seed = agent_wallet_index_seed(wallet_state.index);
    let bump_seed = [wallet_state.bump];
    let seeds = [
        Seed::from(SEED_AGENT_WALLET),
        Seed::from(agent_asset.as_ref()),
        Seed::from(&index_seed),
        Seed::from(&bump_seed),
    ];
    let signer = Signer::from(&seeds);
    let metas = [
        InstructionAccount::writable(source.address()),
        InstructionAccount::readonly(mint.address()),
        InstructionAccount::writable(destination.address()),
        InstructionAccount::readonly_signer(wallet.address()),
    ];
    let mut data = [0u8; 19];
    let data_len = if use_transfer_fee {
        data[0] = 26;
        data[1] = 1;
        data[2..10].copy_from_slice(&amount.to_le_bytes());
        data[10] = decimals;
        data[11..19].copy_from_slice(&expected_fee.to_le_bytes());
        19
    } else {
        data[0] = 12;
        data[1..9].copy_from_slice(&amount.to_le_bytes());
        data[9] = decimals;
        10
    };
    let instruction = InstructionView {
        program_id: token_program.address(),
        accounts: &metas,
        data: &data[..data_len],
    };
    invoke_signed(
        &instruction,
        &[source, mint, destination, wallet],
        &[signer],
    )
}

fn invoke_token_close_account(
    token_program: &AccountView,
    token_account: &AccountView,
    destination: &AccountView,
    wallet: &AccountView,
    agent_asset: &Address,
    wallet_state: &AgentWallet,
) -> ProgramResult {
    let index_seed = agent_wallet_index_seed(wallet_state.index);
    let bump_seed = [wallet_state.bump];
    let seeds = [
        Seed::from(SEED_AGENT_WALLET),
        Seed::from(agent_asset.as_ref()),
        Seed::from(&index_seed),
        Seed::from(&bump_seed),
    ];
    let signer = Signer::from(&seeds);
    let metas = [
        InstructionAccount::writable(token_account.address()),
        InstructionAccount::writable(destination.address()),
        InstructionAccount::readonly_signer(wallet.address()),
    ];
    let data = [9u8];
    let instruction = InstructionView {
        program_id: token_program.address(),
        accounts: &metas,
        data: &data,
    };
    invoke_signed(
        &instruction,
        &[token_account, destination, wallet],
        &[signer],
    )
}

fn load_global_config(
    program_id: &Address,
    account: &AccountView,
) -> Result<GlobalConfig, ProgramError> {
    assert_owned_by(account, program_id)?;
    let data = account.try_borrow()?;
    let bump = read_global_config_bump(&data)?;
    validate_global_config_pda(account.address(), bump, program_id)?;
    let config = unpack_global_config_after_header(&data, bump)?;
    Ok(config)
}

fn load_vault_config(
    program_id: &Address,
    account: &AccountView,
    agent_asset: &Address,
) -> Result<VaultConfig, ProgramError> {
    assert_owned_by(account, program_id)?;
    let data = account.try_borrow()?;
    let bump = read_vault_config_bump(&data)?;
    validate_vault_config_pda(account.address(), bump, program_id, agent_asset)?;
    let config = unpack_vault_config_after_header(&data, bump)?;
    Ok(config)
}

fn load_wallet(
    program_id: &Address,
    account: &AccountView,
    agent_asset: &Address,
) -> Result<AgentWallet, ProgramError> {
    assert_owned_by(account, program_id)?;
    let data = account.try_borrow()?;
    let wallet = unpack_wallet(&data)?;
    validate_agent_wallet_pda(
        account.address(),
        wallet.bump,
        program_id,
        agent_asset,
        wallet.index,
    )?;
    Ok(wallet)
}

fn create_wallet_account(
    program_id: &Address,
    payer: &AccountView,
    wallet_account: &AccountView,
    agent_asset: &Address,
    index: u16,
    bump: u8,
) -> ProgramResult {
    let index_seed = agent_wallet_index_seed(index);
    let bump_seed = [bump];
    let seeds = [
        Seed::from(SEED_AGENT_WALLET),
        Seed::from(agent_asset.as_ref()),
        Seed::from(&index_seed),
        Seed::from(&bump_seed),
    ];
    let signer = Signer::from(&seeds);
    pinocchio_system::create_account_with_minimum_balance_signed(
        wallet_account,
        WALLET_LEN,
        program_id,
        payer,
        None,
        &[signer],
    )
}

fn checked_system_transfer(from: &AccountView, to: &AccountView, lamports: u64) -> ProgramResult {
    if lamports == 0 {
        return Ok(());
    }
    pinocchio_system::instructions::Transfer { from, to, lamports }.invoke()
}

fn checked_lamport_move_preserving_floor(
    from: &AccountView,
    to: &AccountView,
    lamports: u64,
    from_floor: u64,
) -> ProgramResult {
    if from.address() == to.address() {
        return Err(AgentVaultError::DuplicateAccount.into());
    }
    if lamports == 0 {
        return Ok(());
    }
    let (remaining, to_lamports) =
        checked_lamport_move_result(from.lamports(), to.lamports(), lamports, from_floor)?;
    from.set_lamports(remaining);
    to.set_lamports(to_lamports);
    Ok(())
}

pub(crate) fn checked_lamport_move_result(
    from_lamports: u64,
    to_lamports: u64,
    lamports: u64,
    from_floor: u64,
) -> Result<(u64, u64), ProgramError> {
    let remaining = from_lamports
        .checked_sub(lamports)
        .ok_or(AgentVaultError::ArithmeticUnderflow)?;
    if remaining < from_floor {
        return Err(AgentVaultError::RentFloorViolation.into());
    }
    let credited = to_lamports
        .checked_add(lamports)
        .ok_or(AgentVaultError::ArithmeticOverflow)?;
    Ok((remaining, credited))
}

fn checked_close_account(account: &AccountView, receiver: &AccountView) -> ProgramResult {
    if account.address() == receiver.address() {
        return Err(AgentVaultError::DuplicateAccount.into());
    }
    let lamports = account.lamports();
    let receiver_lamports = receiver
        .lamports()
        .checked_add(lamports)
        .ok_or(AgentVaultError::ArithmeticOverflow)?;
    account.set_lamports(0);
    receiver.set_lamports(receiver_lamports);
    account.close()
}

fn rent_minimum(rent_sysvar: &AccountView, data_len: usize) -> Result<u64, ProgramError> {
    let rent = Rent::from_account_view(rent_sysvar)?;
    rent.try_minimum_balance(data_len)
}

fn require_account_count(accounts: &[AccountView], count: usize) -> Result<(), ProgramError> {
    if accounts.len() < count {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    Ok(())
}

fn account(accounts: &[AccountView], index: usize) -> Result<&AccountView, ProgramError> {
    accounts
        .get(index)
        .ok_or(ProgramError::NotEnoughAccountKeys)
}

#[inline(always)]
fn address_bytes(address: &Address) -> [u8; PUBKEY_LEN] {
    let mut out = [0u8; PUBKEY_LEN];
    out.copy_from_slice(address.as_ref());
    out
}

#[inline(always)]
fn account_owner_bytes(account: &AccountView) -> [u8; PUBKEY_LEN] {
    let mut out = [0u8; PUBKEY_LEN];
    // SAFETY: the owner bytes are copied immediately and no reference is kept
    // across any assign/close operation.
    out.copy_from_slice(unsafe { account.owner().as_ref() });
    out
}
