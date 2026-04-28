use crate::{
    constants::{LABEL_LEN, MAX_CPI_ACCOUNTS, MAX_CPI_IX_DATA_LEN, MAX_POST_CHECKS},
    error::AgentVaultError,
    state::{read_u16_le, read_u64_le, PUBKEY_LEN},
};
use pinocchio::error::ProgramError;

pub const TAG_INITIALIZE_GLOBAL_CONFIG: u8 = 0;
pub const TAG_INIT_VAULT_CONFIG: u8 = 1;
pub const TAG_CREATE_WALLET: u8 = 2;
pub const TAG_UPDATE_WALLET_LABEL: u8 = 3;
pub const TAG_DEPOSIT_SOL: u8 = 4;
pub const TAG_WITHDRAW_SOL: u8 = 5;
pub const TAG_TRANSFER_SOL: u8 = 6;
pub const TAG_CLOSE_WALLET: u8 = 7;
pub const TAG_REOPEN_WALLET_FOR_RECOVERY: u8 = 8;

pub const TAG_CREATE_WALLET_ATA: u8 = 32;
pub const TAG_TRANSFER_SPL: u8 = 33;
pub const TAG_WRAP_SOL: u8 = 34;
pub const TAG_UNWRAP_SOL: u8 = 35;
pub const TAG_CLOSE_WALLET_ATA: u8 = 36;

pub const TAG_EXECUTE_CPI_CHECKED: u8 = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenProgramKind {
    Tokenkeg,
    Token2022,
}

impl TokenProgramKind {
    #[inline(always)]
    pub fn from_u8(value: u8) -> Result<Self, ProgramError> {
        match value {
            0 => Ok(Self::Tokenkeg),
            1 => Ok(Self::Token2022),
            _ => Err(AgentVaultError::InvalidTokenProgram.into()),
        }
    }

    #[inline(always)]
    pub fn as_u8(self) -> u8 {
        match self {
            Self::Tokenkeg => 0,
            Self::Token2022 => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitializeGlobalConfig {
    pub registry_program: [u8; PUBKEY_LEN],
    pub collection: [u8; PUBKEY_LEN],
    pub fee_treasury: [u8; PUBKEY_LEN],
    pub vault_activation_fee_lamports: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CreateWallet {
    pub label: [u8; LABEL_LEN],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UpdateWalletLabel {
    pub index: u16,
    pub label: [u8; LABEL_LEN],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DepositSol {
    pub amount: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WithdrawSol {
    pub index: u16,
    pub amount: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransferSol {
    pub from_index: u16,
    pub to_index: u16,
    pub amount: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReopenWalletForRecovery {
    pub index: u16,
    pub label: [u8; LABEL_LEN],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CreateWalletAta {
    pub index: u16,
    pub token_program_kind: TokenProgramKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransferSpl {
    pub index: u16,
    pub amount: u64,
    pub decimals: u8,
    pub expected_fee: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WrapSol {
    pub index: u16,
    pub amount: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndexedWallet {
    pub index: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecuteCpiChecked<'a> {
    pub index: u16,
    pub wallet_meta_index: u8,
    pub target_account_count: u8,
    pub target_ix_data: &'a [u8],
    pub post_check_count: u8,
    pub post_check_data: &'a [u8],
}

impl<'a> ExecuteCpiChecked<'a> {
    #[inline(always)]
    pub fn post_checks(&self) -> PostCheckIter<'a> {
        PostCheckIter {
            remaining: self.post_check_data,
            remaining_count: self.post_check_count,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Instruction<'a> {
    InitializeGlobalConfig(InitializeGlobalConfig),
    InitVaultConfig,
    CreateWallet(CreateWallet),
    UpdateWalletLabel(UpdateWalletLabel),
    DepositSol(DepositSol),
    WithdrawSol(WithdrawSol),
    TransferSol(TransferSol),
    CloseWallet,
    ReopenWalletForRecovery(ReopenWalletForRecovery),
    CreateWalletAta(CreateWalletAta),
    TransferSpl(TransferSpl),
    WrapSol(WrapSol),
    UnwrapSol(IndexedWallet),
    CloseWalletAta(IndexedWallet),
    ExecuteCpiChecked(ExecuteCpiChecked<'a>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptionalPubkey {
    None,
    Some([u8; PUBKEY_LEN]),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PostCheck {
    SolBalanceMin {
        account_index: u8,
        min_lamports: u64,
    },
    SolBalanceMax {
        account_index: u8,
        max_lamports: u64,
    },
    SolIncreaseMin {
        account_index: u8,
        min_lamports_increase: u64,
    },
    SolDecreaseMax {
        account_index: u8,
        max_lamports_decrease: u64,
    },
    TokenBalanceMin {
        token_account_index: u8,
        mint_account_index: u8,
        mint: [u8; PUBKEY_LEN],
        min_amount: u64,
    },
    TokenBalanceMax {
        token_account_index: u8,
        mint_account_index: u8,
        mint: [u8; PUBKEY_LEN],
        max_amount: u64,
    },
    TokenIncreaseMin {
        token_account_index: u8,
        mint_account_index: u8,
        mint: [u8; PUBKEY_LEN],
        min_amount_increase: u64,
    },
    TokenDecreaseMax {
        token_account_index: u8,
        mint_account_index: u8,
        mint: [u8; PUBKEY_LEN],
        max_amount_decrease: u64,
    },
    TokenAuthorityEquals {
        token_account_index: u8,
        authority: [u8; PUBKEY_LEN],
    },
    TokenCustodyUnchanged {
        token_account_index: u8,
        mint_account_index: u8,
    },
    TokenCustodyEquals {
        token_account_index: u8,
        mint_account_index: u8,
        token_program_kind: TokenProgramKind,
        mint: [u8; PUBKEY_LEN],
        authority: [u8; PUBKEY_LEN],
        close_authority: OptionalPubkey,
        delegate: OptionalPubkey,
        state: u8,
        extension_data_hash: [u8; 32],
    },
    AccountOwnerEquals {
        account_index: u8,
        owner: [u8; PUBKEY_LEN],
    },
    AccountStateEquals {
        account_index: u8,
        owner: [u8; PUBKEY_LEN],
        lamports: u64,
        data_len: u32,
        data_hash: [u8; 32],
    },
}

impl PostCheck {
    #[inline(always)]
    pub fn is_economic_balance_bound(&self) -> bool {
        matches!(
            self,
            Self::SolBalanceMin { .. }
                | Self::SolBalanceMax { .. }
                | Self::SolIncreaseMin { .. }
                | Self::SolDecreaseMax { .. }
                | Self::TokenBalanceMin { .. }
                | Self::TokenBalanceMax { .. }
                | Self::TokenIncreaseMin { .. }
                | Self::TokenDecreaseMax { .. }
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PostCheckIter<'a> {
    remaining: &'a [u8],
    remaining_count: u8,
}

impl<'a> PostCheckIter<'a> {
    pub fn next_check(&mut self) -> Result<Option<PostCheck>, ProgramError> {
        if self.remaining_count == 0 {
            if self.remaining.is_empty() {
                return Ok(None);
            }
            return Err(AgentVaultError::InvalidPostCheck.into());
        }

        let (check, consumed) = parse_post_check(self.remaining)?;
        self.remaining = &self.remaining[consumed..];
        self.remaining_count -= 1;
        Ok(Some(check))
    }
}

pub fn parse_instruction(data: &[u8]) -> Result<Instruction<'_>, ProgramError> {
    let (&tag, payload) = data
        .split_first()
        .ok_or(AgentVaultError::InvalidInstruction)?;

    match tag {
        TAG_INITIALIZE_GLOBAL_CONFIG => parse_initialize_global_config(payload),
        TAG_INIT_VAULT_CONFIG => {
            expect_empty(payload)?;
            Ok(Instruction::InitVaultConfig)
        }
        TAG_CREATE_WALLET => {
            expect_len(payload, LABEL_LEN)?;
            Ok(Instruction::CreateWallet(CreateWallet {
                label: read_label(payload, 0)?,
            }))
        }
        TAG_UPDATE_WALLET_LABEL => {
            expect_len(payload, 2 + LABEL_LEN)?;
            Ok(Instruction::UpdateWalletLabel(UpdateWalletLabel {
                index: read_u16_le(payload, 0)?,
                label: read_label(payload, 2)?,
            }))
        }
        TAG_DEPOSIT_SOL => {
            expect_len(payload, 8)?;
            Ok(Instruction::DepositSol(DepositSol {
                amount: read_u64_le(payload, 0)?,
            }))
        }
        TAG_WITHDRAW_SOL => {
            expect_len(payload, 10)?;
            Ok(Instruction::WithdrawSol(WithdrawSol {
                index: read_u16_le(payload, 0)?,
                amount: read_u64_le(payload, 2)?,
            }))
        }
        TAG_TRANSFER_SOL => {
            expect_len(payload, 12)?;
            Ok(Instruction::TransferSol(TransferSol {
                from_index: read_u16_le(payload, 0)?,
                to_index: read_u16_le(payload, 2)?,
                amount: read_u64_le(payload, 4)?,
            }))
        }
        TAG_CLOSE_WALLET => {
            expect_empty(payload)?;
            Ok(Instruction::CloseWallet)
        }
        TAG_REOPEN_WALLET_FOR_RECOVERY => {
            expect_len(payload, 2 + LABEL_LEN)?;
            Ok(Instruction::ReopenWalletForRecovery(
                ReopenWalletForRecovery {
                    index: read_u16_le(payload, 0)?,
                    label: read_label(payload, 2)?,
                },
            ))
        }
        TAG_CREATE_WALLET_ATA => {
            expect_len(payload, 3)?;
            Ok(Instruction::CreateWalletAta(CreateWalletAta {
                index: read_u16_le(payload, 0)?,
                token_program_kind: TokenProgramKind::from_u8(payload[2])?,
            }))
        }
        TAG_TRANSFER_SPL => {
            expect_len(payload, 19)?;
            Ok(Instruction::TransferSpl(TransferSpl {
                index: read_u16_le(payload, 0)?,
                amount: read_u64_le(payload, 2)?,
                decimals: payload[10],
                expected_fee: read_u64_le(payload, 11)?,
            }))
        }
        TAG_WRAP_SOL => {
            expect_len(payload, 10)?;
            Ok(Instruction::WrapSol(WrapSol {
                index: read_u16_le(payload, 0)?,
                amount: read_u64_le(payload, 2)?,
            }))
        }
        TAG_UNWRAP_SOL => {
            expect_len(payload, 2)?;
            Ok(Instruction::UnwrapSol(IndexedWallet {
                index: read_u16_le(payload, 0)?,
            }))
        }
        TAG_CLOSE_WALLET_ATA => {
            expect_len(payload, 2)?;
            Ok(Instruction::CloseWalletAta(IndexedWallet {
                index: read_u16_le(payload, 0)?,
            }))
        }
        TAG_EXECUTE_CPI_CHECKED => parse_execute_cpi_checked(payload),
        _ => Err(AgentVaultError::UnsupportedInstruction.into()),
    }
}

fn parse_initialize_global_config(payload: &[u8]) -> Result<Instruction<'_>, ProgramError> {
    expect_len(payload, PUBKEY_LEN * 3 + 8)?;
    Ok(Instruction::InitializeGlobalConfig(
        InitializeGlobalConfig {
            registry_program: read_pubkey(payload, 0)?,
            collection: read_pubkey(payload, 32)?,
            fee_treasury: read_pubkey(payload, 64)?,
            vault_activation_fee_lamports: read_u64_le(payload, 96)?,
        },
    ))
}

fn parse_execute_cpi_checked(payload: &[u8]) -> Result<Instruction<'_>, ProgramError> {
    if payload.len() < 7 {
        return Err(AgentVaultError::InvalidInstructionData.into());
    }

    let index = read_u16_le(payload, 0)?;
    let wallet_meta_index = payload[2];
    let target_account_count = payload[3];
    if target_account_count > MAX_CPI_ACCOUNTS {
        return Err(AgentVaultError::AccountLimitExceeded.into());
    }
    if wallet_meta_index > target_account_count {
        return Err(AgentVaultError::InvalidCpiAccounts.into());
    }

    let target_ix_data_len = read_u16_le(payload, 4)? as usize;
    if target_ix_data_len > MAX_CPI_IX_DATA_LEN {
        return Err(AgentVaultError::DataLimitExceeded.into());
    }

    let target_ix_start = 6;
    let target_ix_end = target_ix_start + target_ix_data_len;
    if payload.len() < target_ix_end + 1 {
        return Err(AgentVaultError::InvalidInstructionData.into());
    }
    let target_ix_data = &payload[target_ix_start..target_ix_end];
    let post_check_count = payload[target_ix_end];
    if post_check_count == 0 || post_check_count > MAX_POST_CHECKS {
        return Err(AgentVaultError::InvalidPostCheck.into());
    }
    let post_check_data = &payload[target_ix_end + 1..];
    validate_post_checks(post_check_count, post_check_data)?;

    Ok(Instruction::ExecuteCpiChecked(ExecuteCpiChecked {
        index,
        wallet_meta_index,
        target_account_count,
        target_ix_data,
        post_check_count,
        post_check_data,
    }))
}

pub fn validate_post_checks(count: u8, data: &[u8]) -> Result<(), ProgramError> {
    let mut remaining = data;
    let mut has_economic_bound = false;
    let mut i = 0;
    while i < count {
        let (check, consumed) = parse_post_check(remaining)?;
        has_economic_bound |= check.is_economic_balance_bound();
        remaining = &remaining[consumed..];
        i += 1;
    }

    if !remaining.is_empty() {
        return Err(AgentVaultError::InvalidPostCheck.into());
    }
    if !has_economic_bound {
        return Err(AgentVaultError::MissingEconomicPostCheck.into());
    }
    Ok(())
}

pub fn parse_post_check(data: &[u8]) -> Result<(PostCheck, usize), ProgramError> {
    let tag = *data.first().ok_or(AgentVaultError::InvalidPostCheck)?;
    match tag {
        0 => parse_sol_check(data, PostCheckKind::SolMin),
        1 => parse_sol_check(data, PostCheckKind::SolMax),
        2 => parse_sol_check(data, PostCheckKind::SolIncrease),
        3 => parse_sol_check(data, PostCheckKind::SolDecrease),
        4 => parse_token_amount_check(data, PostCheckKind::TokenMin),
        5 => parse_token_amount_check(data, PostCheckKind::TokenMax),
        6 => parse_token_amount_check(data, PostCheckKind::TokenIncrease),
        7 => parse_token_amount_check(data, PostCheckKind::TokenDecrease),
        8 => {
            require_post_check_len(data, 34)?;
            Ok((
                PostCheck::TokenAuthorityEquals {
                    token_account_index: data[1],
                    authority: read_pubkey(data, 2)?,
                },
                34,
            ))
        }
        9 => {
            require_post_check_len(data, 3)?;
            Ok((
                PostCheck::TokenCustodyUnchanged {
                    token_account_index: data[1],
                    mint_account_index: data[2],
                },
                3,
            ))
        }
        10 => parse_token_custody_equals(data),
        11 => {
            require_post_check_len(data, 34)?;
            Ok((
                PostCheck::AccountOwnerEquals {
                    account_index: data[1],
                    owner: read_pubkey(data, 2)?,
                },
                34,
            ))
        }
        12 => {
            require_post_check_len(data, 78)?;
            Ok((
                PostCheck::AccountStateEquals {
                    account_index: data[1],
                    owner: read_pubkey(data, 2)?,
                    lamports: read_u64_le(data, 34)?,
                    data_len: read_u32_le(data, 42)?,
                    data_hash: read_hash(data, 46)?,
                },
                78,
            ))
        }
        _ => Err(AgentVaultError::InvalidPostCheck.into()),
    }
}

#[derive(Clone, Copy)]
enum PostCheckKind {
    SolMin,
    SolMax,
    SolIncrease,
    SolDecrease,
    TokenMin,
    TokenMax,
    TokenIncrease,
    TokenDecrease,
}

fn parse_sol_check(data: &[u8], kind: PostCheckKind) -> Result<(PostCheck, usize), ProgramError> {
    require_post_check_len(data, 10)?;
    let account_index = data[1];
    let value = read_u64_le(data, 2)?;
    let check = match kind {
        PostCheckKind::SolMin => PostCheck::SolBalanceMin {
            account_index,
            min_lamports: value,
        },
        PostCheckKind::SolMax => PostCheck::SolBalanceMax {
            account_index,
            max_lamports: value,
        },
        PostCheckKind::SolIncrease => PostCheck::SolIncreaseMin {
            account_index,
            min_lamports_increase: value,
        },
        PostCheckKind::SolDecrease => PostCheck::SolDecreaseMax {
            account_index,
            max_lamports_decrease: value,
        },
        _ => return Err(AgentVaultError::InvalidPostCheck.into()),
    };
    Ok((check, 10))
}

fn parse_token_amount_check(
    data: &[u8],
    kind: PostCheckKind,
) -> Result<(PostCheck, usize), ProgramError> {
    require_post_check_len(data, 43)?;
    let token_account_index = data[1];
    let mint_account_index = data[2];
    let mint = read_pubkey(data, 3)?;
    let value = read_u64_le(data, 35)?;
    let check = match kind {
        PostCheckKind::TokenMin => PostCheck::TokenBalanceMin {
            token_account_index,
            mint_account_index,
            mint,
            min_amount: value,
        },
        PostCheckKind::TokenMax => PostCheck::TokenBalanceMax {
            token_account_index,
            mint_account_index,
            mint,
            max_amount: value,
        },
        PostCheckKind::TokenIncrease => PostCheck::TokenIncreaseMin {
            token_account_index,
            mint_account_index,
            mint,
            min_amount_increase: value,
        },
        PostCheckKind::TokenDecrease => PostCheck::TokenDecreaseMax {
            token_account_index,
            mint_account_index,
            mint,
            max_amount_decrease: value,
        },
        _ => return Err(AgentVaultError::InvalidPostCheck.into()),
    };
    Ok((check, 43))
}

fn parse_token_custody_equals(data: &[u8]) -> Result<(PostCheck, usize), ProgramError> {
    require_post_check_len(data, 167)?;
    Ok((
        PostCheck::TokenCustodyEquals {
            token_account_index: data[1],
            mint_account_index: data[2],
            token_program_kind: TokenProgramKind::from_u8(data[3])?,
            mint: read_pubkey(data, 4)?,
            authority: read_pubkey(data, 36)?,
            close_authority: parse_optional_pubkey(data, 68)?,
            delegate: parse_optional_pubkey(data, 101)?,
            state: data[134],
            extension_data_hash: read_hash(data, 135)?,
        },
        167,
    ))
}

fn parse_optional_pubkey(data: &[u8], offset: usize) -> Result<OptionalPubkey, ProgramError> {
    let tag = *data.get(offset).ok_or(AgentVaultError::InvalidPostCheck)?;
    let key = read_pubkey(data, offset + 1)?;
    match tag {
        0 => {
            if key != [0u8; PUBKEY_LEN] {
                return Err(AgentVaultError::InvalidPostCheck.into());
            }
            Ok(OptionalPubkey::None)
        }
        1 => Ok(OptionalPubkey::Some(key)),
        _ => Err(AgentVaultError::InvalidPostCheck.into()),
    }
}

#[inline(always)]
fn read_pubkey(input: &[u8], offset: usize) -> Result<[u8; PUBKEY_LEN], ProgramError> {
    crate::state::read_pubkey(input, offset)
}

#[inline(always)]
fn read_hash(input: &[u8], offset: usize) -> Result<[u8; 32], ProgramError> {
    read_pubkey(input, offset)
}

fn read_u32_le(input: &[u8], offset: usize) -> Result<u32, ProgramError> {
    let bytes = input
        .get(offset..offset + 4)
        .ok_or(AgentVaultError::InvalidInstructionData)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_label(input: &[u8], offset: usize) -> Result<[u8; LABEL_LEN], ProgramError> {
    let bytes = input
        .get(offset..offset + LABEL_LEN)
        .ok_or(AgentVaultError::InvalidInstructionData)?;
    let mut label = [0u8; LABEL_LEN];
    label.copy_from_slice(bytes);
    validate_label(&label)?;
    Ok(label)
}

pub fn validate_label(label: &[u8; LABEL_LEN]) -> Result<(), ProgramError> {
    let mut end = LABEL_LEN;
    let mut i = 0;
    while i < LABEL_LEN {
        if label[i] == 0 {
            end = i;
            break;
        }
        i += 1;
    }

    let mut j = end;
    while j < LABEL_LEN {
        if label[j] != 0 {
            return Err(AgentVaultError::InvalidLabel.into());
        }
        j += 1;
    }

    core::str::from_utf8(&label[..end]).map_err(|_| AgentVaultError::InvalidLabel)?;
    Ok(())
}

#[inline(always)]
fn expect_empty(data: &[u8]) -> Result<(), ProgramError> {
    if data.is_empty() {
        Ok(())
    } else {
        Err(AgentVaultError::InvalidInstructionData.into())
    }
}

#[inline(always)]
fn expect_len(data: &[u8], len: usize) -> Result<(), ProgramError> {
    if data.len() == len {
        Ok(())
    } else {
        Err(AgentVaultError::InvalidInstructionData.into())
    }
}

#[inline(always)]
fn require_post_check_len(data: &[u8], len: usize) -> Result<(), ProgramError> {
    if data.len() >= len {
        Ok(())
    } else {
        Err(AgentVaultError::InvalidPostCheck.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_initialize_global_config() {
        let mut data = [0u8; 105];
        data[0] = TAG_INITIALIZE_GLOBAL_CONFIG;
        data[1..33].copy_from_slice(&[1u8; 32]);
        data[33..65].copy_from_slice(&[2u8; 32]);
        data[65..97].copy_from_slice(&[3u8; 32]);
        data[97..105].copy_from_slice(&500_000u64.to_le_bytes());

        match parse_instruction(&data).unwrap() {
            Instruction::InitializeGlobalConfig(ix) => {
                assert_eq!(ix.registry_program, [1u8; 32]);
                assert_eq!(ix.collection, [2u8; 32]);
                assert_eq!(ix.fee_treasury, [3u8; 32]);
                assert_eq!(ix.vault_activation_fee_lamports, 500_000);
            }
            _ => panic!("wrong instruction"),
        }
    }

    #[test]
    fn rejects_labels_with_nonzero_suffix_after_nul() {
        let mut label = [0u8; LABEL_LEN];
        label[0] = b'a';
        label[2] = b'b';
        assert!(validate_label(&label).is_err());
    }

    #[test]
    fn parses_execute_cpi_checked_and_post_check_iterator() {
        let mut data = [0u8; 1 + 2 + 1 + 1 + 2 + 3 + 1 + 10];
        data[0] = TAG_EXECUTE_CPI_CHECKED;
        data[1..3].copy_from_slice(&7u16.to_le_bytes());
        data[3] = 1;
        data[4] = 2;
        data[5..7].copy_from_slice(&3u16.to_le_bytes());
        data[7..10].copy_from_slice(&[9, 8, 7]);
        data[10] = 1;
        data[11] = 0;
        data[12] = 1;
        data[13..21].copy_from_slice(&42u64.to_le_bytes());

        match parse_instruction(&data).unwrap() {
            Instruction::ExecuteCpiChecked(ix) => {
                assert_eq!(ix.index, 7);
                assert_eq!(ix.wallet_meta_index, 1);
                assert_eq!(ix.target_account_count, 2);
                assert_eq!(ix.target_ix_data, &[9, 8, 7]);
                let mut iter = ix.post_checks();
                assert_eq!(
                    iter.next_check().unwrap(),
                    Some(PostCheck::SolBalanceMin {
                        account_index: 1,
                        min_lamports: 42,
                    })
                );
                assert_eq!(iter.next_check().unwrap(), None);
            }
            _ => panic!("wrong instruction"),
        }
    }

    #[test]
    fn parses_execute_cpi_checked_with_only_wallet_meta() {
        let mut data = [0u8; 1 + 2 + 1 + 1 + 2 + 1 + 10];
        data[0] = TAG_EXECUTE_CPI_CHECKED;
        data[1..3].copy_from_slice(&7u16.to_le_bytes());
        data[3] = 0;
        data[4] = 0;
        data[5..7].copy_from_slice(&0u16.to_le_bytes());
        data[7] = 1;
        data[8] = 0;
        data[9] = 0;
        data[10..18].copy_from_slice(&1u64.to_le_bytes());

        match parse_instruction(&data).unwrap() {
            Instruction::ExecuteCpiChecked(ix) => {
                assert_eq!(ix.wallet_meta_index, 0);
                assert_eq!(ix.target_account_count, 0);
                assert!(ix.target_ix_data.is_empty());
            }
            _ => panic!("wrong instruction"),
        }
    }

    #[test]
    fn rejects_execute_cpi_checked_without_economic_check() {
        let mut data = [0u8; 1 + 2 + 1 + 1 + 2 + 1 + 34];
        data[0] = TAG_EXECUTE_CPI_CHECKED;
        data[3] = 0;
        data[4] = 1;
        data[5..7].copy_from_slice(&0u16.to_le_bytes());
        data[7] = 1;
        data[8] = 11;
        data[9] = 0;
        data[10..42].copy_from_slice(&[1u8; 32]);

        assert!(matches!(
            parse_instruction(&data),
            Err(ProgramError::Custom(x)) if x == AgentVaultError::MissingEconomicPostCheck as u32
        ));
    }

    #[test]
    fn parses_account_state_equals_post_check() {
        let mut check = [0u8; 78];
        check[0] = 12;
        check[1] = 3;
        check[2..34].copy_from_slice(&[4u8; 32]);
        check[34..42].copy_from_slice(&99u64.to_le_bytes());
        check[42..46].copy_from_slice(&12u32.to_le_bytes());
        check[46..78].copy_from_slice(&[5u8; 32]);

        assert_eq!(
            parse_post_check(&check).unwrap(),
            (
                PostCheck::AccountStateEquals {
                    account_index: 3,
                    owner: [4u8; 32],
                    lamports: 99,
                    data_len: 12,
                    data_hash: [5u8; 32],
                },
                78,
            )
        );
    }

    #[test]
    fn rejects_optional_pubkey_none_with_nonzero_bytes() {
        let mut check = [0u8; 167];
        check[0] = 10;
        check[3] = 0;
        check[68] = 0;
        check[69] = 1;
        assert!(parse_post_check(&check).is_err());
    }
}
