use crate::{
    constants::{
        ASSOCIATED_TOKEN_PROGRAM_ID, CLOCK_SYSVAR_ID, METAPLEX_CORE_PROGRAM_ID, NATIVE_MINT_ID,
        RENT_SYSVAR_ID, SYSTEM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID,
    },
    error::AgentVaultError,
};
use pinocchio::{error::ProgramError, AccountView, Address};

#[inline(always)]
pub fn assert_address(account: &AccountView, expected: &Address) -> Result<(), ProgramError> {
    if account.address() != expected {
        return Err(AgentVaultError::InvalidProgramId.into());
    }
    Ok(())
}

#[inline(always)]
pub fn assert_sysvar_address(
    account: &AccountView,
    expected: &Address,
) -> Result<(), ProgramError> {
    if account.address() != expected {
        return Err(AgentVaultError::InvalidSysvar.into());
    }
    Ok(())
}

#[inline(always)]
pub fn assert_system_program(account: &AccountView) -> Result<(), ProgramError> {
    assert_address(account, &SYSTEM_PROGRAM_ID)
}

#[inline(always)]
pub fn assert_rent_sysvar(account: &AccountView) -> Result<(), ProgramError> {
    assert_sysvar_address(account, &RENT_SYSVAR_ID)
}

#[inline(always)]
pub fn assert_clock_sysvar(account: &AccountView) -> Result<(), ProgramError> {
    assert_sysvar_address(account, &CLOCK_SYSVAR_ID)
}

#[inline(always)]
pub fn assert_associated_token_program(account: &AccountView) -> Result<(), ProgramError> {
    assert_address(account, &ASSOCIATED_TOKEN_PROGRAM_ID)
}

#[inline(always)]
pub fn assert_token_program(account: &AccountView) -> Result<(), ProgramError> {
    if account.address() != &TOKEN_PROGRAM_ID {
        return Err(AgentVaultError::InvalidTokenProgram.into());
    }
    Ok(())
}

#[inline(always)]
pub fn assert_token_2022_program(account: &AccountView) -> Result<(), ProgramError> {
    if account.address() != &TOKEN_2022_PROGRAM_ID {
        return Err(AgentVaultError::InvalidTokenProgram.into());
    }
    Ok(())
}

#[inline(always)]
pub fn assert_token_program_any(account: &AccountView) -> Result<(), ProgramError> {
    if account.address() != &TOKEN_PROGRAM_ID && account.address() != &TOKEN_2022_PROGRAM_ID {
        return Err(AgentVaultError::InvalidTokenProgram.into());
    }
    Ok(())
}

#[inline(always)]
pub fn assert_native_mint(account: &AccountView) -> Result<(), ProgramError> {
    if account.address() != &NATIVE_MINT_ID {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }
    Ok(())
}

#[inline(always)]
pub fn assert_metaplex_core_program(account: &AccountView) -> Result<(), ProgramError> {
    assert_address(account, &METAPLEX_CORE_PROGRAM_ID)
}

#[inline(always)]
pub fn assert_signer(account: &AccountView) -> Result<(), ProgramError> {
    if !account.is_signer() {
        return Err(AgentVaultError::InvalidSigner.into());
    }
    Ok(())
}

#[inline(always)]
pub fn assert_not_signer(account: &AccountView) -> Result<(), ProgramError> {
    if account.is_signer() {
        return Err(AgentVaultError::InvalidSigner.into());
    }
    Ok(())
}

#[inline(always)]
pub fn assert_writable(account: &AccountView) -> Result<(), ProgramError> {
    if !account.is_writable() {
        return Err(AgentVaultError::InvalidWritable.into());
    }
    Ok(())
}

#[inline(always)]
pub fn assert_readonly(account: &AccountView) -> Result<(), ProgramError> {
    if account.is_writable() {
        return Err(AgentVaultError::InvalidWritable.into());
    }
    Ok(())
}

#[inline(always)]
pub fn assert_owned_by(account: &AccountView, owner: &Address) -> Result<(), ProgramError> {
    if !account.owned_by(owner) {
        return Err(AgentVaultError::InvalidOwner.into());
    }
    Ok(())
}

#[inline(always)]
pub fn assert_data_len(account: &AccountView, expected_len: usize) -> Result<(), ProgramError> {
    if account.data_len() != expected_len {
        return Err(AgentVaultError::InvalidAccountData.into());
    }
    Ok(())
}

#[inline(always)]
pub fn assert_min_data_len(account: &AccountView, min_len: usize) -> Result<(), ProgramError> {
    if account.data_len() < min_len {
        return Err(AgentVaultError::InvalidAccountData.into());
    }
    Ok(())
}

#[inline(always)]
pub fn is_system_owned_zero_data(account: &AccountView) -> bool {
    account.owned_by(&SYSTEM_PROGRAM_ID) && account.is_data_empty()
}

#[inline(always)]
pub fn assert_system_owned_zero_data(account: &AccountView) -> Result<(), ProgramError> {
    if !account.owned_by(&SYSTEM_PROGRAM_ID) {
        return Err(AgentVaultError::InvalidOwner.into());
    }
    if !account.is_data_empty() {
        return Err(AgentVaultError::InvalidAccountData.into());
    }
    Ok(())
}

#[inline(always)]
pub fn assert_uninitialized_pda(account: &AccountView) -> Result<(), ProgramError> {
    assert_system_owned_zero_data(account)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinocchio::account::{RuntimeAccount, NOT_BORROWED};

    #[repr(C)]
    struct TestAccount<const N: usize> {
        account: RuntimeAccount,
        data: [u8; N],
    }

    impl<const N: usize> TestAccount<N> {
        fn new(address: Address, owner: Address, is_signer: bool, is_writable: bool) -> Self {
            Self {
                account: RuntimeAccount {
                    borrow_state: NOT_BORROWED,
                    is_signer: u8::from(is_signer),
                    is_writable: u8::from(is_writable),
                    executable: 0,
                    resize_delta: 0,
                    address,
                    owner,
                    lamports: 0,
                    data_len: N as u64,
                },
                data: [0; N],
            }
        }

        fn view(&mut self) -> AccountView {
            unsafe { AccountView::new_unchecked(&mut self.account as *mut RuntimeAccount) }
        }
    }

    #[test]
    fn signer_and_writable_validators_use_account_flags() {
        let mut raw = TestAccount::<0>::new(
            Address::new_from_array([1; 32]),
            SYSTEM_PROGRAM_ID.clone(),
            true,
            false,
        );
        let account = raw.view();

        assert_eq!(assert_signer(&account), Ok(()));
        assert_eq!(assert_readonly(&account), Ok(()));
        assert_eq!(
            assert_writable(&account),
            Err(AgentVaultError::InvalidWritable.into())
        );
        assert_eq!(
            assert_not_signer(&account),
            Err(AgentVaultError::InvalidSigner.into())
        );
    }

    #[test]
    fn owner_and_data_len_validators_use_account_view_accessors() {
        let owner = Address::new_from_array([2; 32]);
        let mut raw =
            TestAccount::<24>::new(Address::new_from_array([1; 32]), owner.clone(), false, true);
        let account = raw.view();

        assert_eq!(assert_owned_by(&account, &owner), Ok(()));
        assert_eq!(assert_data_len(&account, 24), Ok(()));
        assert_eq!(assert_min_data_len(&account, 8), Ok(()));
        assert_eq!(
            assert_owned_by(&account, &SYSTEM_PROGRAM_ID),
            Err(AgentVaultError::InvalidOwner.into())
        );
        assert_eq!(
            assert_data_len(&account, 25),
            Err(AgentVaultError::InvalidAccountData.into())
        );
    }

    #[test]
    fn uninitialized_pda_requires_system_owner_and_zero_data() {
        let mut raw = TestAccount::<0>::new(
            Address::new_from_array([1; 32]),
            SYSTEM_PROGRAM_ID.clone(),
            false,
            true,
        );
        let account = raw.view();
        assert!(is_system_owned_zero_data(&account));
        assert_eq!(assert_uninitialized_pda(&account), Ok(()));

        let mut data_raw = TestAccount::<1>::new(
            Address::new_from_array([1; 32]),
            SYSTEM_PROGRAM_ID.clone(),
            false,
            true,
        );
        let data_account = data_raw.view();
        assert!(!is_system_owned_zero_data(&data_account));
        assert_eq!(
            assert_uninitialized_pda(&data_account),
            Err(AgentVaultError::InvalidAccountData.into())
        );

        let mut owner_raw = TestAccount::<0>::new(
            Address::new_from_array([1; 32]),
            Address::new_from_array([9; 32]),
            false,
            true,
        );
        let owner_account = owner_raw.view();
        assert!(!is_system_owned_zero_data(&owner_account));
        assert_eq!(
            assert_uninitialized_pda(&owner_account),
            Err(AgentVaultError::InvalidOwner.into())
        );
    }

    #[test]
    fn canonical_id_validators_check_address_only() {
        let mut system = TestAccount::<0>::new(
            SYSTEM_PROGRAM_ID.clone(),
            Address::new_from_array([7; 32]),
            false,
            false,
        );
        let account = system.view();
        assert_eq!(assert_system_program(&account), Ok(()));

        let mut wrong = TestAccount::<0>::new(
            Address::new_from_array([7; 32]),
            SYSTEM_PROGRAM_ID.clone(),
            false,
            false,
        );
        let wrong_account = wrong.view();
        assert_eq!(
            assert_system_program(&wrong_account),
            Err(AgentVaultError::InvalidProgramId.into())
        );
    }
}
