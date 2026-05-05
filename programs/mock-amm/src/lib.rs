#![cfg_attr(not(test), no_std)]

use pinocchio::{
    cpi::{invoke_signed, Seed, Signer},
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    AccountView, Address, ProgramResult,
};

const POOL_AUTHORITY_SEED: &[u8] = b"pool_authority";
const POOL_AUTHORITY_BUMP: u8 = 255;
const PDA_MARKER: &[u8; 21] = b"ProgramDerivedAddress";

#[cfg(not(feature = "no-entrypoint"))]
pinocchio::program_entrypoint!(process_instruction);
#[cfg(not(feature = "no-entrypoint"))]
pinocchio::default_allocator!();
#[cfg(not(feature = "no-entrypoint"))]
pinocchio::nostd_panic_handler!();

pub fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    data: &[u8],
) -> ProgramResult {
    if data.len() == 1 && data[0] == 0 {
        return Ok(());
    }
    if accounts.len() == 5 && data.len() == 10 && data[0] == 1 {
        return approve_delegate_checked(accounts, data);
    }
    if accounts.len() == 5 && data.len() == 1 && data[0] == 2 {
        return set_account_owner_to_wallet(accounts);
    }
    if accounts.len() == 6 && data.len() == 18 && data[0] == 3 {
        return transfer_checked_with_fee(accounts, data);
    }
    if accounts.len() == 8 && data.len() == 18 {
        return swap(accounts, data);
    }
    if accounts.len() == 9 && data.len() == 18 {
        return swap_with_pool_authority(program_id, accounts, data);
    }

    Err(ProgramError::InvalidInstructionData)
}

fn swap(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let wallet = account(accounts, 0)?;
    let user_input = account(accounts, 1)?;
    let pool_input = account(accounts, 2)?;
    let pool_output = account(accounts, 3)?;
    let user_output = account(accounts, 4)?;
    let input_mint = account(accounts, 5)?;
    let output_mint = account(accounts, 6)?;
    let token_program = account(accounts, 7)?;

    let amount_in = read_u64_le(data, 0)?;
    let amount_out = read_u64_le(data, 8)?;
    let input_decimals = data[16];
    let output_decimals = data[17];

    transfer_checked(
        token_program,
        user_input,
        input_mint,
        pool_input,
        wallet,
        amount_in,
        input_decimals,
    )?;
    transfer_checked(
        token_program,
        pool_output,
        output_mint,
        user_output,
        wallet,
        amount_out,
        output_decimals,
    )
}

fn swap_with_pool_authority(
    program_id: &Address,
    accounts: &[AccountView],
    data: &[u8],
) -> ProgramResult {
    let wallet = account(accounts, 0)?;
    let user_input = account(accounts, 1)?;
    let pool_input = account(accounts, 2)?;
    let pool_output = account(accounts, 3)?;
    let user_output = account(accounts, 4)?;
    let pool_authority = account(accounts, 5)?;
    let input_mint = account(accounts, 6)?;
    let output_mint = account(accounts, 7)?;
    let token_program = account(accounts, 8)?;

    let amount_in = read_u64_le(data, 0)?;
    let amount_out = read_u64_le(data, 8)?;
    let input_decimals = data[16];
    let output_decimals = data[17];

    let expected_pool_authority = derive_pool_authority(program_id);
    if pool_authority.address() != &expected_pool_authority {
        return Err(ProgramError::InvalidSeeds);
    }

    transfer_checked(
        token_program,
        user_input,
        input_mint,
        pool_input,
        wallet,
        amount_in,
        input_decimals,
    )?;

    let bump_seed = [POOL_AUTHORITY_BUMP];
    let seeds = [Seed::from(POOL_AUTHORITY_SEED), Seed::from(&bump_seed)];
    let signer = Signer::from(&seeds);
    transfer_checked_with_signers(
        token_program,
        pool_output,
        output_mint,
        user_output,
        pool_authority,
        amount_out,
        output_decimals,
        &[signer],
    )
}

fn derive_pool_authority(program_id: &Address) -> Address {
    let bump_seed = [POOL_AUTHORITY_BUMP];
    let hash = solana_sha256_hasher::hashv(&[
        POOL_AUTHORITY_SEED,
        &bump_seed,
        program_id.as_ref(),
        PDA_MARKER,
    ]);
    Address::new_from_array(hash.to_bytes())
}

fn approve_delegate_checked(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let wallet = account(accounts, 0)?;
    let source = account(accounts, 1)?;
    let mint = account(accounts, 2)?;
    let delegate = account(accounts, 3)?;
    let token_program = account(accounts, 4)?;
    let amount = read_u64_le(data, 1)?;
    let decimals = data[9];

    let metas = [
        InstructionAccount::writable(source.address()),
        InstructionAccount::readonly(mint.address()),
        InstructionAccount::readonly(delegate.address()),
        InstructionAccount::readonly_signer(wallet.address()),
    ];
    let mut ix_data = [0u8; 10];
    ix_data[0] = 13;
    ix_data[1..9].copy_from_slice(&amount.to_le_bytes());
    ix_data[9] = decimals;
    let instruction = InstructionView {
        program_id: token_program.address(),
        accounts: &metas,
        data: &ix_data,
    };
    invoke_signed(&instruction, &[source, mint, delegate, wallet], &[])
}

fn set_account_owner_to_wallet(accounts: &[AccountView]) -> ProgramResult {
    let wallet = account(accounts, 0)?;
    let token_account = account(accounts, 1)?;
    let current_authority = account(accounts, 3)?;
    let token_program = account(accounts, 4)?;

    let metas = [
        InstructionAccount::writable(token_account.address()),
        InstructionAccount::readonly_signer(current_authority.address()),
    ];
    let mut ix_data = [0u8; 35];
    ix_data[0] = 6;
    ix_data[1] = 2;
    ix_data[2] = 1;
    ix_data[3..35].copy_from_slice(wallet.address().as_ref());
    let instruction = InstructionView {
        program_id: token_program.address(),
        accounts: &metas,
        data: &ix_data,
    };
    invoke_signed(&instruction, &[token_account, current_authority], &[])
}

fn transfer_checked_with_fee(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let source = account(accounts, 1)?;
    let mint = account(accounts, 2)?;
    let destination = account(accounts, 3)?;
    let authority = account(accounts, 4)?;
    let token_program = account(accounts, 5)?;
    let amount = read_u64_le(data, 1)?;
    let decimals = data[9];
    let fee = read_u64_le(data, 10)?;

    let metas = [
        InstructionAccount::writable(source.address()),
        InstructionAccount::readonly(mint.address()),
        InstructionAccount::writable(destination.address()),
        InstructionAccount::readonly_signer(authority.address()),
    ];
    let mut ix_data = [0u8; 19];
    ix_data[0] = 26;
    ix_data[1] = 1;
    ix_data[2..10].copy_from_slice(&amount.to_le_bytes());
    ix_data[10] = decimals;
    ix_data[11..19].copy_from_slice(&fee.to_le_bytes());
    let instruction = InstructionView {
        program_id: token_program.address(),
        accounts: &metas,
        data: &ix_data,
    };
    invoke_signed(&instruction, &[source, mint, destination, authority], &[])
}

fn transfer_checked(
    token_program: &AccountView,
    source: &AccountView,
    mint: &AccountView,
    destination: &AccountView,
    authority: &AccountView,
    amount: u64,
    decimals: u8,
) -> ProgramResult {
    transfer_checked_with_signers(
        token_program,
        source,
        mint,
        destination,
        authority,
        amount,
        decimals,
        &[],
    )
}

fn transfer_checked_with_signers(
    token_program: &AccountView,
    source: &AccountView,
    mint: &AccountView,
    destination: &AccountView,
    authority: &AccountView,
    amount: u64,
    decimals: u8,
    signers: &[Signer],
) -> ProgramResult {
    let metas = [
        InstructionAccount::writable(source.address()),
        InstructionAccount::readonly(mint.address()),
        InstructionAccount::writable(destination.address()),
        InstructionAccount::readonly_signer(authority.address()),
    ];
    let mut data = [0u8; 10];
    data[0] = 12;
    data[1..9].copy_from_slice(&amount.to_le_bytes());
    data[9] = decimals;
    let instruction = InstructionView {
        program_id: token_program.address(),
        accounts: &metas,
        data: &data,
    };
    invoke_signed(
        &instruction,
        &[source, mint, destination, authority],
        signers,
    )
}

fn account(accounts: &[AccountView], index: usize) -> Result<&AccountView, ProgramError> {
    accounts
        .get(index)
        .ok_or(ProgramError::NotEnoughAccountKeys)
}

fn read_u64_le(input: &[u8], offset: usize) -> Result<u64, ProgramError> {
    let bytes = input
        .get(offset..offset + 8)
        .ok_or(ProgramError::InvalidInstructionData)?;
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}
