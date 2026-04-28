use {
    agent_vault::{
        agent_account::{
            AGENT_ACCOUNT_ASSET_OFFSET, AGENT_ACCOUNT_BUMP_OFFSET, AGENT_ACCOUNT_COLLECTION_OFFSET,
            AGENT_ACCOUNT_CREATOR_OFFSET, AGENT_ACCOUNT_DISCRIMINATOR, AGENT_ACCOUNT_MIN_LEN,
            AGENT_ACCOUNT_OWNER_OFFSET,
        },
        constants::{
            EXPECTED_ACTIVATION_FEE_LAMPORTS, LABEL_LEN, WALLET_FLAG_ACTIVE,
            WALLET_FLAG_RECOVERY_ONLY,
        },
        core_asset::{
            CORE_ASSET_COLLECTION_OFFSET, CORE_ASSET_COLLECTION_TAG,
            CORE_ASSET_COLLECTION_TAG_OFFSET, CORE_ASSET_MIN_LEN, CORE_ASSET_OWNER_OFFSET,
            CORE_ASSET_V1_KEY,
        },
        error::AgentVaultError,
        instruction::{
            TAG_CLOSE_WALLET, TAG_CLOSE_WALLET_ATA, TAG_CREATE_WALLET, TAG_CREATE_WALLET_ATA,
            TAG_DEPOSIT_SOL, TAG_EXECUTE_CPI_CHECKED, TAG_INITIALIZE_GLOBAL_CONFIG,
            TAG_INIT_VAULT_CONFIG, TAG_REOPEN_WALLET_FOR_RECOVERY, TAG_TRANSFER_SOL,
            TAG_TRANSFER_SPL, TAG_UNWRAP_SOL, TAG_UPDATE_WALLET_LABEL, TAG_WITHDRAW_SOL,
            TAG_WRAP_SOL,
        },
        state::{
            unpack_global_config, unpack_vault_config, unpack_wallet, GLOBAL_CONFIG_FEE_OFFSET,
            GLOBAL_CONFIG_LEN, GLOBAL_CONFIG_REGISTRY_PROGRAM_OFFSET,
            VAULT_CONFIG_WALLET_COUNT_OFFSET, VAULT_CONFIG_LEN, WALLET_LEN,
        },
    },
    litesvm::LiteSVM,
    solana_account::Account,
    solana_address::{address, Address},
    solana_instruction::{account_meta::AccountMeta, error::InstructionError, Instruction},
    solana_message::Message,
    solana_rent::Rent,
    solana_signature::Signature,
    solana_transaction::Transaction,
    solana_transaction_error::TransactionError,
    std::path::PathBuf,
};

const PROGRAM_ID: Address = address!("36u7KMBuxjExvU6V2nfTX5SnNdYMGUupFiYouLzrgpfW");
const INITIALIZER: Address = address!("2KmHw8VbShuz9xfj3ecEjBM5nPKR5BcYHRDSFfK1286t");
const REGISTRY_PROGRAM: Address = address!("8oo4J9tBB3Hna1jRQ3rWvJjojqM5DYTDJo5cejUuJy3C");
const COLLECTION: Address = address!("6CTyGPcn8dMwKEqgtvx2XCpkGUd7uqCVK6937RSM5bhA");
const FEE_TREASURY: Address = address!("EbHMHsePB6GYxjqgz9k2aC4NACx63vTeBXzXyHWFvqPK");
const METAPLEX_CORE_PROGRAM: Address = address!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");
const MEMO_PROGRAM: Address = address!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
const SYSTEM_PROGRAM: Address = address!("11111111111111111111111111111111");
const CLOCK_SYSVAR: Address = address!("SysvarC1ock11111111111111111111111111111111");
const RENT_SYSVAR: Address = address!("SysvarRent111111111111111111111111111111111");
const TOKEN_PROGRAM: Address = address!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const TOKEN_2022_PROGRAM: Address = address!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const ASSOCIATED_TOKEN_PROGRAM: Address = address!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const MOCK_AMM_PROGRAM: Address = Address::new_from_array([7u8; 32]);
const BPF_LOADER: Address = address!("BPFLoader2111111111111111111111111111111111");
const BPF_LOADER_DEPRECATED: Address = address!("BPFLoader1111111111111111111111111111111111");
const BPF_UPGRADEABLE_LOADER: Address = address!("BPFLoaderUpgradeab1e11111111111111111111111");
const NATIVE_LOADER: Address = address!("NativeLoader1111111111111111111111111111111");
const LOADER_V4: Address = address!("LoaderV411111111111111111111111111111111111");
const NATIVE_MINT: Address = address!("So11111111111111111111111111111111111111112");
const TOKEN_MINT_LEN: usize = 82;
const TOKEN_ACCOUNT_LEN: usize = 165;
const SHA256_EMPTY: [u8; 32] = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9,
    0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52,
    0xb8, 0x55,
];

fn program_so_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/deploy/agent_vault.so");
    path
}

fn mock_amm_so_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/deploy/mock_amm.so");
    path
}

fn global_config_pda() -> Address {
    Address::find_program_address(&[b"global_config"], &PROGRAM_ID).0
}

fn vault_config_pda(agent_asset: &Address) -> Address {
    Address::find_program_address(&[b"vault_config", agent_asset.as_ref()], &PROGRAM_ID).0
}

fn wallet_pda(agent_asset: &Address, index: u16) -> Address {
    Address::find_program_address(
        &[b"agent_vault", agent_asset.as_ref(), &index.to_le_bytes()],
        &PROGRAM_ID,
    )
    .0
}

fn ata_address(wallet: &Address, mint: &Address, token_program: &Address) -> Address {
    Address::find_program_address(
        &[wallet.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ASSOCIATED_TOKEN_PROGRAM,
    )
    .0
}

fn registry_agent_pda(agent_asset: &Address) -> (Address, u8) {
    Address::find_program_address(&[b"agent", agent_asset.as_ref()], &REGISTRY_PROGRAM)
}

fn rent_minimum(data_len: usize) -> u64 {
    Rent::default().minimum_balance(data_len)
}

fn token_account_rent() -> u64 {
    rent_minimum(TOKEN_ACCOUNT_LEN)
}

fn initialize_global_config_ix(fee_lamports: u64) -> Instruction {
    initialize_global_config_ix_with(
        INITIALIZER,
        REGISTRY_PROGRAM,
        COLLECTION,
        FEE_TREASURY,
        fee_lamports,
    )
}

fn initialize_global_config_ix_with(
    initializer: Address,
    registry_program: Address,
    collection: Address,
    fee_treasury: Address,
    fee_lamports: u64,
) -> Instruction {
    let global_config = global_config_pda();
    let mut data = Vec::with_capacity(105);
    data.push(TAG_INITIALIZE_GLOBAL_CONFIG);
    data.extend_from_slice(registry_program.as_ref());
    data.extend_from_slice(collection.as_ref());
    data.extend_from_slice(fee_treasury.as_ref());
    data.extend_from_slice(&fee_lamports.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(initializer, true),
            AccountMeta::new(global_config, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        ],
        data,
    }
}

fn send_unsigned_tx(svm: &mut LiteSVM, ix: Instruction) -> Result<u64, TransactionError> {
    let blockhash = svm.latest_blockhash();
    let message = Message::new_with_blockhash(&[ix], Some(&INITIALIZER), &blockhash);
    let signatures = vec![Signature::default(); message.header.num_required_signatures as usize];
    let tx = Transaction {
        message,
        signatures,
    };
    svm.send_transaction(tx)
        .map(|meta| meta.compute_units_consumed)
        .map_err(|err| err.err)
}

fn runtime() -> LiteSVM {
    let mut svm = LiteSVM::new().with_sigverify(false);
    svm.add_program_from_file(PROGRAM_ID, &program_so_path())
        .unwrap();
    svm.airdrop(&INITIALIZER, 10_000_000_000).unwrap();
    svm
}

fn install_executable_account(svm: &mut LiteSVM, program: Address) {
    svm.add_program_from_file(program, &program_so_path())
        .unwrap();
}

fn install_executable_marker_account(svm: &mut LiteSVM, program: Address) {
    svm.set_account(
        program,
        Account {
            lamports: 1_000_000,
            data: Vec::new(),
            owner: NATIVE_LOADER,
            executable: true,
            ..Default::default()
        },
    )
    .unwrap();
}

fn install_mock_amm(svm: &mut LiteSVM) {
    svm.add_program_from_file(MOCK_AMM_PROGRAM, &mock_amm_so_path())
        .unwrap();
}

fn initialize_global_config(svm: &mut LiteSVM) {
    send_unsigned_tx(
        svm,
        initialize_global_config_ix(EXPECTED_ACTIVATION_FEE_LAMPORTS),
    )
    .unwrap();
}

fn install_agent_fixture(svm: &mut LiteSVM, agent_asset: Address) -> (Address, Address) {
    let mut asset_data = vec![0u8; CORE_ASSET_MIN_LEN];
    asset_data[0] = CORE_ASSET_V1_KEY;
    asset_data[CORE_ASSET_OWNER_OFFSET..CORE_ASSET_OWNER_OFFSET + 32]
        .copy_from_slice(INITIALIZER.as_ref());
    asset_data[CORE_ASSET_COLLECTION_TAG_OFFSET] = CORE_ASSET_COLLECTION_TAG;
    asset_data[CORE_ASSET_COLLECTION_OFFSET..CORE_ASSET_COLLECTION_OFFSET + 32]
        .copy_from_slice(COLLECTION.as_ref());
    svm.set_account(
        agent_asset,
        Account {
            lamports: 1_000_000,
            data: asset_data,
            owner: METAPLEX_CORE_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();

    let (agent_account, bump) = registry_agent_pda(&agent_asset);
    let mut agent_data = vec![0u8; AGENT_ACCOUNT_MIN_LEN];
    agent_data[0..8].copy_from_slice(&AGENT_ACCOUNT_DISCRIMINATOR);
    agent_data[AGENT_ACCOUNT_COLLECTION_OFFSET..AGENT_ACCOUNT_COLLECTION_OFFSET + 32]
        .copy_from_slice(COLLECTION.as_ref());
    agent_data[AGENT_ACCOUNT_CREATOR_OFFSET..AGENT_ACCOUNT_CREATOR_OFFSET + 32]
        .copy_from_slice(INITIALIZER.as_ref());
    agent_data[AGENT_ACCOUNT_OWNER_OFFSET..AGENT_ACCOUNT_OWNER_OFFSET + 32]
        .copy_from_slice(INITIALIZER.as_ref());
    agent_data[AGENT_ACCOUNT_ASSET_OFFSET..AGENT_ACCOUNT_ASSET_OFFSET + 32]
        .copy_from_slice(agent_asset.as_ref());
    agent_data[AGENT_ACCOUNT_BUMP_OFFSET] = bump;
    svm.set_account(
        agent_account,
        Account {
            lamports: 1_000_000,
            data: agent_data,
            owner: REGISTRY_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();

    (vault_config_pda(&agent_asset), agent_account)
}

fn set_agent_asset_owner(svm: &mut LiteSVM, agent_asset: Address, owner: Address) {
    let mut account = svm.get_account(&agent_asset).unwrap();
    account.data[CORE_ASSET_OWNER_OFFSET..CORE_ASSET_OWNER_OFFSET + 32]
        .copy_from_slice(owner.as_ref());
    svm.set_account(agent_asset, account).unwrap();
}

fn set_agent_asset_collection(svm: &mut LiteSVM, agent_asset: Address, collection: Address) {
    let mut account = svm.get_account(&agent_asset).unwrap();
    account.data[CORE_ASSET_COLLECTION_OFFSET..CORE_ASSET_COLLECTION_OFFSET + 32]
        .copy_from_slice(collection.as_ref());
    svm.set_account(agent_asset, account).unwrap();
}

fn tokenkeg_mint_data(decimals: u8) -> Vec<u8> {
    let mut data = vec![0u8; TOKEN_MINT_LEN];
    data[44] = decimals;
    data[45] = 1;
    data
}

fn token_2022_transfer_fee_mint_data(decimals: u8, maximum_fee: u64, basis_points: u16) -> Vec<u8> {
    let mut data = vec![0u8; 166 + 4 + 108];
    data[44] = decimals;
    data[45] = 1;
    data[165] = 1;
    data[166..168].copy_from_slice(&1u16.to_le_bytes());
    data[168..170].copy_from_slice(&108u16.to_le_bytes());
    let payload = 170;
    data[payload + 72..payload + 80].copy_from_slice(&0u64.to_le_bytes());
    data[payload + 80..payload + 88].copy_from_slice(&maximum_fee.to_le_bytes());
    data[payload + 88..payload + 90].copy_from_slice(&basis_points.to_le_bytes());
    data[payload + 90..payload + 98].copy_from_slice(&0u64.to_le_bytes());
    data[payload + 98..payload + 106].copy_from_slice(&maximum_fee.to_le_bytes());
    data[payload + 106..payload + 108].copy_from_slice(&basis_points.to_le_bytes());
    data
}

fn token_account_data(mint: Address, authority: Address, amount: u64) -> Vec<u8> {
    let mut data = vec![0u8; TOKEN_ACCOUNT_LEN];
    data[0..32].copy_from_slice(mint.as_ref());
    data[32..64].copy_from_slice(authority.as_ref());
    data[64..72].copy_from_slice(&amount.to_le_bytes());
    data[108] = 1;
    data
}

fn token_account_data_with_delegate(
    mint: Address,
    authority: Address,
    amount: u64,
    delegate: Address,
) -> Vec<u8> {
    let mut data = token_account_data(mint, authority, amount);
    data[72..76].copy_from_slice(&[1, 0, 0, 0]);
    data[76..108].copy_from_slice(delegate.as_ref());
    data[121..129].copy_from_slice(&amount.to_le_bytes());
    data
}

fn token_account_data_with_close_authority(
    mint: Address,
    authority: Address,
    amount: u64,
    close_authority: Address,
) -> Vec<u8> {
    let mut data = token_account_data(mint, authority, amount);
    data[129..133].copy_from_slice(&[1, 0, 0, 0]);
    data[133..165].copy_from_slice(close_authority.as_ref());
    data
}

fn token_2022_account_data_with_withheld_fee(
    mint: Address,
    authority: Address,
    amount: u64,
    withheld_amount: u64,
) -> Vec<u8> {
    let mut data = vec![0u8; 166 + 4 + 8];
    data[0..165].copy_from_slice(&token_account_data(mint, authority, amount));
    data[165] = 2;
    data[166..168].copy_from_slice(&2u16.to_le_bytes());
    data[168..170].copy_from_slice(&8u16.to_le_bytes());
    data[170..178].copy_from_slice(&withheld_amount.to_le_bytes());
    data
}

fn transfer_fee_amount_extension_hash(withheld_amount: u64) -> [u8; 32] {
    let extension_type = 2u16.to_le_bytes();
    let payload_len = 8u16.to_le_bytes();
    let payload = withheld_amount.to_le_bytes();
    let hash = solana_sha256_hasher::hashv(&[&extension_type, &payload_len, &payload]);
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_ref());
    out
}

fn native_token_account_data(
    mint: Address,
    authority: Address,
    amount: u64,
    native_reserve: u64,
) -> Vec<u8> {
    let mut data = vec![0u8; TOKEN_ACCOUNT_LEN];
    data[0..32].copy_from_slice(mint.as_ref());
    data[32..64].copy_from_slice(authority.as_ref());
    data[64..72].copy_from_slice(&amount.to_le_bytes());
    data[108] = 1;
    data[109..113].copy_from_slice(&[1, 0, 0, 0]);
    data[113..121].copy_from_slice(&native_reserve.to_le_bytes());
    data
}

fn token_amount(svm: &LiteSVM, token_account: &Address) -> u64 {
    let account = svm.get_account(token_account).unwrap();
    u64::from_le_bytes(account.data[64..72].try_into().unwrap())
}

fn install_mint(svm: &mut LiteSVM, mint: Address, token_program: Address, decimals: u8) {
    svm.set_account(
        mint,
        Account {
            lamports: 1_000_000,
            data: tokenkeg_mint_data(decimals),
            owner: token_program,
            ..Default::default()
        },
    )
    .unwrap();
}

fn install_token_account(
    svm: &mut LiteSVM,
    token_account: Address,
    token_program: Address,
    data: Vec<u8>,
) {
    svm.set_account(
        token_account,
        Account {
            lamports: token_account_rent(),
            data,
            owner: token_program,
            ..Default::default()
        },
    )
    .unwrap();
}

fn init_vault_config_ix(
    agent_asset: Address,
    vault_config: Address,
    agent_account: Address,
) -> Instruction {
    init_vault_config_ix_with_holder(INITIALIZER, agent_asset, vault_config, agent_account)
}

fn init_vault_config_ix_with_holder(
    holder: Address,
    agent_asset: Address,
    vault_config: Address,
    agent_account: Address,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(holder, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new(vault_config, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(agent_account, false),
            AccountMeta::new(FEE_TREASURY, false),
            AccountMeta::new_readonly(CLOCK_SYSVAR, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        ],
        data: vec![TAG_INIT_VAULT_CONFIG],
    }
}

fn create_wallet_ix(agent_asset: Address, vault_config: Address, wallet: Address) -> Instruction {
    create_wallet_ix_with_holder(INITIALIZER, agent_asset, vault_config, wallet)
}

fn create_wallet_ix_with_holder(
    holder: Address,
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
) -> Instruction {
    let mut data = Vec::with_capacity(1 + LABEL_LEN);
    data.push(TAG_CREATE_WALLET);
    data.extend_from_slice(b"treasury");
    data.resize(1 + LABEL_LEN, 0);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(holder, true),
            AccountMeta::new(vault_config, false),
            AccountMeta::new(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        ],
        data,
    }
}

fn update_wallet_label_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
) -> Instruction {
    update_wallet_label_ix_with(agent_asset, vault_config, wallet, 0, b"ops")
}

fn update_wallet_label_ix_with(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    index: u16,
    label: &[u8],
) -> Instruction {
    let mut data = Vec::with_capacity(1 + 2 + LABEL_LEN);
    data.push(TAG_UPDATE_WALLET_LABEL);
    data.extend_from_slice(&index.to_le_bytes());
    data.extend_from_slice(label);
    data.resize(1 + 2 + LABEL_LEN, 0);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
        ],
        data,
    }
}

fn deposit_sol_ix(agent_asset: Address, wallet: Address, amount: u64) -> Instruction {
    deposit_sol_ix_with_funder(INITIALIZER, agent_asset, wallet, amount)
}

fn deposit_sol_ix_with_funder(
    funder: Address,
    agent_asset: Address,
    wallet: Address,
    amount: u64,
) -> Instruction {
    let mut data = Vec::with_capacity(9);
    data.push(TAG_DEPOSIT_SOL);
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(funder, true),
            AccountMeta::new(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        ],
        data,
    }
}

fn withdraw_sol_ix(
    agent_asset: Address,
    wallet: Address,
    destination: Address,
    amount: u64,
) -> Instruction {
    let mut data = Vec::with_capacity(11);
    data.push(TAG_WITHDRAW_SOL);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(INITIALIZER, true),
            AccountMeta::new(wallet, false),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(RENT_SYSVAR, false),
        ],
        data,
    }
}

fn transfer_sol_ix(
    agent_asset: Address,
    from_index: u16,
    from_wallet: Address,
    to_index: u16,
    to_wallet: Address,
    amount: u64,
) -> Instruction {
    let mut data = Vec::with_capacity(13);
    data.push(TAG_TRANSFER_SOL);
    data.extend_from_slice(&from_index.to_le_bytes());
    data.extend_from_slice(&to_index.to_le_bytes());
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new(from_wallet, false),
            AccountMeta::new(to_wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(RENT_SYSVAR, false),
        ],
        data,
    }
}

fn close_wallet_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    rent_receiver: Address,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new(wallet, false),
            AccountMeta::new(rent_receiver, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(RENT_SYSVAR, false),
        ],
        data: vec![TAG_CLOSE_WALLET],
    }
}

fn reopen_wallet_for_recovery_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
) -> Instruction {
    let mut data = Vec::with_capacity(1 + 2 + LABEL_LEN);
    data.push(TAG_REOPEN_WALLET_FOR_RECOVERY);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(b"recovery");
    data.resize(1 + 2 + LABEL_LEN, 0);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        ],
        data,
    }
}

fn wrap_sol_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    wallet_wsol_ata: Address,
    amount: u64,
) -> Instruction {
    let mut data = Vec::with_capacity(11);
    data.push(TAG_WRAP_SOL);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new(wallet_wsol_ata, false),
            AccountMeta::new_readonly(NATIVE_MINT, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM, false),
            AccountMeta::new_readonly(RENT_SYSVAR, false),
        ],
        data,
    }
}

fn unwrap_sol_ix(agent_asset: Address, vault_config: Address, wallet: Address) -> Instruction {
    let mut data = Vec::with_capacity(3);
    data.push(TAG_UNWRAP_SOL);
    data.extend_from_slice(&0u16.to_le_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new(ata_address(&wallet, &NATIVE_MINT, &TOKEN_PROGRAM), false),
            AccountMeta::new_readonly(TOKEN_PROGRAM, false),
        ],
        data,
    }
}

fn create_wallet_ata_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    mint: Address,
    token_program: Address,
    token_program_kind: u8,
) -> Instruction {
    let mut data = Vec::with_capacity(4);
    data.push(TAG_CREATE_WALLET_ATA);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(token_program_kind);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(ata_address(&wallet, &mint, &token_program), false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM, false),
            AccountMeta::new_readonly(token_program, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        ],
        data,
    }
}

fn transfer_spl_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    mint: Address,
    source_token_account: Address,
    destination_token_account: Address,
    token_program: Address,
    amount: u64,
    decimals: u8,
    expected_fee: u64,
) -> Instruction {
    let mut data = Vec::with_capacity(20);
    data.push(TAG_TRANSFER_SPL);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(decimals);
    data.extend_from_slice(&expected_fee.to_le_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(source_token_account, false),
            AccountMeta::new(destination_token_account, false),
            AccountMeta::new_readonly(token_program, false),
        ],
        data,
    }
}

fn close_wallet_ata_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    mint: Address,
    token_program: Address,
    rent_receiver: Address,
) -> Instruction {
    let mut data = Vec::with_capacity(3);
    data.push(TAG_CLOSE_WALLET_ATA);
    data.extend_from_slice(&0u16.to_le_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(ata_address(&wallet, &mint, &token_program), false),
            AccountMeta::new(rent_receiver, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM, false),
            AccountMeta::new_readonly(token_program, false),
        ],
        data,
    }
}

fn sync_native_ix(wallet_wsol_ata: Address) -> Instruction {
    Instruction {
        program_id: TOKEN_PROGRAM,
        accounts: vec![AccountMeta::new(wallet_wsol_ata, false)],
        data: vec![17],
    }
}

fn execute_cpi_checked_wallet_writable_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
) -> Instruction {
    let mut data = Vec::with_capacity(18);
    data.push(TAG_EXECUTE_CPI_CHECKED);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(0);
    data.push(0);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(1);
    data.push(0);
    data.push(0);
    data.extend_from_slice(&1u64.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        ],
        data,
    }
}

fn execute_cpi_checked_missing_economic_post_check_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
) -> Instruction {
    let mut data = Vec::with_capacity(1 + 6 + 1 + 34);
    data.push(TAG_EXECUTE_CPI_CHECKED);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(0);
    data.push(0);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(1);
    data.push(11);
    data.push(0);
    data.extend_from_slice(SYSTEM_PROGRAM.as_ref());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(MEMO_PROGRAM, false),
        ],
        data,
    }
}

fn execute_cpi_checked_with_target_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    target_program: Address,
    min_wallet_lamports: u64,
) -> Instruction {
    let mut ix =
        execute_cpi_checked_memo_ix(agent_asset, vault_config, wallet, min_wallet_lamports);
    ix.accounts[5] = AccountMeta::new_readonly(target_program, false);
    ix
}

fn execute_cpi_checked_noop_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    min_wallet_lamports: u64,
) -> Instruction {
    let mut data = Vec::with_capacity(1 + 6 + 1 + 1 + 10);
    data.push(TAG_EXECUTE_CPI_CHECKED);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(0);
    data.push(0);
    data.extend_from_slice(&1u16.to_le_bytes());
    data.push(0);
    data.push(1);
    data.push(0);
    data.push(0);
    data.extend_from_slice(&min_wallet_lamports.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(MOCK_AMM_PROGRAM, false),
        ],
        data,
    }
}

fn execute_cpi_checked_wsol_balance_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    wallet_wsol_ata: Address,
    min_wsol_amount: u64,
) -> Instruction {
    let memo = b"wsol-check";
    let mut data = Vec::with_capacity(1 + 6 + memo.len() + 1 + 43 + 3);
    data.push(TAG_EXECUTE_CPI_CHECKED);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(0);
    data.push(2);
    data.extend_from_slice(&(memo.len() as u16).to_le_bytes());
    data.extend_from_slice(memo);
    data.push(2);

    data.push(4);
    data.push(1);
    data.push(2);
    data.extend_from_slice(NATIVE_MINT.as_ref());
    data.extend_from_slice(&min_wsol_amount.to_le_bytes());

    data.push(9);
    data.push(1);
    data.push(2);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(MEMO_PROGRAM, false),
            AccountMeta::new(wallet_wsol_ata, true),
            AccountMeta::new_readonly(NATIVE_MINT, true),
        ],
        data,
    }
}

fn execute_cpi_checked_writable_token_missing_custody_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    token_account: Address,
    min_wallet_lamports: u64,
) -> Instruction {
    let memo = b"token-touch";
    let mut data = Vec::with_capacity(1 + 6 + memo.len() + 1 + 10);
    data.push(TAG_EXECUTE_CPI_CHECKED);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(0);
    data.push(1);
    data.extend_from_slice(&(memo.len() as u16).to_le_bytes());
    data.extend_from_slice(memo);
    data.push(1);
    data.push(0);
    data.push(0);
    data.extend_from_slice(&min_wallet_lamports.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(MEMO_PROGRAM, false),
            AccountMeta::new(token_account, false),
        ],
        data,
    }
}

fn execute_cpi_checked_writable_account_state_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    writable_account: Address,
    writable_account_owner: Address,
    include_state_check: bool,
    min_wallet_lamports: u64,
) -> Instruction {
    let memo = b"state-check";
    let mut data = Vec::with_capacity(1 + 6 + memo.len() + 1 + 10 + 78);
    data.push(TAG_EXECUTE_CPI_CHECKED);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(0);
    data.push(1);
    data.extend_from_slice(&(memo.len() as u16).to_le_bytes());
    data.extend_from_slice(memo);
    data.push(if include_state_check { 2 } else { 1 });
    data.push(0);
    data.push(0);
    data.extend_from_slice(&min_wallet_lamports.to_le_bytes());
    if include_state_check {
        data.push(12);
        data.push(1);
        data.extend_from_slice(writable_account_owner.as_ref());
        data.extend_from_slice(&1u64.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&SHA256_EMPTY);
    }

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(MEMO_PROGRAM, false),
            AccountMeta::new(writable_account, true),
        ],
        data,
    }
}

fn execute_cpi_checked_token_custody_equals_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    token_account: Address,
    mint: Address,
    min_amount: u64,
    expected_authority: Address,
) -> Instruction {
    let memo = b"token-custody";
    let mut data = Vec::with_capacity(1 + 6 + memo.len() + 1 + 43 + 167);
    data.push(TAG_EXECUTE_CPI_CHECKED);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(0);
    data.push(2);
    data.extend_from_slice(&(memo.len() as u16).to_le_bytes());
    data.extend_from_slice(memo);
    data.push(2);

    data.push(4);
    data.push(1);
    data.push(2);
    data.extend_from_slice(mint.as_ref());
    data.extend_from_slice(&min_amount.to_le_bytes());

    data.push(10);
    data.push(1);
    data.push(2);
    data.push(0);
    data.extend_from_slice(mint.as_ref());
    data.extend_from_slice(expected_authority.as_ref());
    data.push(0);
    data.extend_from_slice(&[0u8; 32]);
    data.push(0);
    data.extend_from_slice(&[0u8; 32]);
    data.push(1);
    data.extend_from_slice(&SHA256_EMPTY);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(MEMO_PROGRAM, false),
            AccountMeta::new(token_account, true),
            AccountMeta::new_readonly(mint, true),
        ],
        data,
    }
}

fn execute_cpi_checked_token_balance_min_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    token_account: Address,
    mint_account: Address,
    expected_mint: Address,
) -> Instruction {
    let memo = b"token-balance";
    let mut data = Vec::with_capacity(1 + 6 + memo.len() + 1 + 43);
    data.push(TAG_EXECUTE_CPI_CHECKED);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(0);
    data.push(2);
    data.extend_from_slice(&(memo.len() as u16).to_le_bytes());
    data.extend_from_slice(memo);
    data.push(1);
    data.push(4);
    data.push(1);
    data.push(2);
    data.extend_from_slice(expected_mint.as_ref());
    data.extend_from_slice(&0u64.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(MEMO_PROGRAM, false),
            AccountMeta::new_readonly(token_account, true),
            AccountMeta::new_readonly(mint_account, true),
        ],
        data,
    }
}

fn execute_cpi_checked_mutate_delegate_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    token_account: Address,
    mint: Address,
    delegate: Address,
) -> Instruction {
    let mut target_data = [0u8; 10];
    target_data[0] = 1;
    target_data[1..9].copy_from_slice(&1u64.to_le_bytes());
    target_data[9] = 6;

    let mut data = Vec::with_capacity(1 + 6 + target_data.len() + 1 + 43 + 3);
    data.push(TAG_EXECUTE_CPI_CHECKED);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(0);
    data.push(4);
    data.extend_from_slice(&(target_data.len() as u16).to_le_bytes());
    data.extend_from_slice(&target_data);
    data.push(2);
    data.push(4);
    data.push(1);
    data.push(2);
    data.extend_from_slice(mint.as_ref());
    data.extend_from_slice(&1u64.to_le_bytes());
    data.push(9);
    data.push(1);
    data.push(2);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(MOCK_AMM_PROGRAM, false),
            AccountMeta::new(token_account, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(delegate, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM, false),
        ],
        data,
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_cpi_checked_set_authority_to_wallet_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    token_account: Address,
    mint: Address,
    current_authority: Address,
    expected_authority: Address,
) -> Instruction {
    let target_data = [2u8];
    let mut data = Vec::with_capacity(1 + 6 + target_data.len() + 1 + 43 + 167);
    data.push(TAG_EXECUTE_CPI_CHECKED);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(0);
    data.push(4);
    data.extend_from_slice(&(target_data.len() as u16).to_le_bytes());
    data.extend_from_slice(&target_data);
    data.push(2);
    data.push(4);
    data.push(1);
    data.push(2);
    data.extend_from_slice(mint.as_ref());
    data.extend_from_slice(&5u64.to_le_bytes());
    data.push(10);
    data.push(1);
    data.push(2);
    data.push(0);
    data.extend_from_slice(mint.as_ref());
    data.extend_from_slice(expected_authority.as_ref());
    data.push(0);
    data.extend_from_slice(&[0u8; 32]);
    data.push(0);
    data.extend_from_slice(&[0u8; 32]);
    data.push(1);
    data.extend_from_slice(&SHA256_EMPTY);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(MOCK_AMM_PROGRAM, false),
            AccountMeta::new(token_account, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(current_authority, true),
            AccountMeta::new_readonly(TOKEN_PROGRAM, false),
        ],
        data,
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_cpi_checked_token_2022_fee_receive_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    source: Address,
    mint: Address,
    destination: Address,
    authority: Address,
    amount: u64,
    fee: u64,
    expected_hash: [u8; 32],
) -> Instruction {
    let mut target_data = [0u8; 18];
    target_data[0] = 3;
    target_data[1..9].copy_from_slice(&amount.to_le_bytes());
    target_data[9] = 6;
    target_data[10..18].copy_from_slice(&fee.to_le_bytes());

    let received = amount.saturating_sub(fee);
    let mut data = Vec::with_capacity(1 + 6 + target_data.len() + 1 + 43 + 167);
    data.push(TAG_EXECUTE_CPI_CHECKED);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(0);
    data.push(5);
    data.extend_from_slice(&(target_data.len() as u16).to_le_bytes());
    data.extend_from_slice(&target_data);
    data.push(2);
    data.push(6);
    data.push(3);
    data.push(2);
    data.extend_from_slice(mint.as_ref());
    data.extend_from_slice(&received.to_le_bytes());
    data.push(10);
    data.push(3);
    data.push(2);
    data.push(1);
    data.extend_from_slice(mint.as_ref());
    data.extend_from_slice(wallet.as_ref());
    data.push(0);
    data.extend_from_slice(&[0u8; 32]);
    data.push(0);
    data.extend_from_slice(&[0u8; 32]);
    data.push(1);
    data.extend_from_slice(&expected_hash);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(MOCK_AMM_PROGRAM, false),
            AccountMeta::new(source, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(TOKEN_2022_PROGRAM, false),
        ],
        data,
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_cpi_checked_mock_swap_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    input_mint: Address,
    output_mint: Address,
    user_input: Address,
    pool_input: Address,
    pool_output: Address,
    user_output: Address,
    amount_in: u64,
    max_input: u64,
    amount_out: u64,
    min_output: u64,
) -> Instruction {
    let mut target_data = [0u8; 18];
    target_data[0..8].copy_from_slice(&amount_in.to_le_bytes());
    target_data[8..16].copy_from_slice(&amount_out.to_le_bytes());
    target_data[16] = 6;
    target_data[17] = 6;

    let mut data = Vec::with_capacity(1 + 6 + target_data.len() + 1 + (43 * 3) + (3 * 2));
    data.push(TAG_EXECUTE_CPI_CHECKED);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(0);
    data.push(7);
    data.extend_from_slice(&(target_data.len() as u16).to_le_bytes());
    data.extend_from_slice(&target_data);
    data.push(5);

    data.push(7);
    data.push(1);
    data.push(5);
    data.extend_from_slice(input_mint.as_ref());
    data.extend_from_slice(&max_input.to_le_bytes());

    data.push(9);
    data.push(1);
    data.push(5);

    data.push(7);
    data.push(3);
    data.push(6);
    data.extend_from_slice(output_mint.as_ref());
    data.extend_from_slice(&amount_out.to_le_bytes());

    data.push(9);
    data.push(3);
    data.push(6);

    data.push(6);
    data.push(4);
    data.push(6);
    data.extend_from_slice(output_mint.as_ref());
    data.extend_from_slice(&min_output.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(INITIALIZER, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(MOCK_AMM_PROGRAM, false),
            AccountMeta::new(user_input, false),
            AccountMeta::new(pool_input, false),
            AccountMeta::new(pool_output, false),
            AccountMeta::new(user_output, false),
            AccountMeta::new_readonly(input_mint, false),
            AccountMeta::new_readonly(output_mint, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM, false),
        ],
        data,
    }
}

fn execute_cpi_checked_memo_ix(
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    min_wallet_lamports: u64,
) -> Instruction {
    execute_cpi_checked_memo_ix_with_holder(
        INITIALIZER,
        agent_asset,
        vault_config,
        wallet,
        min_wallet_lamports,
    )
}

fn execute_cpi_checked_memo_ix_with_holder(
    holder: Address,
    agent_asset: Address,
    vault_config: Address,
    wallet: Address,
    min_wallet_lamports: u64,
) -> Instruction {
    let memo = b"agent-vault";
    let mut data = Vec::with_capacity(18 + memo.len());
    data.push(TAG_EXECUTE_CPI_CHECKED);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.push(0);
    data.push(0);
    data.extend_from_slice(&(memo.len() as u16).to_le_bytes());
    data.extend_from_slice(memo);
    data.push(1);
    data.push(0);
    data.push(0);
    data.extend_from_slice(&min_wallet_lamports.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(holder, true),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(vault_config, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(agent_asset, false),
            AccountMeta::new_readonly(MEMO_PROGRAM, false),
        ],
        data,
    }
}

fn create_agent_vault(svm: &mut LiteSVM, agent_asset: Address) -> Address {
    let (vault_config, agent_account) = install_agent_fixture(svm, agent_asset);
    send_unsigned_tx(
        svm,
        init_vault_config_ix(agent_asset, vault_config, agent_account),
    )
    .unwrap();
    vault_config
}

fn create_agent_vault_and_wallet(svm: &mut LiteSVM, agent_asset: Address) -> (Address, Address) {
    let vault_config = create_agent_vault(svm, agent_asset);
    let wallet = wallet_pda(&agent_asset, 0);
    send_unsigned_tx(svm, create_wallet_ix(agent_asset, vault_config, wallet)).unwrap();
    (vault_config, wallet)
}

fn create_recovery_only_wallet(svm: &mut LiteSVM, agent_asset: Address) -> (Address, Address) {
    let (vault_config, wallet) = create_agent_vault_and_wallet(svm, agent_asset);
    let rent_receiver = Address::new_unique();
    svm.airdrop(&rent_receiver, 1).unwrap();
    send_unsigned_tx(
        svm,
        close_wallet_ix(agent_asset, vault_config, wallet, rent_receiver),
    )
    .unwrap();
    send_unsigned_tx(
        svm,
        reopen_wallet_for_recovery_ix(agent_asset, vault_config, wallet),
    )
    .unwrap();

    let wallet_account = svm.get_account(&wallet).unwrap();
    let decoded_wallet = unpack_wallet(&wallet_account.data).unwrap();
    assert_eq!(decoded_wallet.flags, WALLET_FLAG_RECOVERY_ONLY);
    (vault_config, wallet)
}

#[test]
fn initializes_global_config_from_devnet_manifest_constants() {
    let mut svm = runtime();
    let cu = send_unsigned_tx(
        &mut svm,
        initialize_global_config_ix(EXPECTED_ACTIVATION_FEE_LAMPORTS),
    )
    .unwrap();
    assert!(cu <= 18_000, "initialize_global_config CU: {cu}");

    let account = svm.get_account(&global_config_pda()).unwrap();
    assert_eq!(account.owner, PROGRAM_ID);
    assert_eq!(account.data.len(), GLOBAL_CONFIG_LEN);

    let decoded = unpack_global_config(&account.data).unwrap();
    assert_eq!(decoded.registry_program, *REGISTRY_PROGRAM.as_ref());
    assert_eq!(decoded.collection, *COLLECTION.as_ref());
    assert_eq!(decoded.fee_treasury, *FEE_TREASURY.as_ref());
    assert_eq!(
        decoded.vault_activation_fee_lamports,
        EXPECTED_ACTIVATION_FEE_LAMPORTS
    );
    assert_eq!(
        &account.data
            [GLOBAL_CONFIG_REGISTRY_PROGRAM_OFFSET..GLOBAL_CONFIG_REGISTRY_PROGRAM_OFFSET + 32],
        REGISTRY_PROGRAM.as_ref()
    );
    assert_eq!(
        u64::from_le_bytes(
            account.data[GLOBAL_CONFIG_FEE_OFFSET..GLOBAL_CONFIG_FEE_OFFSET + 8]
                .try_into()
                .unwrap()
        ),
        EXPECTED_ACTIVATION_FEE_LAMPORTS
    );
}

#[test]
fn rejects_non_manifest_activation_fee() {
    let mut svm = runtime();
    let error = send_unsigned_tx(&mut svm, initialize_global_config_ix(0)).unwrap_err();

    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidGlobalConfig as u32),
        )
    );
    assert!(svm.get_account(&global_config_pda()).is_none());
}

#[test]
fn rejects_non_manifest_global_config_fields_before_create() {
    for ix in [
        initialize_global_config_ix_with(
            Address::new_unique(),
            REGISTRY_PROGRAM,
            COLLECTION,
            FEE_TREASURY,
            EXPECTED_ACTIVATION_FEE_LAMPORTS,
        ),
        initialize_global_config_ix_with(
            INITIALIZER,
            Address::new_unique(),
            COLLECTION,
            FEE_TREASURY,
            EXPECTED_ACTIVATION_FEE_LAMPORTS,
        ),
        initialize_global_config_ix_with(
            INITIALIZER,
            REGISTRY_PROGRAM,
            Address::new_unique(),
            FEE_TREASURY,
            EXPECTED_ACTIVATION_FEE_LAMPORTS,
        ),
        initialize_global_config_ix_with(
            INITIALIZER,
            REGISTRY_PROGRAM,
            COLLECTION,
            Address::new_unique(),
            EXPECTED_ACTIVATION_FEE_LAMPORTS,
        ),
    ] {
        let mut svm = runtime();
        let error = send_unsigned_tx(&mut svm, ix).unwrap_err();
        assert!(matches!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(code),
            ) if code == AgentVaultError::InvalidSigner as u32
                || code == AgentVaultError::InvalidGlobalConfig as u32
        ));
        assert!(svm.get_account(&global_config_pda()).is_none());
    }
}

#[test]
fn initialize_global_config_is_immutable_once_created() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    let original_global = svm.get_account(&global_config_pda()).unwrap();

    let error = send_unsigned_tx(
        &mut svm,
        initialize_global_config_ix(EXPECTED_ACTIVATION_FEE_LAMPORTS),
    )
    .unwrap_err();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidOwner as u32),
        )
    );
    let current_global = svm.get_account(&global_config_pda()).unwrap();
    assert_eq!(current_global.owner, original_global.owner);
    assert_eq!(current_global.lamports, original_global.lamports);
    assert_eq!(current_global.data, original_global.data);
}

#[test]
fn global_config_init_handles_dusted_pdas_and_rejects_squatters() {
    let mut dusted = runtime();
    dusted
        .set_account(
            global_config_pda(),
            Account {
                lamports: 123_456,
                data: Vec::new(),
                owner: SYSTEM_PROGRAM,
                ..Default::default()
            },
        )
        .unwrap();
    send_unsigned_tx(
        &mut dusted,
        initialize_global_config_ix(EXPECTED_ACTIVATION_FEE_LAMPORTS),
    )
    .unwrap();
    assert_eq!(dusted.get_account(&global_config_pda()).unwrap().owner, PROGRAM_ID);

    let mut data_squatter = runtime();
    data_squatter
        .set_account(
            global_config_pda(),
            Account {
                lamports: 123_456,
                data: vec![1],
                owner: SYSTEM_PROGRAM,
                ..Default::default()
            },
        )
        .unwrap();
    let data_error = send_unsigned_tx(
        &mut data_squatter,
        initialize_global_config_ix(EXPECTED_ACTIVATION_FEE_LAMPORTS),
    )
    .unwrap_err();
    assert_eq!(
        data_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidAccountData as u32),
        )
    );

    let mut owner_squatter = runtime();
    owner_squatter
        .set_account(
            global_config_pda(),
            Account {
                lamports: 123_456,
                data: Vec::new(),
                owner: Address::new_unique(),
                ..Default::default()
            },
        )
        .unwrap();
    let owner_error = send_unsigned_tx(
        &mut owner_squatter,
        initialize_global_config_ix(EXPECTED_ACTIVATION_FEE_LAMPORTS),
    )
    .unwrap_err();
    assert_eq!(
        owner_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidOwner as u32),
        )
    );
}

#[test]
fn init_vault_config_is_one_time_fee() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, agent_account) = install_agent_fixture(&mut svm, agent_asset);
    send_unsigned_tx(
        &mut svm,
        init_vault_config_ix(agent_asset, vault_config, agent_account),
    )
    .unwrap();
    let treasury_after_first_init = svm.get_balance(&FEE_TREASURY).unwrap();
    assert_eq!(
        treasury_after_first_init,
        1 + EXPECTED_ACTIVATION_FEE_LAMPORTS
    );

    let error = send_unsigned_tx(
        &mut svm,
        init_vault_config_ix(agent_asset, vault_config, agent_account),
    )
    .unwrap_err();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidOwner as u32),
        )
    );
    assert_eq!(
        svm.get_balance(&FEE_TREASURY).unwrap(),
        treasury_after_first_init
    );
}

#[test]
fn routine_v0_instructions_do_not_charge_protocol_fees() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let vault_config = create_agent_vault(&mut svm, agent_asset);
    let fee_balance = svm.get_balance(&FEE_TREASURY).unwrap();
    let assert_no_fee = |svm: &LiteSVM| {
        assert_eq!(svm.get_balance(&FEE_TREASURY).unwrap(), fee_balance);
    };

    let wallet = wallet_pda(&agent_asset, 0);
    let create_wallet_cu = send_unsigned_tx(
        &mut svm,
        create_wallet_ix(agent_asset, vault_config, wallet),
    )
    .unwrap();
    assert!(create_wallet_cu <= 18_000, "create_wallet CU: {create_wallet_cu}");
    let wallet_rent_floor = svm.get_balance(&wallet).unwrap();
    assert_no_fee(&svm);

    let wallet_1 = wallet_pda(&agent_asset, 1);
    let create_wallet_1_cu = send_unsigned_tx(
        &mut svm,
        create_wallet_ix(agent_asset, vault_config, wallet_1),
    )
    .unwrap();
    assert!(
        create_wallet_1_cu <= 18_000,
        "create_wallet second CU: {create_wallet_1_cu}"
    );
    assert_no_fee(&svm);

    let update_label_cu = send_unsigned_tx(
        &mut svm,
        update_wallet_label_ix(agent_asset, vault_config, wallet),
    )
    .unwrap();
    assert!(
        update_label_cu <= 17_600,
        "update_wallet_label CU: {update_label_cu}"
    );
    assert_no_fee(&svm);

    let deposit_cu =
        send_unsigned_tx(&mut svm, deposit_sol_ix(agent_asset, wallet, 1_000_000)).unwrap();
    assert!(deposit_cu <= 10_700, "deposit_sol CU: {deposit_cu}");
    assert_no_fee(&svm);

    let destination = Address::new_unique();
    svm.airdrop(&destination, 1).unwrap();
    let withdraw_cu = send_unsigned_tx(
        &mut svm,
        withdraw_sol_ix(agent_asset, wallet, destination, 100_000),
    )
    .unwrap();
    assert!(withdraw_cu <= 9_700, "withdraw_sol CU: {withdraw_cu}");
    assert_no_fee(&svm);

    let transfer_sol_cu = send_unsigned_tx(
        &mut svm,
        transfer_sol_ix(agent_asset, 0, wallet, 1, wallet_1, 100_000),
    )
    .unwrap();
    assert!(
        transfer_sol_cu <= 15_000,
        "transfer_sol CU: {transfer_sol_cu}"
    );
    assert_no_fee(&svm);

    let mint = Address::new_unique();
    install_mint(&mut svm, mint, TOKEN_PROGRAM, 6);
    let create_ata_cu = send_unsigned_tx(
        &mut svm,
        create_wallet_ata_ix(agent_asset, vault_config, wallet, mint, TOKEN_PROGRAM, 0),
    )
    .unwrap();
    assert!(
        create_ata_cu <= 48_000,
        "create_wallet_ata CU: {create_ata_cu}"
    );
    assert_no_fee(&svm);

    let source_ata = ata_address(&wallet, &mint, &TOKEN_PROGRAM);
    let token_destination = Address::new_unique();
    install_token_account(
        &mut svm,
        source_ata,
        TOKEN_PROGRAM,
        token_account_data(mint, wallet, 25),
    );
    install_token_account(
        &mut svm,
        token_destination,
        TOKEN_PROGRAM,
        token_account_data(mint, Address::new_unique(), 0),
    );
    let transfer_spl_cu = send_unsigned_tx(
        &mut svm,
        transfer_spl_ix(
            agent_asset,
            vault_config,
            wallet,
            mint,
            source_ata,
            token_destination,
            TOKEN_PROGRAM,
            25,
            6,
            0,
        ),
    )
    .unwrap();
    assert!(
        transfer_spl_cu <= 31_500,
        "transfer_spl CU: {transfer_spl_cu}"
    );
    assert_no_fee(&svm);

    let rent_receiver = Address::new_unique();
    svm.airdrop(&rent_receiver, 1).unwrap();
    let close_ata_cu = send_unsigned_tx(
        &mut svm,
        close_wallet_ata_ix(
            agent_asset,
            vault_config,
            wallet,
            mint,
            TOKEN_PROGRAM,
            rent_receiver,
        ),
    )
    .unwrap();
    assert!(
        close_ata_cu <= 27_000,
        "close_wallet_ata CU: {close_ata_cu}"
    );
    assert_no_fee(&svm);

    let native_reserve = token_account_rent();
    let wallet_wsol_ata = ata_address(&wallet, &NATIVE_MINT, &TOKEN_PROGRAM);
    svm.set_account(
        wallet_wsol_ata,
        Account {
            lamports: native_reserve,
            data: native_token_account_data(NATIVE_MINT, wallet, 0, native_reserve),
            owner: TOKEN_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();
    let wrap_sol_cu = send_unsigned_tx(
        &mut svm,
        wrap_sol_ix(agent_asset, vault_config, wallet, wallet_wsol_ata, 250_000),
    )
    .unwrap();
    assert!(wrap_sol_cu <= 21_600, "wrap_sol CU: {wrap_sol_cu}");
    assert_no_fee(&svm);

    send_unsigned_tx(&mut svm, sync_native_ix(wallet_wsol_ata)).unwrap();
    let unwrap_sol_cu =
        send_unsigned_tx(&mut svm, unwrap_sol_ix(agent_asset, vault_config, wallet)).unwrap();
    assert!(unwrap_sol_cu <= 26_200, "unwrap_sol CU: {unwrap_sol_cu}");
    assert_no_fee(&svm);

    let current_wallet_balance = svm.get_balance(&wallet).unwrap();
    let execute_cpi_cu = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_memo_ix(agent_asset, vault_config, wallet, current_wallet_balance),
    )
    .unwrap();
    assert!(
        execute_cpi_cu <= 50_000,
        "execute_cpi_checked memo CU: {execute_cpi_cu}"
    );
    assert_no_fee(&svm);

    let excess_lamports = svm.get_balance(&wallet).unwrap() - wallet_rent_floor;
    send_unsigned_tx(
        &mut svm,
        withdraw_sol_ix(agent_asset, wallet, destination, excess_lamports),
    )
    .unwrap();
    assert_no_fee(&svm);

    let close_wallet_cu = send_unsigned_tx(
        &mut svm,
        close_wallet_ix(agent_asset, vault_config, wallet, rent_receiver),
    )
    .unwrap();
    assert!(
        close_wallet_cu <= 17_400,
        "close_wallet CU: {close_wallet_cu}"
    );
    assert_no_fee(&svm);

    let reopen_cu = send_unsigned_tx(
        &mut svm,
        reopen_wallet_for_recovery_ix(agent_asset, vault_config, wallet),
    )
    .unwrap();
    assert!(
        reopen_cu <= 19_600,
        "reopen_wallet_for_recovery CU: {reopen_cu}"
    );
    assert_no_fee(&svm);
}

#[test]
fn init_vault_config_rejects_malformed_registry_agent_account() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    for mutate in 0..4 {
        let agent_asset = Address::new_unique();
        let (vault_config, agent_account) = install_agent_fixture(&mut svm, agent_asset);
        let mut account = svm.get_account(&agent_account).unwrap();
        match mutate {
            0 => account.data[AGENT_ACCOUNT_COLLECTION_OFFSET..AGENT_ACCOUNT_COLLECTION_OFFSET + 32]
                .copy_from_slice(Address::new_unique().as_ref()),
            1 => account.data[AGENT_ACCOUNT_ASSET_OFFSET..AGENT_ACCOUNT_ASSET_OFFSET + 32]
                .copy_from_slice(Address::new_unique().as_ref()),
            2 => account.data[AGENT_ACCOUNT_BUMP_OFFSET] =
                account.data[AGENT_ACCOUNT_BUMP_OFFSET].wrapping_add(1),
            _ => account.data.truncate(AGENT_ACCOUNT_MIN_LEN - 1),
        }
        svm.set_account(agent_account, account).unwrap();

        let error = send_unsigned_tx(
            &mut svm,
            init_vault_config_ix(agent_asset, vault_config, agent_account),
        )
        .unwrap_err();
        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(AgentVaultError::InvalidAgentAccount as u32),
            )
        );
    }
}

#[test]
fn init_vault_config_checks_treasury_registry_owner_pda_and_dusted_pdas() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, agent_account) = install_agent_fixture(&mut svm, agent_asset);

    let wrong_treasury = Address::new_unique();
    svm.airdrop(&wrong_treasury, 1).unwrap();
    let mut wrong_treasury_ix = init_vault_config_ix(agent_asset, vault_config, agent_account);
    wrong_treasury_ix.accounts[5] = AccountMeta::new(wrong_treasury, false);
    let treasury_error = send_unsigned_tx(&mut svm, wrong_treasury_ix).unwrap_err();
    assert_eq!(
        treasury_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidTreasury as u32),
        )
    );

    let mut wrong_owner = svm.get_account(&agent_account).unwrap();
    wrong_owner.owner = SYSTEM_PROGRAM;
    svm.set_account(agent_account, wrong_owner).unwrap();
    let owner_error = send_unsigned_tx(
        &mut svm,
        init_vault_config_ix(agent_asset, vault_config, agent_account),
    )
    .unwrap_err();
    assert_eq!(
        owner_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidOwner as u32),
        )
    );

    let (vault_config, agent_account) = install_agent_fixture(&mut svm, agent_asset);
    let wrong_agent_account = Address::new_unique();
    let data = svm.get_account(&agent_account).unwrap().data;
    svm.set_account(
        wrong_agent_account,
        Account {
            lamports: 1_000_000,
            data,
            owner: REGISTRY_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();
    let pda_error = send_unsigned_tx(
        &mut svm,
        init_vault_config_ix(agent_asset, vault_config, wrong_agent_account),
    )
    .unwrap_err();
    assert_eq!(
        pda_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidAgentAccount as u32),
        )
    );

    svm.set_account(
        vault_config,
        Account {
            lamports: 123_456,
            data: Vec::new(),
            owner: SYSTEM_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();
    send_unsigned_tx(
        &mut svm,
        init_vault_config_ix(agent_asset, vault_config, agent_account),
    )
    .unwrap();
    assert_eq!(svm.get_account(&vault_config).unwrap().owner, PROGRAM_ID);

    let mut data_squatter = runtime();
    initialize_global_config(&mut data_squatter);
    data_squatter.airdrop(&FEE_TREASURY, 1).unwrap();
    let squatted_agent = Address::new_unique();
    let (squatted_vault, squatted_account) =
        install_agent_fixture(&mut data_squatter, squatted_agent);
    data_squatter
        .set_account(
            squatted_vault,
            Account {
                lamports: 123_456,
                data: vec![1],
                owner: SYSTEM_PROGRAM,
                ..Default::default()
            },
        )
        .unwrap();
    let data_error = send_unsigned_tx(
        &mut data_squatter,
        init_vault_config_ix(squatted_agent, squatted_vault, squatted_account),
    )
    .unwrap_err();
    assert_eq!(
        data_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidAccountData as u32),
        )
    );

    let mut owner_squatter = runtime();
    initialize_global_config(&mut owner_squatter);
    owner_squatter.airdrop(&FEE_TREASURY, 1).unwrap();
    let squatted_agent = Address::new_unique();
    let (squatted_vault, squatted_account) =
        install_agent_fixture(&mut owner_squatter, squatted_agent);
    owner_squatter
        .set_account(
            squatted_vault,
            Account {
                lamports: 123_456,
                data: Vec::new(),
                owner: Address::new_unique(),
                ..Default::default()
            },
        )
        .unwrap();
    let owner_error = send_unsigned_tx(
        &mut owner_squatter,
        init_vault_config_ix(squatted_agent, squatted_vault, squatted_account),
    )
    .unwrap_err();
    assert_eq!(
        owner_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidOwner as u32),
        )
    );
}

#[test]
fn config_loads_reject_valid_data_at_wrong_pda_addresses() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);

    let wrong_global_config = Address::new_unique();
    svm.set_account(
        wrong_global_config,
        Account {
            lamports: 1_000_000,
            data: svm.get_account(&global_config_pda()).unwrap().data,
            owner: PROGRAM_ID,
            ..Default::default()
        },
    )
    .unwrap();
    let mut wrong_global_ix = update_wallet_label_ix(agent_asset, vault_config, wallet);
    wrong_global_ix.accounts[1] = AccountMeta::new_readonly(wrong_global_config, false);
    let wrong_global_error = send_unsigned_tx(&mut svm, wrong_global_ix).unwrap_err();
    assert_eq!(
        wrong_global_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidPda as u32),
        )
    );

    let wrong_vault_config = Address::new_unique();
    svm.set_account(
        wrong_vault_config,
        Account {
            lamports: 1_000_000,
            data: svm.get_account(&vault_config).unwrap().data,
            owner: PROGRAM_ID,
            ..Default::default()
        },
    )
    .unwrap();
    let wrong_vault_error = send_unsigned_tx(
        &mut svm,
        update_wallet_label_ix(agent_asset, wrong_vault_config, wallet),
    )
    .unwrap_err();
    assert_eq!(
        wrong_vault_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidPda as u32),
        )
    );
}

#[test]
fn protected_ops_follow_live_core_asset_owner_and_collection() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let vault_config = create_agent_vault(&mut svm, agent_asset);
    let new_holder = Address::new_unique();
    svm.airdrop(&new_holder, 1_000_000_000).unwrap();
    set_agent_asset_owner(&mut svm, agent_asset, new_holder);

    let wallet = wallet_pda(&agent_asset, 0);
    let old_holder_error = send_unsigned_tx(
        &mut svm,
        create_wallet_ix(agent_asset, vault_config, wallet),
    )
    .unwrap_err();
    assert_eq!(
        old_holder_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidHolder as u32),
        )
    );

    send_unsigned_tx(
        &mut svm,
        create_wallet_ix_with_holder(new_holder, agent_asset, vault_config, wallet),
    )
    .unwrap();
    let min_wallet_lamports = svm.get_balance(&wallet).unwrap();
    let cpi_old_holder_error = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_memo_ix_with_holder(
            INITIALIZER,
            agent_asset,
            vault_config,
            wallet,
            min_wallet_lamports,
        ),
    )
    .unwrap_err();
    assert_eq!(
        cpi_old_holder_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidHolder as u32),
        )
    );
    send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_memo_ix_with_holder(
            new_holder,
            agent_asset,
            vault_config,
            wallet,
            min_wallet_lamports,
        ),
    )
    .unwrap();

    set_agent_asset_collection(&mut svm, agent_asset, Address::new_unique());
    let wallet_1 = wallet_pda(&agent_asset, 1);
    let collection_error = send_unsigned_tx(
        &mut svm,
        create_wallet_ix_with_holder(new_holder, agent_asset, vault_config, wallet_1),
    )
    .unwrap_err();
    assert_eq!(
        collection_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidCollection as u32),
        )
    );
}

#[test]
fn deposit_sol_is_permissionless_but_wallet_count_cannot_overflow() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let vault_config = create_agent_vault(&mut svm, agent_asset);
    let wallet = wallet_pda(&agent_asset, 0);
    send_unsigned_tx(
        &mut svm,
        create_wallet_ix(agent_asset, vault_config, wallet),
    )
    .unwrap();

    let funder = Address::new_unique();
    svm.airdrop(&funder, 1_000_000).unwrap();
    let before = svm.get_balance(&wallet).unwrap();
    send_unsigned_tx(
        &mut svm,
        deposit_sol_ix_with_funder(funder, agent_asset, wallet, 123_456),
    )
    .unwrap();
    assert_eq!(svm.get_balance(&wallet).unwrap(), before + 123_456);

    let mut vault_account = svm.get_account(&vault_config).unwrap();
    vault_account.data[VAULT_CONFIG_WALLET_COUNT_OFFSET..VAULT_CONFIG_WALLET_COUNT_OFFSET + 2]
        .copy_from_slice(&u16::MAX.to_le_bytes());
    svm.set_account(vault_config, vault_account).unwrap();
    let overflow_wallet = wallet_pda(&agent_asset, u16::MAX);
    let overflow_error = send_unsigned_tx(
        &mut svm,
        create_wallet_ix(agent_asset, vault_config, overflow_wallet),
    )
    .unwrap_err();
    assert_eq!(
        overflow_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::WalletCountOverflow as u32),
        )
    );
}

#[test]
fn init_create_deposit_and_withdraw_sol_flow() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, agent_account) = install_agent_fixture(&mut svm, agent_asset);
    let init_vault_cu = send_unsigned_tx(
        &mut svm,
        init_vault_config_ix(agent_asset, vault_config, agent_account),
    )
    .unwrap();
    assert!(
        init_vault_cu <= 30_000,
        "init_vault_config CU: {init_vault_cu}"
    );

    let vault_account = svm.get_account(&vault_config).unwrap();
    assert_eq!(vault_account.owner, PROGRAM_ID);
    assert_eq!(vault_account.data.len(), VAULT_CONFIG_LEN);
    assert_eq!(
        unpack_vault_config(&vault_account.data)
            .unwrap()
            .wallet_count,
        0
    );
    assert_eq!(
        svm.get_balance(&FEE_TREASURY).unwrap(),
        1 + EXPECTED_ACTIVATION_FEE_LAMPORTS
    );

    let wallet = wallet_pda(&agent_asset, 0);
    let create_wallet_cu = send_unsigned_tx(
        &mut svm,
        create_wallet_ix(agent_asset, vault_config, wallet),
    )
    .unwrap();
    assert!(
        create_wallet_cu <= 18_000,
        "create_wallet CU: {create_wallet_cu}"
    );
    let vault_account = svm.get_account(&vault_config).unwrap();
    assert_eq!(
        unpack_vault_config(&vault_account.data)
            .unwrap()
            .wallet_count,
        1
    );
    let wallet_account = svm.get_account(&wallet).unwrap();
    assert_eq!(wallet_account.owner, PROGRAM_ID);
    assert_eq!(wallet_account.data.len(), WALLET_LEN);
    let decoded_wallet = unpack_wallet(&wallet_account.data).unwrap();
    assert_eq!(decoded_wallet.index, 0);
    assert_eq!(decoded_wallet.flags, WALLET_FLAG_ACTIVE);
    assert_eq!(&decoded_wallet.label[..8], b"treasury");
    assert_eq!(decoded_wallet.flags & WALLET_FLAG_RECOVERY_ONLY, 0,);

    let wallet_1 = wallet_pda(&agent_asset, 1);
    send_unsigned_tx(
        &mut svm,
        create_wallet_ix(agent_asset, vault_config, wallet_1),
    )
    .unwrap();
    let wallet_1_before_transfer = svm.get_balance(&wallet_1).unwrap();

    let wallet_before_deposit = svm.get_balance(&wallet).unwrap();
    let deposit_cu =
        send_unsigned_tx(&mut svm, deposit_sol_ix(agent_asset, wallet, 1_000_000)).unwrap();
    assert!(deposit_cu <= 16_000, "deposit_sol CU: {deposit_cu}");
    assert_eq!(
        svm.get_balance(&wallet).unwrap(),
        wallet_before_deposit + 1_000_000
    );

    let destination = Address::new_unique();
    svm.airdrop(&destination, 1).unwrap();
    let withdraw_cu = send_unsigned_tx(
        &mut svm,
        withdraw_sol_ix(agent_asset, wallet, destination, 400_000),
    )
    .unwrap();
    assert!(withdraw_cu <= 16_000, "withdraw_sol CU: {withdraw_cu}");
    assert_eq!(svm.get_balance(&destination).unwrap(), 400_001);
    assert_eq!(
        svm.get_balance(&wallet).unwrap(),
        wallet_before_deposit + 600_000
    );

    send_unsigned_tx(
        &mut svm,
        transfer_sol_ix(agent_asset, 0, wallet, 1, wallet_1, 100_000),
    )
    .unwrap();
    assert_eq!(
        svm.get_balance(&wallet).unwrap(),
        wallet_before_deposit + 500_000
    );
    assert_eq!(
        svm.get_balance(&wallet_1).unwrap(),
        wallet_1_before_transfer + 100_000
    );
}

#[test]
fn rent_snapshots_match_active_rent() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    assert_eq!(
        svm.get_balance(&global_config_pda()).unwrap(),
        rent_minimum(GLOBAL_CONFIG_LEN)
    );

    let agent_asset = Address::new_unique();
    let (vault_config, agent_account) = install_agent_fixture(&mut svm, agent_asset);
    send_unsigned_tx(
        &mut svm,
        init_vault_config_ix(agent_asset, vault_config, agent_account),
    )
    .unwrap();
    assert_eq!(
        svm.get_balance(&vault_config).unwrap(),
        rent_minimum(VAULT_CONFIG_LEN)
    );

    let wallet = wallet_pda(&agent_asset, 0);
    send_unsigned_tx(
        &mut svm,
        create_wallet_ix(agent_asset, vault_config, wallet),
    )
    .unwrap();
    assert_eq!(svm.get_balance(&wallet).unwrap(), rent_minimum(WALLET_LEN));

    let mint = Address::new_unique();
    install_mint(&mut svm, mint, TOKEN_PROGRAM, 6);
    send_unsigned_tx(
        &mut svm,
        create_wallet_ata_ix(agent_asset, vault_config, wallet, mint, TOKEN_PROGRAM, 0),
    )
    .unwrap();
    assert_eq!(
        svm.get_balance(&ata_address(&wallet, &mint, &TOKEN_PROGRAM))
            .unwrap(),
        token_account_rent()
    );
}

#[test]
fn devnet_release_cost_report() {
    let mut svm = runtime();
    install_mock_amm(&mut svm);
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, agent_account) = install_agent_fixture(&mut svm, agent_asset);
    let init_vault_cu = send_unsigned_tx(
        &mut svm,
        init_vault_config_ix(agent_asset, vault_config, agent_account),
    )
    .unwrap();

    let wallet = wallet_pda(&agent_asset, 0);
    let create_wallet_cu = send_unsigned_tx(
        &mut svm,
        create_wallet_ix(agent_asset, vault_config, wallet),
    )
    .unwrap();
    let wallet_rent_floor = svm.get_balance(&wallet).unwrap();
    let update_label_cu = send_unsigned_tx(
        &mut svm,
        update_wallet_label_ix(agent_asset, vault_config, wallet),
    )
    .unwrap();
    let wallet_1 = wallet_pda(&agent_asset, 1);
    let create_wallet_1_cu = send_unsigned_tx(
        &mut svm,
        create_wallet_ix(agent_asset, vault_config, wallet_1),
    )
    .unwrap();
    let deposit_cu =
        send_unsigned_tx(&mut svm, deposit_sol_ix(agent_asset, wallet, 1_000_000)).unwrap();

    let destination = Address::new_unique();
    svm.airdrop(&destination, 1).unwrap();
    let withdraw_cu = send_unsigned_tx(
        &mut svm,
        withdraw_sol_ix(agent_asset, wallet, destination, 100_000),
    )
    .unwrap();
    let transfer_sol_cu = send_unsigned_tx(
        &mut svm,
        transfer_sol_ix(agent_asset, 0, wallet, 1, wallet_1, 100_000),
    )
    .unwrap();

    let mint = Address::new_unique();
    install_mint(&mut svm, mint, TOKEN_PROGRAM, 6);
    let create_ata_cu = send_unsigned_tx(
        &mut svm,
        create_wallet_ata_ix(agent_asset, vault_config, wallet, mint, TOKEN_PROGRAM, 0),
    )
    .unwrap();
    let source_ata = ata_address(&wallet, &mint, &TOKEN_PROGRAM);
    let token_destination = Address::new_unique();
    install_token_account(
        &mut svm,
        source_ata,
        TOKEN_PROGRAM,
        token_account_data(mint, wallet, 25),
    );
    install_token_account(
        &mut svm,
        token_destination,
        TOKEN_PROGRAM,
        token_account_data(mint, Address::new_unique(), 0),
    );
    let transfer_spl_cu = send_unsigned_tx(
        &mut svm,
        transfer_spl_ix(
            agent_asset,
            vault_config,
            wallet,
            mint,
            source_ata,
            token_destination,
            TOKEN_PROGRAM,
            25,
            6,
            0,
        ),
    )
    .unwrap();
    let rent_receiver = Address::new_unique();
    svm.airdrop(&rent_receiver, 1).unwrap();
    let close_ata_cu = send_unsigned_tx(
        &mut svm,
        close_wallet_ata_ix(
            agent_asset,
            vault_config,
            wallet,
            mint,
            TOKEN_PROGRAM,
            rent_receiver,
        ),
    )
    .unwrap();

    let native_reserve = token_account_rent();
    let wallet_wsol_ata = ata_address(&wallet, &NATIVE_MINT, &TOKEN_PROGRAM);
    svm.set_account(
        wallet_wsol_ata,
        Account {
            lamports: native_reserve,
            data: native_token_account_data(NATIVE_MINT, wallet, 0, native_reserve),
            owner: TOKEN_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();
    let wrap_sol_cu = send_unsigned_tx(
        &mut svm,
        wrap_sol_ix(agent_asset, vault_config, wallet, wallet_wsol_ata, 250_000),
    )
    .unwrap();
    send_unsigned_tx(&mut svm, sync_native_ix(wallet_wsol_ata)).unwrap();
    let unwrap_sol_cu =
        send_unsigned_tx(&mut svm, unwrap_sol_ix(agent_asset, vault_config, wallet)).unwrap();

    let min_wallet_lamports = svm.get_balance(&wallet).unwrap();
    let execute_cpi_noop_cu = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_noop_ix(agent_asset, vault_config, wallet, min_wallet_lamports),
    )
    .unwrap();
    assert!(
        execute_cpi_noop_cu <= 22_200,
        "execute_cpi_checked noop baseline CU: {execute_cpi_noop_cu}"
    );
    let excess_lamports = svm.get_balance(&wallet).unwrap() - wallet_rent_floor;
    send_unsigned_tx(
        &mut svm,
        withdraw_sol_ix(agent_asset, wallet, destination, excess_lamports),
    )
    .unwrap();
    let close_wallet_cu = send_unsigned_tx(
        &mut svm,
        close_wallet_ix(agent_asset, vault_config, wallet, rent_receiver),
    )
    .unwrap();
    let reopen_cu = send_unsigned_tx(
        &mut svm,
        reopen_wallet_for_recovery_ix(agent_asset, vault_config, wallet),
    )
    .unwrap();

    println!("agent-vault devnet cost report");
    println!("activation_fee_lamports={EXPECTED_ACTIVATION_FEE_LAMPORTS}");
    println!("rent_global_config_160={}", rent_minimum(GLOBAL_CONFIG_LEN));
    println!("rent_vault_config_24={}", rent_minimum(VAULT_CONFIG_LEN));
    println!("rent_wallet_32={}", rent_minimum(WALLET_LEN));
    println!("rent_token_account_165={}", token_account_rent());
    println!("cu_init_vault_config={init_vault_cu}");
    println!("cu_create_wallet={create_wallet_cu}");
    println!("cu_create_wallet_second={create_wallet_1_cu}");
    println!("cu_update_wallet_label={update_label_cu}");
    println!("cu_deposit_sol={deposit_cu}");
    println!("cu_withdraw_sol={withdraw_cu}");
    println!("cu_transfer_sol={transfer_sol_cu}");
    println!("cu_create_wallet_ata={create_ata_cu}");
    println!("cu_transfer_spl={transfer_spl_cu}");
    println!("cu_close_wallet_ata={close_ata_cu}");
    println!("cu_wrap_sol={wrap_sol_cu}");
    println!("cu_unwrap_sol={unwrap_sol_cu}");
    println!("cu_execute_cpi_checked_noop_baseline={execute_cpi_noop_cu}");
    println!("cu_close_wallet={close_wallet_cu}");
    println!("cu_reopen_wallet_for_recovery={reopen_cu}");
}

#[test]
fn update_wallet_label_persists_utf8_label() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    send_unsigned_tx(
        &mut svm,
        update_wallet_label_ix(agent_asset, vault_config, wallet),
    )
    .unwrap();

    let wallet_account = svm.get_account(&wallet).unwrap();
    let decoded_wallet = unpack_wallet(&wallet_account.data).unwrap();
    assert_eq!(decoded_wallet.index, 0);
    assert_eq!(decoded_wallet.flags, WALLET_FLAG_ACTIVE);
    assert_eq!(&decoded_wallet.label[..3], b"ops");
    assert!(decoded_wallet.label[3..].iter().all(|byte| *byte == 0));
}

#[test]
fn update_wallet_label_rejects_wrong_index_and_recovery_wallets() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let original_wallet = svm.get_account(&wallet).unwrap();
    let wrong_index = send_unsigned_tx(
        &mut svm,
        update_wallet_label_ix_with(agent_asset, vault_config, wallet, 1, b"bad"),
    )
    .unwrap_err();
    assert_eq!(
        wrong_index,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidWallet as u32),
        )
    );
    assert_eq!(svm.get_account(&wallet).unwrap().data, original_wallet.data);

    let recovery_agent = Address::new_unique();
    let (recovery_vault, recovery_wallet) = create_recovery_only_wallet(&mut svm, recovery_agent);
    let recovery_before = svm.get_account(&recovery_wallet).unwrap();
    let recovery_error = send_unsigned_tx(
        &mut svm,
        update_wallet_label_ix_with(recovery_agent, recovery_vault, recovery_wallet, 0, b"bad"),
    )
    .unwrap_err();
    assert_eq!(
        recovery_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidWallet as u32),
        )
    );
    assert_eq!(
        svm.get_account(&recovery_wallet).unwrap().data,
        recovery_before.data
    );
}

#[test]
fn sol_paths_preserve_rent_floor_and_reject_duplicate_moves() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let destination = Address::new_unique();
    svm.airdrop(&destination, 1).unwrap();
    let rent_floor = svm.get_balance(&wallet).unwrap();

    let rent_error = send_unsigned_tx(
        &mut svm,
        withdraw_sol_ix(agent_asset, wallet, destination, 1),
    )
    .unwrap_err();
    assert_eq!(
        rent_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::RentFloorViolation as u32),
        )
    );
    assert_eq!(svm.get_balance(&wallet).unwrap(), rent_floor);

    let wallet_1 = wallet_pda(&agent_asset, 1);
    send_unsigned_tx(
        &mut svm,
        create_wallet_ix(agent_asset, vault_config, wallet_1),
    )
    .unwrap();
    let transfer_error = send_unsigned_tx(
        &mut svm,
        transfer_sol_ix(agent_asset, 0, wallet, 0, wallet, 0),
    )
    .unwrap_err();
    assert_eq!(
        transfer_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::DuplicateAccount as u32),
        )
    );

    send_unsigned_tx(&mut svm, deposit_sol_ix(agent_asset, wallet, 10)).unwrap();
    let close_error = send_unsigned_tx(
        &mut svm,
        close_wallet_ix(agent_asset, vault_config, wallet, destination),
    )
    .unwrap_err();
    assert_eq!(
        close_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InsufficientLamports as u32),
        )
    );

    let duplicate_close_receiver = send_unsigned_tx(
        &mut svm,
        close_wallet_ix(agent_asset, vault_config, wallet_1, wallet_1),
    )
    .unwrap_err();
    assert_eq!(
        duplicate_close_receiver,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::DuplicateAccount as u32),
        )
    );
}

#[test]
fn execute_cpi_checked_rejects_writable_wallet_account() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let error = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_wallet_writable_ix(agent_asset, vault_config, wallet),
    )
    .unwrap_err();

    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidWritable as u32),
        )
    );
}

#[test]
fn execute_cpi_checked_rejects_missing_economic_post_check() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let error = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_missing_economic_post_check_ix(agent_asset, vault_config, wallet),
    )
    .unwrap_err();

    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::MissingEconomicPostCheck as u32),
        )
    );
}

#[test]
fn execute_cpi_checked_rejects_denied_target_programs() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();
    for program in [
        TOKEN_PROGRAM,
        TOKEN_2022_PROGRAM,
        ASSOCIATED_TOKEN_PROGRAM,
    ] {
        install_executable_account(&mut svm, program);
    }
    for program in [
        BPF_LOADER,
        BPF_LOADER_DEPRECATED,
        BPF_UPGRADEABLE_LOADER,
        NATIVE_LOADER,
        LOADER_V4,
    ] {
        install_executable_marker_account(&mut svm, program);
    }

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let min_wallet_lamports = svm.get_balance(&wallet).unwrap();

    for denied_program in [
        PROGRAM_ID,
        TOKEN_PROGRAM,
        TOKEN_2022_PROGRAM,
        ASSOCIATED_TOKEN_PROGRAM,
        BPF_LOADER,
        BPF_LOADER_DEPRECATED,
        BPF_UPGRADEABLE_LOADER,
        NATIVE_LOADER,
        LOADER_V4,
    ] {
        let error = send_unsigned_tx(
            &mut svm,
            execute_cpi_checked_with_target_ix(
                agent_asset,
                vault_config,
                wallet,
                denied_program,
                min_wallet_lamports,
            ),
        )
        .unwrap_err();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(AgentVaultError::InvalidCpiTarget as u32),
            )
        );
    }
}

#[test]
fn execute_cpi_checked_invokes_memo_with_only_wallet_meta() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let min_wallet_lamports = svm.get_balance(&wallet).unwrap();

    let execute_cu = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_memo_ix(agent_asset, vault_config, wallet, min_wallet_lamports),
    )
    .unwrap();
    assert!(
        execute_cu <= 50_000,
        "execute_cpi_checked memo CU: {execute_cu}"
    );

    assert_eq!(svm.get_balance(&wallet).unwrap(), min_wallet_lamports);
    assert_eq!(svm.get_account(&wallet).unwrap().owner, PROGRAM_ID);
}

#[test]
fn execute_cpi_checked_uses_redeemable_wsol_balance() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let native_reserve = token_account_rent();
    let redeemable_lamports = 500;
    let wallet_wsol_ata = ata_address(&wallet, &NATIVE_MINT, &TOKEN_PROGRAM);

    svm.set_account(
        NATIVE_MINT,
        Account {
            lamports: 1_000_000,
            data: tokenkeg_mint_data(9),
            owner: TOKEN_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();
    svm.set_account(
        wallet_wsol_ata,
        Account {
            lamports: native_reserve + redeemable_lamports,
            data: native_token_account_data(NATIVE_MINT, wallet, 0, native_reserve),
            owner: TOKEN_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();

    send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_wsol_balance_ix(
            agent_asset,
            vault_config,
            wallet,
            wallet_wsol_ata,
            redeemable_lamports,
        ),
    )
    .unwrap();
}

#[test]
fn execute_cpi_checked_rejects_non_executable_and_duplicate_remaining_accounts() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let min_wallet_lamports = svm.get_balance(&wallet).unwrap();
    let non_executable = Address::new_unique();
    svm.airdrop(&non_executable, 1).unwrap();

    let non_executable_error = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_with_target_ix(
            agent_asset,
            vault_config,
            wallet,
            non_executable,
            min_wallet_lamports,
        ),
    )
    .unwrap_err();
    assert_eq!(
        non_executable_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidCpiTarget as u32),
        )
    );

    let mut duplicate_holder_ix =
        execute_cpi_checked_memo_ix(agent_asset, vault_config, wallet, min_wallet_lamports);
    duplicate_holder_ix.data[4] = 1;
    duplicate_holder_ix
        .accounts
        .push(AccountMeta::new_readonly(INITIALIZER, false));
    let duplicate_error = send_unsigned_tx(&mut svm, duplicate_holder_ix).unwrap_err();
    assert_eq!(
        duplicate_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::DuplicateAccount as u32),
        )
    );
}

#[test]
fn execute_cpi_checked_requires_custody_checks_for_writable_wallet_tokens() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let min_wallet_lamports = svm.get_balance(&wallet).unwrap();
    let mint = Address::new_unique();
    install_mint(&mut svm, mint, TOKEN_PROGRAM, 6);
    let wallet_ata = ata_address(&wallet, &mint, &TOKEN_PROGRAM);
    install_token_account(
        &mut svm,
        wallet_ata,
        TOKEN_PROGRAM,
        token_account_data(mint, wallet, 1),
    );

    let error = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_writable_token_missing_custody_ix(
            agent_asset,
            vault_config,
            wallet,
            wallet_ata,
            min_wallet_lamports,
        ),
    )
    .unwrap_err();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::MissingCustodyPostCheck as u32),
        )
    );
}

#[test]
fn execute_cpi_checked_validates_token_custody_equals_and_ata_status() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let mint = Address::new_unique();
    install_mint(&mut svm, mint, TOKEN_PROGRAM, 6);
    let wallet_ata = ata_address(&wallet, &mint, &TOKEN_PROGRAM);
    install_token_account(
        &mut svm,
        wallet_ata,
        TOKEN_PROGRAM,
        token_account_data(mint, wallet, 1),
    );

    send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_token_custody_equals_ix(
            agent_asset,
            vault_config,
            wallet,
            wallet_ata,
            mint,
            1,
            wallet,
        ),
    )
    .unwrap();

    let wrong_authority = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_token_custody_equals_ix(
            agent_asset,
            vault_config,
            wallet,
            wallet_ata,
            mint,
            1,
            Address::new_unique(),
        ),
    )
    .unwrap_err();
    assert_eq!(
        wrong_authority,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::PostCheckFailed as u32),
        )
    );

    let non_ata_wallet_token = Address::new_unique();
    install_token_account(
        &mut svm,
        non_ata_wallet_token,
        TOKEN_PROGRAM,
        token_account_data(mint, wallet, 1),
    );
    let non_ata_error = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_token_custody_equals_ix(
            agent_asset,
            vault_config,
            wallet,
            non_ata_wallet_token,
            mint,
            1,
            wallet,
        ),
    )
    .unwrap_err();
    assert_eq!(
        non_ata_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidAta as u32),
        )
    );

    svm.set_account(
        wallet_ata,
        Account {
            lamports: token_account_rent(),
            data: vec![0u8; TOKEN_ACCOUNT_LEN - 1],
            owner: TOKEN_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();
    let malformed_error = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_token_custody_equals_ix(
            agent_asset,
            vault_config,
            wallet,
            wallet_ata,
            mint,
            1,
            wallet,
        ),
    )
    .unwrap_err();
    assert_eq!(
        malformed_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidTokenAccount as u32),
        )
    );
}

#[test]
fn execute_cpi_checked_rejects_malformed_token_balance_post_checks() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let mint = Address::new_unique();
    install_mint(&mut svm, mint, TOKEN_PROGRAM, 6);

    let system_token = Address::new_unique();
    svm.airdrop(&system_token, 1).unwrap();
    let system_error = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_token_balance_min_ix(
            agent_asset,
            vault_config,
            wallet,
            system_token,
            mint,
            mint,
        ),
    )
    .unwrap_err();
    assert_eq!(
        system_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidTokenAccount as u32),
        )
    );

    let short_token = Address::new_unique();
    svm.set_account(
        short_token,
        Account {
            lamports: token_account_rent(),
            data: vec![0u8; TOKEN_ACCOUNT_LEN - 1],
            owner: TOKEN_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();
    let short_error = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_token_balance_min_ix(
            agent_asset,
            vault_config,
            wallet,
            short_token,
            mint,
            mint,
        ),
    )
    .unwrap_err();
    assert_eq!(
        short_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidTokenAccount as u32),
        )
    );

    let token = Address::new_unique();
    install_token_account(
        &mut svm,
        token,
        TOKEN_PROGRAM,
        token_account_data(mint, Address::new_unique(), 1),
    );
    let wrong_owner_mint = Address::new_unique();
    svm.airdrop(&wrong_owner_mint, 1).unwrap();
    let mint_owner_error = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_token_balance_min_ix(
            agent_asset,
            vault_config,
            wallet,
            token,
            wrong_owner_mint,
            wrong_owner_mint,
        ),
    )
    .unwrap_err();
    assert_eq!(
        mint_owner_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidTokenAccount as u32),
        )
    );

    let wrong_mint = Address::new_unique();
    install_mint(&mut svm, wrong_mint, TOKEN_PROGRAM, 6);
    let wrong_mint_error = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_token_balance_min_ix(
            agent_asset,
            vault_config,
            wallet,
            token,
            wrong_mint,
            wrong_mint,
        ),
    )
    .unwrap_err();
    assert_eq!(
        wrong_mint_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidTokenAccount as u32),
        )
    );
}

#[test]
fn execute_cpi_checked_rejects_actual_token_custody_mutation() {
    let mut svm = runtime();
    install_mock_amm(&mut svm);
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let mint = Address::new_unique();
    install_mint(&mut svm, mint, TOKEN_PROGRAM, 6);
    let wallet_ata = ata_address(&wallet, &mint, &TOKEN_PROGRAM);
    install_token_account(
        &mut svm,
        wallet_ata,
        TOKEN_PROGRAM,
        token_account_data(mint, wallet, 10),
    );
    let delegate = Address::new_unique();
    svm.airdrop(&delegate, 1).unwrap();

    let error = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_mutate_delegate_ix(
            agent_asset,
            vault_config,
            wallet,
            wallet_ata,
            mint,
            delegate,
        ),
    )
    .unwrap_err();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::CustodyChanged as u32),
        )
    );
    let account = svm.get_account(&wallet_ata).unwrap();
    assert_eq!(token_amount(&svm, &wallet_ata), 10);
    assert_eq!(&account.data[72..76], &[0, 0, 0, 0]);
    assert_eq!(u64::from_le_bytes(account.data[121..129].try_into().unwrap()), 0);
}

#[test]
fn execute_cpi_checked_token_custody_equals_supports_new_wallet_control() {
    let mut svm = runtime();
    install_mock_amm(&mut svm);
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let mint = Address::new_unique();
    let current_authority = Address::new_unique();
    svm.airdrop(&current_authority, 1).unwrap();
    install_mint(&mut svm, mint, TOKEN_PROGRAM, 6);
    let wallet_ata = ata_address(&wallet, &mint, &TOKEN_PROGRAM);
    install_token_account(
        &mut svm,
        wallet_ata,
        TOKEN_PROGRAM,
        token_account_data(mint, current_authority, 5),
    );

    let wrong_expected = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_set_authority_to_wallet_ix(
            agent_asset,
            vault_config,
            wallet,
            wallet_ata,
            mint,
            current_authority,
            current_authority,
        ),
    )
    .unwrap_err();
    assert_eq!(
        wrong_expected,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::PostCheckFailed as u32),
        )
    );
    assert_eq!(&svm.get_account(&wallet_ata).unwrap().data[32..64], current_authority.as_ref());

    send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_set_authority_to_wallet_ix(
            agent_asset,
            vault_config,
            wallet,
            wallet_ata,
            mint,
            current_authority,
            wallet,
        ),
    )
    .unwrap();
    assert_eq!(&svm.get_account(&wallet_ata).unwrap().data[32..64], wallet.as_ref());
}

#[test]
fn execute_cpi_checked_token_2022_custody_equals_checks_extension_hash() {
    let mut svm = runtime();
    install_mock_amm(&mut svm);
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let mint = Address::new_unique();
    svm.set_account(
        mint,
        Account {
            lamports: 1_000_000,
            data: token_2022_transfer_fee_mint_data(6, 1_000, 100),
            owner: TOKEN_2022_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();
    let source_authority = Address::new_unique();
    svm.airdrop(&source_authority, 1).unwrap();
    let source = Address::new_unique();
    let destination = ata_address(&wallet, &mint, &TOKEN_2022_PROGRAM);
    install_token_account(
        &mut svm,
        source,
        TOKEN_2022_PROGRAM,
        token_2022_account_data_with_withheld_fee(mint, source_authority, 1_000, 0),
    );
    install_token_account(
        &mut svm,
        destination,
        TOKEN_2022_PROGRAM,
        token_2022_account_data_with_withheld_fee(mint, wallet, 0, 0),
    );

    let wrong_hash = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_token_2022_fee_receive_ix(
            agent_asset,
            vault_config,
            wallet,
            source,
            mint,
            destination,
            source_authority,
            1_000,
            10,
            SHA256_EMPTY,
        ),
    )
    .unwrap_err();
    assert_eq!(
        wrong_hash,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::PostCheckFailed as u32),
        )
    );
    assert_eq!(token_amount(&svm, &source), 1_000);
    assert_eq!(token_amount(&svm, &destination), 0);

    send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_token_2022_fee_receive_ix(
            agent_asset,
            vault_config,
            wallet,
            source,
            mint,
            destination,
            source_authority,
            1_000,
            10,
            transfer_fee_amount_extension_hash(10),
        ),
    )
    .unwrap();
    assert_eq!(token_amount(&svm, &source), 0);
    assert_eq!(token_amount(&svm, &destination), 990);
}

#[test]
fn execute_cpi_checked_mock_swap_enforces_max_input_and_min_output() {
    let mut svm = runtime();
    install_mock_amm(&mut svm);
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let input_mint = Address::new_unique();
    let output_mint = Address::new_unique();
    install_mint(&mut svm, input_mint, TOKEN_PROGRAM, 6);
    install_mint(&mut svm, output_mint, TOKEN_PROGRAM, 6);

    let user_input = ata_address(&wallet, &input_mint, &TOKEN_PROGRAM);
    let pool_input = Address::new_unique();
    let pool_output = ata_address(&wallet, &output_mint, &TOKEN_PROGRAM);
    let user_output = Address::new_unique();

    let reset_swap_accounts = |svm: &mut LiteSVM| {
        install_token_account(
            svm,
            user_input,
            TOKEN_PROGRAM,
            token_account_data(input_mint, wallet, 1_000),
        );
        install_token_account(
            svm,
            pool_input,
            TOKEN_PROGRAM,
            token_account_data(input_mint, Address::new_unique(), 0),
        );
        install_token_account(
            svm,
            pool_output,
            TOKEN_PROGRAM,
            token_account_data(output_mint, wallet, 500),
        );
        install_token_account(
            svm,
            user_output,
            TOKEN_PROGRAM,
            token_account_data(output_mint, Address::new_unique(), 0),
        );
    };

    reset_swap_accounts(&mut svm);
    send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_mock_swap_ix(
            agent_asset,
            vault_config,
            wallet,
            input_mint,
            output_mint,
            user_input,
            pool_input,
            pool_output,
            user_output,
            100,
            100,
            40,
            40,
        ),
    )
    .unwrap();
    assert_eq!(token_amount(&svm, &user_input), 900);
    assert_eq!(token_amount(&svm, &pool_input), 100);
    assert_eq!(token_amount(&svm, &pool_output), 460);
    assert_eq!(token_amount(&svm, &user_output), 40);

    reset_swap_accounts(&mut svm);
    let min_out_error = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_mock_swap_ix(
            agent_asset,
            vault_config,
            wallet,
            input_mint,
            output_mint,
            user_input,
            pool_input,
            pool_output,
            user_output,
            100,
            100,
            30,
            40,
        ),
    )
    .unwrap_err();
    assert_eq!(
        min_out_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::PostCheckFailed as u32),
        )
    );
    assert_eq!(token_amount(&svm, &user_input), 1_000);
    assert_eq!(token_amount(&svm, &pool_input), 0);
    assert_eq!(token_amount(&svm, &pool_output), 500);
    assert_eq!(token_amount(&svm, &user_output), 0);

    let max_input_error = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_mock_swap_ix(
            agent_asset,
            vault_config,
            wallet,
            input_mint,
            output_mint,
            user_input,
            pool_input,
            pool_output,
            user_output,
            120,
            100,
            40,
            40,
        ),
    )
    .unwrap_err();
    assert_eq!(
        max_input_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::PostCheckFailed as u32),
        )
    );
    assert_eq!(token_amount(&svm, &user_input), 1_000);
    assert_eq!(token_amount(&svm, &pool_input), 0);
    assert_eq!(token_amount(&svm, &pool_output), 500);
    assert_eq!(token_amount(&svm, &user_output), 0);
}

#[test]
fn execute_cpi_checked_requires_state_checks_for_writable_non_token_accounts() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let min_wallet_lamports = svm.get_balance(&wallet).unwrap();
    let writable_account = Address::new_unique();
    svm.airdrop(&writable_account, 1).unwrap();

    let missing_owner_check = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_writable_account_state_ix(
            agent_asset,
            vault_config,
            wallet,
            writable_account,
            SYSTEM_PROGRAM,
            false,
            min_wallet_lamports,
        ),
    )
    .unwrap_err();
    assert_eq!(
        missing_owner_check,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::MissingCustodyPostCheck as u32),
        )
    );

    send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_writable_account_state_ix(
            agent_asset,
            vault_config,
            wallet,
            writable_account,
            SYSTEM_PROGRAM,
            true,
            min_wallet_lamports,
        ),
    )
    .unwrap();
}

#[test]
fn wsol_wrap_and_unwrap_preserve_wallet_authority_and_rent() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let native_reserve = token_account_rent();
    let wallet_wsol_ata = ata_address(&wallet, &NATIVE_MINT, &TOKEN_PROGRAM);
    install_token_account(
        &mut svm,
        wallet_wsol_ata,
        TOKEN_PROGRAM,
        native_token_account_data(NATIVE_MINT, wallet, 0, native_reserve),
    );
    svm.set_account(
        wallet_wsol_ata,
        Account {
            lamports: native_reserve,
            data: native_token_account_data(NATIVE_MINT, wallet, 0, native_reserve),
            owner: TOKEN_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();

    send_unsigned_tx(&mut svm, deposit_sol_ix(agent_asset, wallet, 1_000_000)).unwrap();
    let wallet_before_wrap = svm.get_balance(&wallet).unwrap();
    send_unsigned_tx(
        &mut svm,
        wrap_sol_ix(agent_asset, vault_config, wallet, wallet_wsol_ata, 250_000),
    )
    .unwrap();
    assert_eq!(
        svm.get_balance(&wallet).unwrap(),
        wallet_before_wrap - 250_000
    );
    assert_eq!(token_amount(&svm, &wallet_wsol_ata), 0);

    send_unsigned_tx(&mut svm, sync_native_ix(wallet_wsol_ata)).unwrap();
    assert_eq!(token_amount(&svm, &wallet_wsol_ata), 250_000);

    send_unsigned_tx(&mut svm, unwrap_sol_ix(agent_asset, vault_config, wallet)).unwrap();
    assert_eq!(
        svm.get_balance(&wallet).unwrap(),
        wallet_before_wrap + native_reserve
    );
}

#[test]
fn wsol_wrap_rejects_malformed_wallet_atas() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    send_unsigned_tx(&mut svm, deposit_sol_ix(agent_asset, wallet, 1_000_000)).unwrap();
    let wallet_wsol_ata = ata_address(&wallet, &NATIVE_MINT, &TOKEN_PROGRAM);
    let native_reserve = token_account_rent();

    for data in [
        token_account_data(NATIVE_MINT, wallet, 0),
        native_token_account_data(NATIVE_MINT, Address::new_unique(), 0, native_reserve),
        token_account_data_with_delegate(NATIVE_MINT, wallet, 0, Address::new_unique()),
        token_account_data_with_close_authority(NATIVE_MINT, wallet, 0, Address::new_unique()),
    ] {
        svm.set_account(
            wallet_wsol_ata,
            Account {
                lamports: native_reserve,
                data,
                owner: TOKEN_PROGRAM,
                ..Default::default()
            },
        )
        .unwrap();
        let error = send_unsigned_tx(
            &mut svm,
            wrap_sol_ix(agent_asset, vault_config, wallet, wallet_wsol_ata, 1),
        )
        .unwrap_err();
        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(AgentVaultError::InvalidWsolAccount as u32),
            )
        );
    }
}

#[test]
fn unwrap_sol_rejects_malformed_wsol_ata() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let wallet_wsol_ata = ata_address(&wallet, &NATIVE_MINT, &TOKEN_PROGRAM);
    let native_reserve = token_account_rent();

    for data in [
        token_account_data(NATIVE_MINT, wallet, 0),
        native_token_account_data(NATIVE_MINT, Address::new_unique(), 0, native_reserve),
        token_account_data_with_delegate(NATIVE_MINT, wallet, 0, Address::new_unique()),
        token_account_data_with_close_authority(NATIVE_MINT, wallet, 0, Address::new_unique()),
    ] {
        svm.set_account(
            wallet_wsol_ata,
            Account {
                lamports: native_reserve,
                data,
                owner: TOKEN_PROGRAM,
                ..Default::default()
            },
        )
        .unwrap();
        let error = send_unsigned_tx(
            &mut svm,
            unwrap_sol_ix(agent_asset, vault_config, wallet),
        )
        .unwrap_err();
        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(AgentVaultError::InvalidWsolAccount as u32),
            )
        );
    }
}

#[test]
fn tokenkeg_ata_transfer_and_close_paths_work() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let mint = Address::new_unique();
    install_mint(&mut svm, mint, TOKEN_PROGRAM, 6);

    send_unsigned_tx(
        &mut svm,
        create_wallet_ata_ix(agent_asset, vault_config, wallet, mint, TOKEN_PROGRAM, 0),
    )
    .unwrap();
    let source_ata = ata_address(&wallet, &mint, &TOKEN_PROGRAM);
    assert_eq!(svm.get_account(&source_ata).unwrap().owner, TOKEN_PROGRAM);

    install_token_account(
        &mut svm,
        source_ata,
        TOKEN_PROGRAM,
        token_account_data(mint, wallet, 1_000_000),
    );
    let destination = Address::new_unique();
    install_token_account(
        &mut svm,
        destination,
        TOKEN_PROGRAM,
        token_account_data(mint, Address::new_unique(), 0),
    );

    send_unsigned_tx(
        &mut svm,
        transfer_spl_ix(
            agent_asset,
            vault_config,
            wallet,
            mint,
            source_ata,
            destination,
            TOKEN_PROGRAM,
            1_000_000,
            6,
            0,
        ),
    )
    .unwrap();
    assert_eq!(token_amount(&svm, &source_ata), 0);
    assert_eq!(token_amount(&svm, &destination), 1_000_000);

    install_token_account(
        &mut svm,
        source_ata,
        TOKEN_PROGRAM,
        token_account_data(mint, wallet, 10),
    );
    let non_ata_wallet_destination = Address::new_unique();
    install_token_account(
        &mut svm,
        non_ata_wallet_destination,
        TOKEN_PROGRAM,
        token_account_data(mint, wallet, 0),
    );
    let non_ata_error = send_unsigned_tx(
        &mut svm,
        transfer_spl_ix(
            agent_asset,
            vault_config,
            wallet,
            mint,
            source_ata,
            non_ata_wallet_destination,
            TOKEN_PROGRAM,
            1,
            6,
            0,
        ),
    )
    .unwrap_err();
    assert_eq!(
        non_ata_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidAta as u32),
        )
    );

    let rent_receiver = Address::new_unique();
    svm.airdrop(&rent_receiver, 1).unwrap();
    let nonzero_close = send_unsigned_tx(
        &mut svm,
        close_wallet_ata_ix(
            agent_asset,
            vault_config,
            wallet,
            mint,
            TOKEN_PROGRAM,
            rent_receiver,
        ),
    )
    .unwrap_err();
    assert_eq!(
        nonzero_close,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidTokenAccount as u32),
        )
    );

    install_token_account(
        &mut svm,
        source_ata,
        TOKEN_PROGRAM,
        token_account_data(mint, wallet, 0),
    );
    let rent_receiver_before = svm.get_balance(&rent_receiver).unwrap();
    send_unsigned_tx(
        &mut svm,
        close_wallet_ata_ix(
            agent_asset,
            vault_config,
            wallet,
            mint,
            TOKEN_PROGRAM,
            rent_receiver,
        ),
    )
    .unwrap();
    assert_eq!(
        svm.get_balance(&rent_receiver).unwrap(),
        rent_receiver_before + token_account_rent()
    );
}

#[test]
fn token_2022_create_and_close_ata_paths_work() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let mint = Address::new_unique();
    install_mint(&mut svm, mint, TOKEN_2022_PROGRAM, 6);
    let wallet_ata = ata_address(&wallet, &mint, &TOKEN_2022_PROGRAM);

    send_unsigned_tx(
        &mut svm,
        create_wallet_ata_ix(
            agent_asset,
            vault_config,
            wallet,
            mint,
            TOKEN_2022_PROGRAM,
            1,
        ),
    )
    .unwrap();
    let ata_account = svm.get_account(&wallet_ata).unwrap();
    assert_eq!(ata_account.owner, TOKEN_2022_PROGRAM);
    assert_eq!(token_amount(&svm, &wallet_ata), 0);

    let rent_receiver = Address::new_unique();
    svm.airdrop(&rent_receiver, 1).unwrap();
    let rent_receiver_before = svm.get_balance(&rent_receiver).unwrap();
    send_unsigned_tx(
        &mut svm,
        close_wallet_ata_ix(
            agent_asset,
            vault_config,
            wallet,
            mint,
            TOKEN_2022_PROGRAM,
            rent_receiver,
        ),
    )
    .unwrap();
    assert!(svm.get_balance(&rent_receiver).unwrap() > rent_receiver_before);
}

#[test]
fn create_wallet_ata_rejects_token_program_kind_mismatch() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let mint = Address::new_unique();
    install_mint(&mut svm, mint, TOKEN_PROGRAM, 6);

    let error = send_unsigned_tx(
        &mut svm,
        create_wallet_ata_ix(agent_asset, vault_config, wallet, mint, TOKEN_PROGRAM, 1),
    )
    .unwrap_err();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidTokenProgram as u32),
        )
    );
}

#[test]
fn token_2022_transfer_and_extension_rejections_are_checked() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let mint = Address::new_unique();
    install_mint(&mut svm, mint, TOKEN_2022_PROGRAM, 6);
    let source_ata = ata_address(&wallet, &mint, &TOKEN_2022_PROGRAM);
    let destination = Address::new_unique();
    install_token_account(
        &mut svm,
        source_ata,
        TOKEN_2022_PROGRAM,
        token_account_data(mint, wallet, 100),
    );
    install_token_account(
        &mut svm,
        destination,
        TOKEN_2022_PROGRAM,
        token_account_data(mint, Address::new_unique(), 0),
    );

    send_unsigned_tx(
        &mut svm,
        transfer_spl_ix(
            agent_asset,
            vault_config,
            wallet,
            mint,
            source_ata,
            destination,
            TOKEN_2022_PROGRAM,
            25,
            6,
            0,
        ),
    )
    .unwrap();
    assert_eq!(token_amount(&svm, &source_ata), 75);
    assert_eq!(token_amount(&svm, &destination), 25);

    let wrong_decimals = send_unsigned_tx(
        &mut svm,
        transfer_spl_ix(
            agent_asset,
            vault_config,
            wallet,
            mint,
            source_ata,
            destination,
            TOKEN_2022_PROGRAM,
            1,
            5,
            0,
        ),
    )
    .unwrap_err();
    assert_eq!(
        wrong_decimals,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidTokenAccount as u32),
        )
    );

    let fee_mint = Address::new_unique();
    svm.set_account(
        fee_mint,
        Account {
            lamports: 1_000_000,
            data: token_2022_transfer_fee_mint_data(6, 1_000, 100),
            owner: TOKEN_2022_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();
    let fee_source_ata = ata_address(&wallet, &fee_mint, &TOKEN_2022_PROGRAM);
    let fee_destination = Address::new_unique();
    install_token_account(
        &mut svm,
        fee_source_ata,
        TOKEN_2022_PROGRAM,
        token_2022_account_data_with_withheld_fee(fee_mint, wallet, 1_000, 0),
    );
    install_token_account(
        &mut svm,
        fee_destination,
        TOKEN_2022_PROGRAM,
        token_2022_account_data_with_withheld_fee(fee_mint, Address::new_unique(), 0, 0),
    );
    send_unsigned_tx(
        &mut svm,
        transfer_spl_ix(
            agent_asset,
            vault_config,
            wallet,
            fee_mint,
            fee_source_ata,
            fee_destination,
            TOKEN_2022_PROGRAM,
            1_000,
            6,
            10,
        ),
    )
    .unwrap();
    assert_eq!(token_amount(&svm, &fee_source_ata), 0);
    assert_eq!(token_amount(&svm, &fee_destination), 990);

    install_token_account(
        &mut svm,
        fee_source_ata,
        TOKEN_2022_PROGRAM,
        token_2022_account_data_with_withheld_fee(fee_mint, wallet, 1_000, 0),
    );
    install_token_account(
        &mut svm,
        fee_destination,
        TOKEN_2022_PROGRAM,
        token_2022_account_data_with_withheld_fee(fee_mint, Address::new_unique(), 0, 0),
    );
    let fee_mismatch = send_unsigned_tx(
        &mut svm,
        transfer_spl_ix(
            agent_asset,
            vault_config,
            wallet,
            fee_mint,
            fee_source_ata,
            fee_destination,
            TOKEN_2022_PROGRAM,
            1_000,
            6,
            0,
        ),
    )
    .unwrap_err();
    assert_eq!(
        fee_mismatch,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidTokenAccount as u32),
        )
    );

    install_token_account(
        &mut svm,
        fee_source_ata,
        TOKEN_2022_PROGRAM,
        token_2022_account_data_with_withheld_fee(fee_mint, wallet, 0, 1),
    );
    let rent_receiver = Address::new_unique();
    svm.airdrop(&rent_receiver, 1).unwrap();
    let withheld_close = send_unsigned_tx(
        &mut svm,
        close_wallet_ata_ix(
            agent_asset,
            vault_config,
            wallet,
            fee_mint,
            TOKEN_2022_PROGRAM,
            rent_receiver,
        ),
    )
    .unwrap_err();
    assert_eq!(
        withheld_close,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidTokenAccount as u32),
        )
    );
}

#[test]
fn close_wallet_ata_rejects_native_wsol_route() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_agent_vault_and_wallet(&mut svm, agent_asset);
    let wallet_wsol_ata = ata_address(&wallet, &NATIVE_MINT, &TOKEN_PROGRAM);
    let native_reserve = token_account_rent();
    svm.set_account(
        NATIVE_MINT,
        Account {
            lamports: 1_000_000,
            data: tokenkeg_mint_data(9),
            owner: TOKEN_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();
    svm.set_account(
        wallet_wsol_ata,
        Account {
            lamports: native_reserve + 500,
            data: native_token_account_data(NATIVE_MINT, wallet, 0, native_reserve),
            owner: TOKEN_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();
    let rent_receiver = Address::new_unique();
    svm.airdrop(&rent_receiver, 1).unwrap();

    let error = send_unsigned_tx(
        &mut svm,
        close_wallet_ata_ix(
            agent_asset,
            vault_config,
            wallet,
            NATIVE_MINT,
            TOKEN_PROGRAM,
            rent_receiver,
        ),
    )
    .unwrap_err();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidWsolAccount as u32),
        )
    );
}

#[test]
fn recovery_only_wallet_rejects_hot_path_operations() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();
    svm.set_account(
        TOKEN_PROGRAM,
        Account {
            lamports: 1_000_000,
            data: Vec::new(),
            owner: SYSTEM_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_recovery_only_wallet(&mut svm, agent_asset);
    let wallet_balance = svm.get_balance(&wallet).unwrap();

    let deposit_error =
        send_unsigned_tx(&mut svm, deposit_sol_ix(agent_asset, wallet, 1_000_000)).unwrap_err();
    assert_eq!(
        deposit_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::WalletRecoveryOnly as u32),
        )
    );
    assert_eq!(svm.get_balance(&wallet).unwrap(), wallet_balance);

    let wallet_wsol_ata = Address::new_unique();
    svm.airdrop(&wallet_wsol_ata, 1).unwrap();
    let wrap_error = send_unsigned_tx(
        &mut svm,
        wrap_sol_ix(agent_asset, vault_config, wallet, wallet_wsol_ata, 1_000),
    )
    .unwrap_err();
    assert_eq!(
        wrap_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::WalletRecoveryOnly as u32),
        )
    );

    let execute_error = send_unsigned_tx(
        &mut svm,
        execute_cpi_checked_memo_ix(agent_asset, vault_config, wallet, wallet_balance),
    )
    .unwrap_err();
    assert_eq!(
        execute_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::WalletRecoveryOnly as u32),
        )
    );
}

#[test]
fn recovery_only_wallet_allows_constrained_cleanup_paths() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let (vault_config, wallet) = create_recovery_only_wallet(&mut svm, agent_asset);
    let destination = Address::new_unique();
    svm.airdrop(&destination, 1).unwrap();
    svm.airdrop(&wallet, 2_000).unwrap();
    send_unsigned_tx(
        &mut svm,
        withdraw_sol_ix(agent_asset, wallet, destination, 1_000),
    )
    .unwrap();
    assert_eq!(svm.get_balance(&destination).unwrap(), 1_001);

    let native_reserve = token_account_rent();
    let wallet_wsol_ata = ata_address(&wallet, &NATIVE_MINT, &TOKEN_PROGRAM);
    svm.set_account(
        wallet_wsol_ata,
        Account {
            lamports: native_reserve + 500,
            data: native_token_account_data(NATIVE_MINT, wallet, 500, native_reserve),
            owner: TOKEN_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();
    send_unsigned_tx(&mut svm, unwrap_sol_ix(agent_asset, vault_config, wallet)).unwrap();
    assert!(svm.get_balance(&wallet).unwrap() >= native_reserve + 1_500);

    let mint = Address::new_unique();
    install_mint(&mut svm, mint, TOKEN_PROGRAM, 6);
    let source_ata = ata_address(&wallet, &mint, &TOKEN_PROGRAM);
    let external_destination = Address::new_unique();
    install_token_account(
        &mut svm,
        source_ata,
        TOKEN_PROGRAM,
        token_account_data(mint, wallet, 10),
    );
    install_token_account(
        &mut svm,
        external_destination,
        TOKEN_PROGRAM,
        token_account_data(mint, Address::new_unique(), 0),
    );
    send_unsigned_tx(
        &mut svm,
        transfer_spl_ix(
            agent_asset,
            vault_config,
            wallet,
            mint,
            source_ata,
            external_destination,
            TOKEN_PROGRAM,
            10,
            6,
            0,
        ),
    )
    .unwrap();
    assert_eq!(token_amount(&svm, &external_destination), 10);

    let rent_receiver = Address::new_unique();
    svm.airdrop(&rent_receiver, 1).unwrap();
    send_unsigned_tx(
        &mut svm,
        close_wallet_ata_ix(
            agent_asset,
            vault_config,
            wallet,
            mint,
            TOKEN_PROGRAM,
            rent_receiver,
        ),
    )
    .unwrap();

    install_token_account(
        &mut svm,
        source_ata,
        TOKEN_PROGRAM,
        token_account_data(mint, wallet, 10),
    );
    let wallet_destination = ata_address(&wallet, &mint, &TOKEN_PROGRAM);
    install_token_account(
        &mut svm,
        wallet_destination,
        TOKEN_PROGRAM,
        token_account_data(mint, wallet, 0),
    );
    let wallet_custody_destination = send_unsigned_tx(
        &mut svm,
        transfer_spl_ix(
            agent_asset,
            vault_config,
            wallet,
            mint,
            source_ata,
            wallet_destination,
            TOKEN_PROGRAM,
            1,
            6,
            0,
        ),
    )
    .unwrap_err();
    assert_eq!(
        wallet_custody_destination,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::WalletRecoveryOnly as u32),
        )
    );
}

#[test]
fn cross_agent_wallet_substitution_fails_on_deposit() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_a = Address::new_unique();
    let (_vault_a, wallet_a) = create_agent_vault_and_wallet(&mut svm, agent_a);
    let agent_b = Address::new_unique();
    let (_vault_b, _wallet_b) = create_agent_vault_and_wallet(&mut svm, agent_b);
    let wallet_balance = svm.get_balance(&wallet_a).unwrap();

    let error =
        send_unsigned_tx(&mut svm, deposit_sol_ix(agent_b, wallet_a, 1_000_000)).unwrap_err();

    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AgentVaultError::InvalidPda as u32),
        )
    );
    assert_eq!(svm.get_balance(&wallet_a).unwrap(), wallet_balance);
}

#[test]
fn dusted_system_owned_wallet_pda_can_be_created_and_reopened() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_asset = Address::new_unique();
    let vault_config = create_agent_vault(&mut svm, agent_asset);
    let wallet = wallet_pda(&agent_asset, 0);
    svm.set_account(
        wallet,
        Account {
            lamports: 123_456,
            data: Vec::new(),
            owner: SYSTEM_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();

    send_unsigned_tx(
        &mut svm,
        create_wallet_ix(agent_asset, vault_config, wallet),
    )
    .unwrap();
    let wallet_account = svm.get_account(&wallet).unwrap();
    assert_eq!(wallet_account.owner, PROGRAM_ID);
    assert_eq!(wallet_account.data.len(), WALLET_LEN);
    assert!(wallet_account.lamports > 123_456);
    assert_eq!(
        unpack_wallet(&wallet_account.data).unwrap().flags,
        WALLET_FLAG_ACTIVE
    );

    let rent_receiver = Address::new_unique();
    svm.airdrop(&rent_receiver, 1).unwrap();
    send_unsigned_tx(
        &mut svm,
        close_wallet_ix(agent_asset, vault_config, wallet, rent_receiver),
    )
    .unwrap();
    svm.set_account(
        wallet,
        Account {
            lamports: 123_456,
            data: Vec::new(),
            owner: SYSTEM_PROGRAM,
            ..Default::default()
        },
    )
    .unwrap();

    send_unsigned_tx(
        &mut svm,
        reopen_wallet_for_recovery_ix(agent_asset, vault_config, wallet),
    )
    .unwrap();
    let wallet_account = svm.get_account(&wallet).unwrap();
    assert_eq!(wallet_account.owner, PROGRAM_ID);
    assert_eq!(wallet_account.data.len(), WALLET_LEN);
    assert_eq!(
        unpack_wallet(&wallet_account.data).unwrap().flags,
        WALLET_FLAG_RECOVERY_ONLY
    );
}

#[test]
fn cross_agent_wallet_substitution_fails_on_protected_wallet_ops() {
    let mut svm = runtime();
    initialize_global_config(&mut svm);
    svm.airdrop(&FEE_TREASURY, 1).unwrap();

    let agent_a = Address::new_unique();
    let (_vault_a, wallet_a) = create_agent_vault_and_wallet(&mut svm, agent_a);
    let agent_b = Address::new_unique();
    let (vault_b, wallet_b) = create_agent_vault_and_wallet(&mut svm, agent_b);
    let destination = Address::new_unique();
    let rent_receiver = Address::new_unique();
    svm.airdrop(&destination, 1).unwrap();
    svm.airdrop(&rent_receiver, 1).unwrap();
    let min_wallet_lamports = svm.get_balance(&wallet_b).unwrap();

    for ix in [
        update_wallet_label_ix(agent_b, vault_b, wallet_a),
        withdraw_sol_ix(agent_b, wallet_a, destination, 1),
        transfer_sol_ix(agent_b, 0, wallet_a, 0, wallet_b, 1),
        transfer_sol_ix(agent_b, 0, wallet_b, 0, wallet_a, 1),
        close_wallet_ix(agent_b, vault_b, wallet_a, rent_receiver),
        execute_cpi_checked_memo_ix(agent_b, vault_b, wallet_a, min_wallet_lamports),
    ] {
        let error = send_unsigned_tx(&mut svm, ix).unwrap_err();
        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(AgentVaultError::InvalidPda as u32),
            )
        );
    }
}
