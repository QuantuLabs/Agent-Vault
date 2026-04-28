use crate::{
    constants::{MAX_CPI_ACCOUNTS, MAX_POST_CHECKS},
    error::AgentVaultError,
    instruction::{ExecuteCpiChecked, PostCheck},
    state::PUBKEY_LEN,
};
use pinocchio::error::ProgramError;

pub type AccountKey = [u8; PUBKEY_LEN];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpiAccountMeta {
    pub key: AccountKey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl CpiAccountMeta {
    #[inline(always)]
    pub const fn new(key: AccountKey, is_signer: bool, is_writable: bool) -> Self {
        Self {
            key,
            is_signer,
            is_writable,
        }
    }

    #[inline(always)]
    fn flags_match(&self, other: &Self) -> bool {
        self.is_signer == other.is_signer && self.is_writable == other.is_writable
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtectedAccounts {
    pub holder: AccountKey,
    pub global_config: AccountKey,
    pub vault_config: AccountKey,
    pub agent_asset: AccountKey,
    pub target_program: AccountKey,
}

impl ProtectedAccounts {
    #[inline(always)]
    pub fn contains(&self, key: &AccountKey) -> bool {
        key == &self.holder
            || key == &self.global_config
            || key == &self.vault_config
            || key == &self.agent_asset
            || key == &self.target_program
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlannedAccount {
    Wallet,
    Remaining { index: usize },
}

pub fn validate_execute_cpi_plan(
    ix: &ExecuteCpiChecked<'_>,
    target_remaining_accounts: &[CpiAccountMeta],
    protected_accounts: &ProtectedAccounts,
    wallet_key: &AccountKey,
) -> Result<(), ProgramError> {
    validate_remaining_account_count(ix.target_account_count, target_remaining_accounts.len())?;
    validate_wallet_meta_index(ix.wallet_meta_index, ix.target_account_count)?;
    validate_duplicate_policy(target_remaining_accounts, protected_accounts, wallet_key)?;
    validate_execute_cpi_post_check_indexes(ix, target_remaining_accounts, wallet_key)?;
    Ok(())
}

pub fn validate_remaining_account_count(
    target_account_count: u8,
    remaining_account_count: usize,
) -> Result<(), ProgramError> {
    validate_target_account_count(target_account_count)?;
    if remaining_account_count != target_account_count as usize {
        return Err(AgentVaultError::InvalidCpiAccounts.into());
    }
    Ok(())
}

pub fn validate_wallet_meta_index(
    wallet_meta_index: u8,
    target_account_count: u8,
) -> Result<(), ProgramError> {
    validate_target_account_count(target_account_count)?;
    if wallet_meta_index > target_account_count {
        return Err(AgentVaultError::InvalidCpiAccounts.into());
    }
    Ok(())
}

pub fn final_cpi_account_count(target_account_count: u8) -> Result<usize, ProgramError> {
    validate_target_account_count(target_account_count)?;
    Ok(target_account_count as usize + 1)
}

pub fn planned_account_at(
    final_account_index: u8,
    wallet_meta_index: u8,
    target_account_count: u8,
) -> Result<PlannedAccount, ProgramError> {
    validate_wallet_meta_index(wallet_meta_index, target_account_count)?;
    if final_account_index > target_account_count {
        return Err(AgentVaultError::InvalidPostCheck.into());
    }
    if final_account_index == wallet_meta_index {
        Ok(PlannedAccount::Wallet)
    } else if final_account_index < wallet_meta_index {
        Ok(PlannedAccount::Remaining {
            index: final_account_index as usize,
        })
    } else {
        Ok(PlannedAccount::Remaining {
            index: final_account_index as usize - 1,
        })
    }
}

pub fn final_account_meta_at(
    final_account_index: u8,
    wallet_meta_index: u8,
    target_account_count: u8,
    target_remaining_accounts: &[CpiAccountMeta],
    wallet_key: &AccountKey,
) -> Result<CpiAccountMeta, ProgramError> {
    validate_remaining_account_count(target_account_count, target_remaining_accounts.len())?;
    match planned_account_at(final_account_index, wallet_meta_index, target_account_count)? {
        PlannedAccount::Wallet => Ok(CpiAccountMeta::new(*wallet_key, true, false)),
        PlannedAccount::Remaining { index } => target_remaining_accounts
            .get(index)
            .copied()
            .ok_or(AgentVaultError::InvalidPostCheck.into()),
    }
}

pub fn validate_duplicate_policy(
    target_remaining_accounts: &[CpiAccountMeta],
    protected_accounts: &ProtectedAccounts,
    wallet_key: &AccountKey,
) -> Result<(), ProgramError> {
    if target_remaining_accounts.len() > MAX_CPI_ACCOUNTS as usize {
        return Err(AgentVaultError::AccountLimitExceeded.into());
    }

    let mut i = 0;
    while i < target_remaining_accounts.len() {
        let account = &target_remaining_accounts[i];
        if &account.key == wallet_key || protected_accounts.contains(&account.key) {
            return Err(AgentVaultError::DuplicateAccount.into());
        }

        let mut j = i + 1;
        while j < target_remaining_accounts.len() {
            let other = &target_remaining_accounts[j];
            if account.key == other.key && !account.flags_match(other) {
                return Err(AgentVaultError::DuplicateAccount.into());
            }
            j += 1;
        }
        i += 1;
    }

    Ok(())
}

pub fn validate_execute_cpi_post_check_indexes(
    ix: &ExecuteCpiChecked<'_>,
    target_remaining_accounts: &[CpiAccountMeta],
    wallet_key: &AccountKey,
) -> Result<(), ProgramError> {
    validate_remaining_account_count(ix.target_account_count, target_remaining_accounts.len())?;
    validate_wallet_meta_index(ix.wallet_meta_index, ix.target_account_count)?;
    if ix.post_check_count == 0 || ix.post_check_count > MAX_POST_CHECKS {
        return Err(AgentVaultError::InvalidPostCheck.into());
    }

    let mut has_economic_bound = false;
    let mut post_checks = ix.post_checks();
    while let Some(check) = post_checks.next_check()? {
        has_economic_bound |= is_economic_balance_bound(&check);
        validate_post_check_account_indexes(
            &check,
            ix.wallet_meta_index,
            ix.target_account_count,
            target_remaining_accounts,
            wallet_key,
        )?;
    }

    if !has_economic_bound {
        return Err(AgentVaultError::MissingEconomicPostCheck.into());
    }
    Ok(())
}

pub fn validate_post_check_account_indexes(
    check: &PostCheck,
    wallet_meta_index: u8,
    target_account_count: u8,
    target_remaining_accounts: &[CpiAccountMeta],
    wallet_key: &AccountKey,
) -> Result<(), ProgramError> {
    validate_remaining_account_count(target_account_count, target_remaining_accounts.len())?;
    validate_wallet_meta_index(wallet_meta_index, target_account_count)?;

    match check {
        PostCheck::SolBalanceMin { account_index, .. }
        | PostCheck::SolBalanceMax { account_index, .. }
        | PostCheck::SolIncreaseMin { account_index, .. }
        | PostCheck::SolDecreaseMax { account_index, .. }
        | PostCheck::AccountOwnerEquals { account_index, .. }
        | PostCheck::AccountStateEquals { account_index, .. } => validate_post_check_index(
            *account_index,
            wallet_meta_index,
            target_account_count,
            target_remaining_accounts,
            wallet_key,
        ),
        PostCheck::TokenBalanceMin {
            token_account_index,
            mint_account_index,
            ..
        }
        | PostCheck::TokenBalanceMax {
            token_account_index,
            mint_account_index,
            ..
        }
        | PostCheck::TokenIncreaseMin {
            token_account_index,
            mint_account_index,
            ..
        }
        | PostCheck::TokenDecreaseMax {
            token_account_index,
            mint_account_index,
            ..
        }
        | PostCheck::TokenCustodyUnchanged {
            token_account_index,
            mint_account_index,
        }
        | PostCheck::TokenCustodyEquals {
            token_account_index,
            mint_account_index,
            ..
        } => {
            validate_post_check_index(
                *token_account_index,
                wallet_meta_index,
                target_account_count,
                target_remaining_accounts,
                wallet_key,
            )?;
            validate_post_check_index(
                *mint_account_index,
                wallet_meta_index,
                target_account_count,
                target_remaining_accounts,
                wallet_key,
            )
        }
        PostCheck::TokenAuthorityEquals {
            token_account_index,
            ..
        } => validate_post_check_index(
            *token_account_index,
            wallet_meta_index,
            target_account_count,
            target_remaining_accounts,
            wallet_key,
        ),
    }
}

#[inline(always)]
pub fn is_economic_balance_bound(check: &PostCheck) -> bool {
    check.is_economic_balance_bound()
}

fn validate_post_check_index(
    final_account_index: u8,
    wallet_meta_index: u8,
    target_account_count: u8,
    target_remaining_accounts: &[CpiAccountMeta],
    wallet_key: &AccountKey,
) -> Result<(), ProgramError> {
    let checked_key = final_account_key_at(
        final_account_index,
        wallet_meta_index,
        target_account_count,
        target_remaining_accounts,
        wallet_key,
    )?;

    if count_final_account_key(&checked_key, target_remaining_accounts, wallet_key) != 1 {
        return Err(AgentVaultError::InvalidPostCheck.into());
    }
    Ok(())
}

fn final_account_key_at(
    final_account_index: u8,
    wallet_meta_index: u8,
    target_account_count: u8,
    target_remaining_accounts: &[CpiAccountMeta],
    wallet_key: &AccountKey,
) -> Result<AccountKey, ProgramError> {
    Ok(final_account_meta_at(
        final_account_index,
        wallet_meta_index,
        target_account_count,
        target_remaining_accounts,
        wallet_key,
    )?
    .key)
}

fn count_final_account_key(
    key: &AccountKey,
    target_remaining_accounts: &[CpiAccountMeta],
    wallet_key: &AccountKey,
) -> usize {
    let mut count = 0;
    if key == wallet_key {
        count += 1;
    }

    let mut i = 0;
    while i < target_remaining_accounts.len() {
        if &target_remaining_accounts[i].key == key {
            count += 1;
        }
        i += 1;
    }
    count
}

#[inline(always)]
fn validate_target_account_count(target_account_count: u8) -> Result<(), ProgramError> {
    if target_account_count > MAX_CPI_ACCOUNTS {
        Err(AgentVaultError::AccountLimitExceeded.into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::{OptionalPubkey, TokenProgramKind};

    fn key(value: u8) -> AccountKey {
        [value; PUBKEY_LEN]
    }

    fn meta(value: u8, is_signer: bool, is_writable: bool) -> CpiAccountMeta {
        CpiAccountMeta::new(key(value), is_signer, is_writable)
    }

    fn protected() -> ProtectedAccounts {
        ProtectedAccounts {
            holder: key(1),
            global_config: key(2),
            vault_config: key(3),
            agent_asset: key(4),
            target_program: key(5),
        }
    }

    fn assert_custom_error<T>(result: Result<T, ProgramError>, expected: AgentVaultError) {
        match result {
            Err(ProgramError::Custom(actual)) => assert_eq!(actual, expected as u32),
            Err(other) => panic!("unexpected error: {:?}", other),
            Ok(_) => panic!("expected error {:?}", expected),
        }
    }

    #[test]
    fn maps_final_cpi_indexes_around_wallet_insertion() {
        assert_eq!(final_cpi_account_count(3).unwrap(), 4);
        assert_eq!(
            planned_account_at(0, 1, 3).unwrap(),
            PlannedAccount::Remaining { index: 0 }
        );
        assert_eq!(planned_account_at(1, 1, 3).unwrap(), PlannedAccount::Wallet);
        assert_eq!(
            planned_account_at(2, 1, 3).unwrap(),
            PlannedAccount::Remaining { index: 1 }
        );
        assert_eq!(
            planned_account_at(3, 1, 3).unwrap(),
            PlannedAccount::Remaining { index: 2 }
        );
        assert_custom_error(
            planned_account_at(4, 1, 3),
            AgentVaultError::InvalidPostCheck,
        );
    }

    #[test]
    fn final_meta_inserts_readonly_wallet_signer_and_preserves_outer_flags() {
        let wallet = key(9);
        let accounts = [
            meta(10, false, true),
            meta(11, true, false),
            meta(12, false, false),
        ];

        assert_eq!(
            final_account_meta_at(1, 1, 3, &accounts, &wallet).unwrap(),
            CpiAccountMeta::new(wallet, true, false)
        );
        assert_eq!(
            final_account_meta_at(0, 1, 3, &accounts, &wallet).unwrap(),
            accounts[0]
        );
        assert_eq!(
            final_account_meta_at(2, 1, 3, &accounts, &wallet).unwrap(),
            accounts[1]
        );
    }

    #[test]
    fn rejects_remaining_count_mismatch_and_invalid_wallet_meta_index() {
        let accounts = [meta(10, false, false), meta(11, false, true)];
        assert_custom_error(
            validate_remaining_account_count(3, accounts.len()),
            AgentVaultError::InvalidCpiAccounts,
        );
        assert_custom_error(
            validate_wallet_meta_index(3, 2),
            AgentVaultError::InvalidCpiAccounts,
        );
        assert!(validate_remaining_account_count(0, 0).is_ok());
        assert_eq!(final_cpi_account_count(0).unwrap(), 1);
    }

    #[test]
    fn duplicate_policy_rejects_protected_and_wallet_accounts() {
        let protected = protected();
        let wallet = key(9);

        assert_custom_error(
            validate_duplicate_policy(&[meta(1, false, false)], &protected, &wallet),
            AgentVaultError::DuplicateAccount,
        );
        assert_custom_error(
            validate_duplicate_policy(
                &[CpiAccountMeta::new(wallet, false, false)],
                &protected,
                &wallet,
            ),
            AgentVaultError::DuplicateAccount,
        );
    }

    #[test]
    fn duplicate_policy_allows_only_identical_flags_for_unprotected_duplicates() {
        let protected = protected();
        let wallet = key(9);

        assert!(validate_duplicate_policy(
            &[meta(10, false, true), meta(10, false, true)],
            &protected,
            &wallet,
        )
        .is_ok());
        assert_custom_error(
            validate_duplicate_policy(
                &[meta(10, false, true), meta(10, true, true)],
                &protected,
                &wallet,
            ),
            AgentVaultError::DuplicateAccount,
        );
        assert_custom_error(
            validate_duplicate_policy(
                &[meta(10, false, true), meta(10, false, false)],
                &protected,
                &wallet,
            ),
            AgentVaultError::DuplicateAccount,
        );
    }

    #[test]
    fn post_check_indexes_are_bounded_by_final_cpi_list() {
        let wallet = key(9);
        let accounts = [meta(10, false, false), meta(11, false, true)];
        let check = PostCheck::SolBalanceMin {
            account_index: 3,
            min_lamports: 1,
        };

        assert_custom_error(
            validate_post_check_account_indexes(&check, 1, 2, &accounts, &wallet),
            AgentVaultError::InvalidPostCheck,
        );
    }

    #[test]
    fn post_check_referenced_account_must_be_present_once() {
        let wallet = key(9);
        let accounts = [meta(10, false, true), meta(10, false, true)];
        let check = PostCheck::SolBalanceMin {
            account_index: 0,
            min_lamports: 1,
        };

        assert!(validate_duplicate_policy(&accounts, &protected(), &wallet).is_ok());
        assert_custom_error(
            validate_post_check_account_indexes(&check, 1, 2, &accounts, &wallet),
            AgentVaultError::InvalidPostCheck,
        );

        let wallet_check = PostCheck::SolBalanceMin {
            account_index: 1,
            min_lamports: 1,
        };
        assert!(
            validate_post_check_account_indexes(&wallet_check, 1, 2, &accounts, &wallet).is_ok()
        );
    }

    #[test]
    fn token_post_check_indexes_follow_wallet_insertion() {
        let wallet = key(9);
        let accounts = [meta(10, false, true), meta(11, false, false)];
        let check = PostCheck::TokenBalanceMin {
            token_account_index: 1,
            mint_account_index: 2,
            mint: key(11),
            min_amount: 1,
        };

        assert!(validate_post_check_account_indexes(&check, 0, 2, &accounts, &wallet).is_ok());
    }

    #[test]
    fn validates_full_execute_cpi_plan_from_parser_types() {
        let wallet = key(9);
        let accounts = [meta(10, false, true), meta(11, false, false)];
        let target_ix_data = [7, 8, 9];
        let mut post_check_data = [0u8; 10];
        post_check_data[0] = 0;
        post_check_data[1] = 1;
        post_check_data[2..10].copy_from_slice(&5u64.to_le_bytes());
        let ix = ExecuteCpiChecked {
            index: 2,
            wallet_meta_index: 0,
            target_account_count: 2,
            target_ix_data: &target_ix_data,
            post_check_count: 1,
            post_check_data: &post_check_data,
        };

        assert!(validate_execute_cpi_plan(&ix, &accounts, &protected(), &wallet).is_ok());
    }

    #[test]
    fn economic_detection_delegates_to_instruction_post_check_semantics() {
        assert!(is_economic_balance_bound(&PostCheck::SolDecreaseMax {
            account_index: 0,
            max_lamports_decrease: 1,
        }));
        assert!(is_economic_balance_bound(&PostCheck::TokenIncreaseMin {
            token_account_index: 0,
            mint_account_index: 1,
            mint: key(11),
            min_amount_increase: 1,
        }));
        assert!(!is_economic_balance_bound(&PostCheck::AccountOwnerEquals {
            account_index: 0,
            owner: key(12),
        }));
        assert!(!is_economic_balance_bound(&PostCheck::AccountStateEquals {
            account_index: 0,
            owner: key(12),
            lamports: 1,
            data_len: 0,
            data_hash: [0u8; 32],
        }));
        assert!(!is_economic_balance_bound(
            &PostCheck::TokenAuthorityEquals {
                token_account_index: 0,
                authority: key(12),
            }
        ));
        assert!(!is_economic_balance_bound(
            &PostCheck::TokenCustodyUnchanged {
                token_account_index: 0,
                mint_account_index: 1,
            }
        ));
        assert!(!is_economic_balance_bound(&PostCheck::TokenCustodyEquals {
            token_account_index: 0,
            mint_account_index: 1,
            token_program_kind: TokenProgramKind::Tokenkeg,
            mint: key(11),
            authority: key(12),
            close_authority: OptionalPubkey::None,
            delegate: OptionalPubkey::None,
            state: 1,
            extension_data_hash: [0u8; 32],
        }));
    }
}
