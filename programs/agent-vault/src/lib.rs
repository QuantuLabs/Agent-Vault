#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod agent_account;
pub mod constants;
pub mod core_asset;
pub mod cpi_plan;
pub mod error;
pub mod instruction;
pub mod pda;
pub mod processor;
pub mod state;
pub mod token_state;
pub mod validation;

#[cfg(kani)]
mod kani_harness;

use pinocchio::{AccountView, Address, ProgramResult};

pub const ID: Address = Address::new_from_array([
    31, 58, 50, 151, 90, 236, 84, 30, 255, 202, 201, 121, 39, 106, 94, 18, 48, 251, 31, 19, 233,
    255, 51, 44, 82, 193, 40, 194, 133, 91, 239, 221,
]);

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
    processor::process_instruction(program_id, accounts, data)
}
