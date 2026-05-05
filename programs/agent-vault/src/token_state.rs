use crate::{
    error::AgentVaultError,
    instruction::{OptionalPubkey, TokenProgramKind},
};
use core::cmp;
use pinocchio::error::ProgramError;

pub const PUBKEY_LEN: usize = 32;
pub const MINT_LEN: usize = 82;
pub const TOKEN_ACCOUNT_LEN: usize = 165;
pub const MULTISIG_LEN: usize = 355;

pub const TOKEN_STATE_UNINITIALIZED: u8 = 0;
pub const TOKEN_STATE_INITIALIZED: u8 = 1;
pub const TOKEN_STATE_FROZEN: u8 = 2;

pub const TOKEN_2022_ACCOUNT_TYPE_OFFSET: usize = 165;
pub const TOKEN_2022_TLV_START: usize = 166;
pub const TOKEN_2022_ACCOUNT_TYPE_MINT: u8 = 1;
pub const TOKEN_2022_ACCOUNT_TYPE_ACCOUNT: u8 = 2;

pub const EXTENSION_TYPE_UNINITIALIZED: u16 = 0;
pub const EXTENSION_TYPE_TRANSFER_FEE_CONFIG: u16 = 1;
pub const EXTENSION_TYPE_TRANSFER_FEE_AMOUNT: u16 = 2;
pub const EXTENSION_TYPE_IMMUTABLE_OWNER: u16 = 7;

pub const TRANSFER_FEE_LEN: usize = 18;
pub const TRANSFER_FEE_CONFIG_LEN: usize = 108;
pub const TRANSFER_FEE_AMOUNT_LEN: usize = 8;
pub const IMMUTABLE_OWNER_LEN: usize = 0;
pub const MAX_FEE_BASIS_POINTS: u16 = 10_000;
pub const TOKEN_2022_MINT_MAX_LEN: usize =
    TOKEN_2022_TLV_START + TLV_HEADER_LEN + TRANSFER_FEE_CONFIG_LEN;
pub const TOKEN_2022_ACCOUNT_MAX_LEN: usize = TOKEN_2022_TLV_START
    + TLV_HEADER_LEN
    + TRANSFER_FEE_AMOUNT_LEN
    + TLV_HEADER_LEN
    + IMMUTABLE_OWNER_LEN;

const MINT_AUTHORITY_OFFSET: usize = 0;
const MINT_SUPPLY_OFFSET: usize = 36;
const MINT_DECIMALS_OFFSET: usize = 44;
const MINT_INITIALIZED_OFFSET: usize = 45;
const MINT_FREEZE_AUTHORITY_OFFSET: usize = 46;

const ACCOUNT_MINT_OFFSET: usize = 0;
const ACCOUNT_AUTHORITY_OFFSET: usize = 32;
const ACCOUNT_AMOUNT_OFFSET: usize = 64;
const ACCOUNT_DELEGATE_OFFSET: usize = 72;
const ACCOUNT_STATE_OFFSET: usize = 108;
const ACCOUNT_IS_NATIVE_OFFSET: usize = 109;
const ACCOUNT_DELEGATED_AMOUNT_OFFSET: usize = 121;
const ACCOUNT_CLOSE_AUTHORITY_OFFSET: usize = 129;

const TLV_HEADER_LEN: usize = 4;
const TLV_TYPE_LEN: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptionalU64 {
    None,
    Some(u64),
}

impl OptionalU64 {
    #[inline(always)]
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }

    #[inline(always)]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    #[inline(always)]
    pub fn value(&self) -> Option<u64> {
        match self {
            Self::None => None,
            Self::Some(value) => Some(*value),
        }
    }
}

impl OptionalPubkey {
    #[inline(always)]
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }

    #[inline(always)]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    #[inline(always)]
    pub fn value(&self) -> Option<[u8; PUBKEY_LEN]> {
        match self {
            Self::None => None,
            Self::Some(value) => Some(*value),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TlvEntry<'a> {
    pub extension_type: u16,
    pub payload: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransferFee {
    pub epoch: u64,
    pub maximum_fee: u64,
    pub transfer_fee_basis_points: u16,
}

impl TransferFee {
    pub fn calculate_fee(&self, pre_fee_amount: u64) -> Option<u64> {
        let basis_points = self.transfer_fee_basis_points as u128;
        if basis_points == 0 || pre_fee_amount == 0 {
            return Some(0);
        }

        let numerator = (pre_fee_amount as u128).checked_mul(basis_points)?;
        let raw_fee = numerator
            .checked_add(MAX_FEE_BASIS_POINTS as u128)?
            .checked_sub(1)?
            .checked_div(MAX_FEE_BASIS_POINTS as u128)?;

        if raw_fee > u64::MAX as u128 {
            return None;
        }

        Some(cmp::min(raw_fee as u64, self.maximum_fee))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransferFeeConfig<'a> {
    pub raw: &'a [u8],
    pub transfer_fee_config_authority: OptionalPubkey,
    pub withdraw_withheld_authority: OptionalPubkey,
    pub withheld_amount: u64,
    pub older_transfer_fee: TransferFee,
    pub newer_transfer_fee: TransferFee,
}

impl<'a> TransferFeeConfig<'a> {
    #[inline(always)]
    pub fn fee_for_epoch(&self, epoch: u64) -> TransferFee {
        if epoch >= self.newer_transfer_fee.epoch {
            self.newer_transfer_fee
        } else {
            self.older_transfer_fee
        }
    }

    #[inline(always)]
    pub fn calculate_epoch_fee(&self, epoch: u64, pre_fee_amount: u64) -> Option<u64> {
        self.fee_for_epoch(epoch).calculate_fee(pre_fee_amount)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransferFeeAmount<'a> {
    pub raw: &'a [u8],
    pub withheld_amount: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImmutableOwner<'a> {
    pub raw: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MintExtensionPolicy<'a> {
    pub transfer_fee_config: Option<TransferFeeConfig<'a>>,
    pub entry_count: u8,
}

impl<'a> MintExtensionPolicy<'a> {
    #[inline(always)]
    pub const fn none() -> Self {
        Self {
            transfer_fee_config: None,
            entry_count: 0,
        }
    }

    #[inline(always)]
    pub fn has_transfer_fee_config(&self) -> bool {
        self.transfer_fee_config.is_some()
    }

    #[inline(always)]
    pub fn canonical_entry(&self, index: usize) -> Option<TlvEntry<'a>> {
        if index == 0 {
            self.transfer_fee_config.map(|config| TlvEntry {
                extension_type: EXTENSION_TYPE_TRANSFER_FEE_CONFIG,
                payload: config.raw,
            })
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenAccountExtensionPolicy<'a> {
    pub immutable_owner: Option<ImmutableOwner<'a>>,
    pub transfer_fee_amount: Option<TransferFeeAmount<'a>>,
    pub entry_count: u8,
}

impl<'a> TokenAccountExtensionPolicy<'a> {
    #[inline(always)]
    pub const fn none() -> Self {
        Self {
            immutable_owner: None,
            transfer_fee_amount: None,
            entry_count: 0,
        }
    }

    #[inline(always)]
    pub fn has_immutable_owner(&self) -> bool {
        self.immutable_owner.is_some()
    }

    #[inline(always)]
    pub fn has_transfer_fee_amount(&self) -> bool {
        self.transfer_fee_amount.is_some()
    }

    #[inline(always)]
    pub fn transfer_fee_withheld_amount(&self) -> u64 {
        match self.transfer_fee_amount {
            Some(extension) => extension.withheld_amount,
            None => 0,
        }
    }

    #[inline(always)]
    pub fn is_closable(&self) -> bool {
        self.transfer_fee_withheld_amount() == 0
    }

    pub fn canonical_entry(&self, index: usize) -> Option<TlvEntry<'a>> {
        match index {
            0 => {
                if let Some(extension) = self.transfer_fee_amount {
                    Some(TlvEntry {
                        extension_type: EXTENSION_TYPE_TRANSFER_FEE_AMOUNT,
                        payload: extension.raw,
                    })
                } else {
                    self.immutable_owner.map(|extension| TlvEntry {
                        extension_type: EXTENSION_TYPE_IMMUTABLE_OWNER,
                        payload: extension.raw,
                    })
                }
            }
            1 => {
                if self.transfer_fee_amount.is_some() {
                    self.immutable_owner.map(|extension| TlvEntry {
                        extension_type: EXTENSION_TYPE_IMMUTABLE_OWNER,
                        payload: extension.raw,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenMint<'a> {
    pub mint_authority: OptionalPubkey,
    pub supply: u64,
    pub decimals: u8,
    pub freeze_authority: OptionalPubkey,
    pub extensions: MintExtensionPolicy<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenAccount<'a> {
    pub mint: [u8; PUBKEY_LEN],
    pub authority: [u8; PUBKEY_LEN],
    pub amount: u64,
    pub delegate: OptionalPubkey,
    pub state: u8,
    pub native_reserve: OptionalU64,
    pub delegated_amount: u64,
    pub close_authority: OptionalPubkey,
    pub extensions: TokenAccountExtensionPolicy<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenAccountCustodyProbe {
    pub mint: [u8; PUBKEY_LEN],
    pub authority: [u8; PUBKEY_LEN],
    pub delegate: OptionalPubkey,
    pub close_authority: OptionalPubkey,
}

pub fn parse_mint(
    data: &[u8],
    token_program_kind: TokenProgramKind,
) -> Result<TokenMint<'_>, ProgramError> {
    validate_mint_len(data, token_program_kind)?;

    let mint_authority = read_coption_pubkey(data, MINT_AUTHORITY_OFFSET)?;
    let supply = read_u64(data, MINT_SUPPLY_OFFSET)?;
    let decimals = *data
        .get(MINT_DECIMALS_OFFSET)
        .ok_or(AgentVaultError::InvalidTokenAccount)?;
    let is_initialized = *data
        .get(MINT_INITIALIZED_OFFSET)
        .ok_or(AgentVaultError::InvalidTokenAccount)?;
    if is_initialized != 1 {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }
    let freeze_authority = read_coption_pubkey(data, MINT_FREEZE_AUTHORITY_OFFSET)?;

    let extensions = match token_program_kind {
        TokenProgramKind::Tokenkeg => MintExtensionPolicy::none(),
        TokenProgramKind::Token2022 => {
            let tlv_data = token_2022_tlv_data(data, MINT_LEN, TOKEN_2022_ACCOUNT_TYPE_MINT)?;
            parse_mint_extensions(tlv_data)?
        }
    };

    Ok(TokenMint {
        mint_authority,
        supply,
        decimals,
        freeze_authority,
        extensions,
    })
}

#[inline(always)]
pub fn unpack_mint(
    data: &[u8],
    token_program_kind: TokenProgramKind,
) -> Result<TokenMint<'_>, ProgramError> {
    parse_mint(data, token_program_kind)
}

pub fn parse_token_account(
    data: &[u8],
    token_program_kind: TokenProgramKind,
    mint_has_transfer_fee_config: bool,
) -> Result<TokenAccount<'_>, ProgramError> {
    validate_token_account_len(data, token_program_kind)?;

    let mint = read_pubkey(data, ACCOUNT_MINT_OFFSET)?;
    let authority = read_pubkey(data, ACCOUNT_AUTHORITY_OFFSET)?;
    let amount = read_u64(data, ACCOUNT_AMOUNT_OFFSET)?;
    let delegate = read_coption_pubkey(data, ACCOUNT_DELEGATE_OFFSET)?;
    let state = *data
        .get(ACCOUNT_STATE_OFFSET)
        .ok_or(AgentVaultError::InvalidTokenAccount)?;
    if state != TOKEN_STATE_INITIALIZED {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }
    let native_reserve = read_coption_u64(data, ACCOUNT_IS_NATIVE_OFFSET)?;
    let delegated_amount = read_u64(data, ACCOUNT_DELEGATED_AMOUNT_OFFSET)?;
    let close_authority = read_coption_pubkey(data, ACCOUNT_CLOSE_AUTHORITY_OFFSET)?;

    let extensions = match token_program_kind {
        TokenProgramKind::Tokenkeg => TokenAccountExtensionPolicy::none(),
        TokenProgramKind::Token2022 => {
            let tlv_data =
                token_2022_tlv_data(data, TOKEN_ACCOUNT_LEN, TOKEN_2022_ACCOUNT_TYPE_ACCOUNT)?;
            parse_token_account_extensions(tlv_data, mint_has_transfer_fee_config)?
        }
    };

    Ok(TokenAccount {
        mint,
        authority,
        amount,
        delegate,
        state,
        native_reserve,
        delegated_amount,
        close_authority,
        extensions,
    })
}

pub fn probe_token_account_custody(
    data: &[u8],
    token_program_kind: TokenProgramKind,
) -> Result<Option<TokenAccountCustodyProbe>, ProgramError> {
    if !looks_like_token_account(data, token_program_kind) {
        return Ok(None);
    }

    let mint = read_pubkey(data, ACCOUNT_MINT_OFFSET)?;
    let authority = read_pubkey(data, ACCOUNT_AUTHORITY_OFFSET)?;
    let delegate = read_coption_pubkey(data, ACCOUNT_DELEGATE_OFFSET)?;
    let close_authority = read_coption_pubkey(data, ACCOUNT_CLOSE_AUTHORITY_OFFSET)?;

    Ok(Some(TokenAccountCustodyProbe {
        mint,
        authority,
        delegate,
        close_authority,
    }))
}

pub fn token_multisig_contains_wallet(
    data: &[u8],
    wallet_key: &[u8; PUBKEY_LEN],
) -> Result<bool, ProgramError> {
    if data.len() != MULTISIG_LEN {
        return Ok(false);
    }

    let m = data[0] as usize;
    let n = data[1] as usize;
    let is_initialized = data[2];
    if is_initialized == TOKEN_STATE_UNINITIALIZED {
        return Ok(false);
    }
    if is_initialized != TOKEN_STATE_INITIALIZED || m == 0 || n == 0 || m > n || n > 11 {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }

    let mut i = 0usize;
    while i < n {
        let offset = 3 + (i * PUBKEY_LEN);
        if slice(data, offset, PUBKEY_LEN)? == wallet_key {
            return Ok(true);
        }
        i += 1;
    }
    Ok(false)
}

#[inline(always)]
pub fn unpack_token_account(
    data: &[u8],
    token_program_kind: TokenProgramKind,
    mint_has_transfer_fee_config: bool,
) -> Result<TokenAccount<'_>, ProgramError> {
    parse_token_account(data, token_program_kind, mint_has_transfer_fee_config)
}

#[inline(always)]
pub fn parse_token_account_for_mint<'a>(
    data: &'a [u8],
    token_program_kind: TokenProgramKind,
    mint: &TokenMint<'_>,
) -> Result<TokenAccount<'a>, ProgramError> {
    parse_token_account(
        data,
        token_program_kind,
        mint.extensions.has_transfer_fee_config(),
    )
}

fn looks_like_token_account(data: &[u8], token_program_kind: TokenProgramKind) -> bool {
    match token_program_kind {
        TokenProgramKind::Tokenkeg => {
            if data.len() != TOKEN_ACCOUNT_LEN {
                return false;
            }
        }
        TokenProgramKind::Token2022 => {
            if data.len() < TOKEN_ACCOUNT_LEN || data.len() == MULTISIG_LEN {
                return false;
            }
            if data.len() > TOKEN_ACCOUNT_LEN {
                if data.len() <= TOKEN_2022_ACCOUNT_TYPE_OFFSET {
                    return false;
                }
                if data[TOKEN_2022_ACCOUNT_TYPE_OFFSET] != TOKEN_2022_ACCOUNT_TYPE_ACCOUNT {
                    return false;
                }
            }
        }
    }

    true
}

fn validate_mint_len(
    data: &[u8],
    token_program_kind: TokenProgramKind,
) -> Result<(), ProgramError> {
    match token_program_kind {
        TokenProgramKind::Tokenkeg => {
            if data.len() == MINT_LEN {
                Ok(())
            } else {
                Err(AgentVaultError::InvalidTokenAccount.into())
            }
        }
        TokenProgramKind::Token2022 => validate_token_2022_len(data, MINT_LEN),
    }
}

fn validate_token_account_len(
    data: &[u8],
    token_program_kind: TokenProgramKind,
) -> Result<(), ProgramError> {
    match token_program_kind {
        TokenProgramKind::Tokenkeg => {
            if data.len() == TOKEN_ACCOUNT_LEN {
                Ok(())
            } else {
                Err(AgentVaultError::InvalidTokenAccount.into())
            }
        }
        TokenProgramKind::Token2022 => validate_token_2022_len(data, TOKEN_ACCOUNT_LEN),
    }
}

fn validate_token_2022_len(data: &[u8], base_len: usize) -> Result<(), ProgramError> {
    if data.len() == MULTISIG_LEN || data.len() < base_len {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }

    if data.len() == base_len {
        return Ok(());
    }

    let max_len = if base_len == MINT_LEN {
        TOKEN_2022_MINT_MAX_LEN
    } else {
        TOKEN_2022_ACCOUNT_MAX_LEN
    };
    if data.len() > max_len {
        return Err(AgentVaultError::UnsupportedTokenExtension.into());
    }

    if data.len() <= TOKEN_2022_TLV_START {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }

    Ok(())
}

fn token_2022_tlv_data(
    data: &[u8],
    base_len: usize,
    expected_account_type: u8,
) -> Result<&[u8], ProgramError> {
    if data.len() == base_len {
        return Ok(&[]);
    }

    if base_len < TOKEN_2022_ACCOUNT_TYPE_OFFSET
        && !all_zero(slice(
            data,
            base_len,
            TOKEN_2022_ACCOUNT_TYPE_OFFSET - base_len,
        )?)
    {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }

    if data[TOKEN_2022_ACCOUNT_TYPE_OFFSET] != expected_account_type {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }

    Ok(&data[TOKEN_2022_TLV_START..])
}

fn parse_mint_extensions(tlv_data: &[u8]) -> Result<MintExtensionPolicy<'_>, ProgramError> {
    let mut policy = MintExtensionPolicy::none();
    let mut offset = 0usize;

    while let Some(entry) = next_tlv_entry(tlv_data, offset)? {
        match entry.extension_type {
            EXTENSION_TYPE_TRANSFER_FEE_CONFIG => {
                if policy.transfer_fee_config.is_some()
                    || entry.payload.len() != TRANSFER_FEE_CONFIG_LEN
                {
                    return Err(AgentVaultError::UnsupportedTokenExtension.into());
                }
                let transfer_fee_config = parse_transfer_fee_config(entry.payload)?;
                policy.transfer_fee_config = Some(transfer_fee_config);
                policy.entry_count = checked_entry_count(policy.entry_count)?;
            }
            _ => return Err(AgentVaultError::UnsupportedTokenExtension.into()),
        }

        offset = entry.next_offset;
    }

    Ok(policy)
}

fn parse_token_account_extensions(
    tlv_data: &[u8],
    mint_has_transfer_fee_config: bool,
) -> Result<TokenAccountExtensionPolicy<'_>, ProgramError> {
    let mut policy = TokenAccountExtensionPolicy::none();
    let mut offset = 0usize;

    while let Some(entry) = next_tlv_entry(tlv_data, offset)? {
        match entry.extension_type {
            EXTENSION_TYPE_IMMUTABLE_OWNER => {
                if policy.immutable_owner.is_some() || entry.payload.len() != IMMUTABLE_OWNER_LEN {
                    return Err(AgentVaultError::UnsupportedTokenExtension.into());
                }
                policy.immutable_owner = Some(ImmutableOwner { raw: entry.payload });
                policy.entry_count = checked_entry_count(policy.entry_count)?;
            }
            EXTENSION_TYPE_TRANSFER_FEE_AMOUNT => {
                if !mint_has_transfer_fee_config
                    || policy.transfer_fee_amount.is_some()
                    || entry.payload.len() != TRANSFER_FEE_AMOUNT_LEN
                {
                    return Err(AgentVaultError::UnsupportedTokenExtension.into());
                }
                policy.transfer_fee_amount = Some(TransferFeeAmount {
                    raw: entry.payload,
                    withheld_amount: read_u64(entry.payload, 0)?,
                });
                policy.entry_count = checked_entry_count(policy.entry_count)?;
            }
            _ => return Err(AgentVaultError::UnsupportedTokenExtension.into()),
        }

        offset = entry.next_offset;
    }

    Ok(policy)
}

struct ParsedTlvEntry<'a> {
    extension_type: u16,
    payload: &'a [u8],
    next_offset: usize,
}

fn next_tlv_entry(
    tlv_data: &[u8],
    offset: usize,
) -> Result<Option<ParsedTlvEntry<'_>>, ProgramError> {
    if offset >= tlv_data.len() {
        return Ok(None);
    }

    let remaining = tlv_data.len() - offset;
    if remaining < TLV_TYPE_LEN {
        if all_zero(&tlv_data[offset..]) {
            return Ok(None);
        }
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }

    let extension_type = read_u16(tlv_data, offset)?;
    if extension_type == EXTENSION_TYPE_UNINITIALIZED {
        if all_zero(&tlv_data[offset..]) {
            return Ok(None);
        }
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }

    if remaining < TLV_HEADER_LEN {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }

    let payload_len = read_u16(tlv_data, offset + TLV_TYPE_LEN)? as usize;
    let payload_offset = offset
        .checked_add(TLV_HEADER_LEN)
        .ok_or(AgentVaultError::InvalidTokenAccount)?;
    let next_offset = payload_offset
        .checked_add(payload_len)
        .ok_or(AgentVaultError::InvalidTokenAccount)?;
    if next_offset > tlv_data.len() {
        return Err(AgentVaultError::InvalidTokenAccount.into());
    }

    Ok(Some(ParsedTlvEntry {
        extension_type,
        payload: &tlv_data[payload_offset..next_offset],
        next_offset,
    }))
}

fn parse_transfer_fee_config(payload: &[u8]) -> Result<TransferFeeConfig<'_>, ProgramError> {
    if payload.len() != TRANSFER_FEE_CONFIG_LEN {
        return Err(AgentVaultError::UnsupportedTokenExtension.into());
    }

    let older_transfer_fee = read_transfer_fee(payload, 72)?;
    let newer_transfer_fee = read_transfer_fee(payload, 90)?;
    validate_transfer_fee(older_transfer_fee)?;
    validate_transfer_fee(newer_transfer_fee)?;

    Ok(TransferFeeConfig {
        raw: payload,
        transfer_fee_config_authority: read_optional_nonzero_pubkey(payload, 0)?,
        withdraw_withheld_authority: read_optional_nonzero_pubkey(payload, 32)?,
        withheld_amount: read_u64(payload, 64)?,
        older_transfer_fee,
        newer_transfer_fee,
    })
}

fn read_transfer_fee(data: &[u8], offset: usize) -> Result<TransferFee, ProgramError> {
    Ok(TransferFee {
        epoch: read_u64(data, offset)?,
        maximum_fee: read_u64(data, offset + 8)?,
        transfer_fee_basis_points: read_u16(data, offset + 16)?,
    })
}

fn validate_transfer_fee(transfer_fee: TransferFee) -> Result<(), ProgramError> {
    if transfer_fee.transfer_fee_basis_points > MAX_FEE_BASIS_POINTS {
        Err(AgentVaultError::InvalidTokenAccount.into())
    } else {
        Ok(())
    }
}

fn checked_entry_count(entry_count: u8) -> Result<u8, ProgramError> {
    entry_count
        .checked_add(1)
        .ok_or(AgentVaultError::UnsupportedTokenExtension.into())
}

fn read_coption_pubkey(data: &[u8], offset: usize) -> Result<OptionalPubkey, ProgramError> {
    let tag = slice(data, offset, 4)?;
    match tag {
        [0, 0, 0, 0] => Ok(OptionalPubkey::None),
        [1, 0, 0, 0] => Ok(OptionalPubkey::Some(read_pubkey(data, offset + 4)?)),
        _ => Err(AgentVaultError::InvalidTokenAccount.into()),
    }
}

fn read_coption_u64(data: &[u8], offset: usize) -> Result<OptionalU64, ProgramError> {
    let tag = slice(data, offset, 4)?;
    match tag {
        [0, 0, 0, 0] => Ok(OptionalU64::None),
        [1, 0, 0, 0] => Ok(OptionalU64::Some(read_u64(data, offset + 4)?)),
        _ => Err(AgentVaultError::InvalidTokenAccount.into()),
    }
}

fn read_optional_nonzero_pubkey(
    data: &[u8],
    offset: usize,
) -> Result<OptionalPubkey, ProgramError> {
    let key = read_pubkey(data, offset)?;
    if key == [0u8; PUBKEY_LEN] {
        Ok(OptionalPubkey::None)
    } else {
        Ok(OptionalPubkey::Some(key))
    }
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<[u8; PUBKEY_LEN], ProgramError> {
    let mut out = [0u8; PUBKEY_LEN];
    out.copy_from_slice(slice(data, offset, PUBKEY_LEN)?);
    Ok(out)
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, ProgramError> {
    let bytes = slice(data, offset, 2)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64, ProgramError> {
    let bytes = slice(data, offset, 8)?;
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

fn slice(data: &[u8], offset: usize, len: usize) -> Result<&[u8], ProgramError> {
    let end = offset
        .checked_add(len)
        .ok_or(AgentVaultError::InvalidTokenAccount)?;
    data.get(offset..end)
        .ok_or(AgentVaultError::InvalidTokenAccount.into())
}

fn all_zero(data: &[u8]) -> bool {
    data.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_u16(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64(data: &mut [u8], offset: usize, value: u64) {
        data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_coption_pubkey(data: &mut [u8], offset: usize, value: Option<[u8; PUBKEY_LEN]>) {
        match value {
            Some(key) => {
                data[offset..offset + 4].copy_from_slice(&[1, 0, 0, 0]);
                data[offset + 4..offset + 36].copy_from_slice(&key);
            }
            None => data[offset..offset + 4].copy_from_slice(&[0, 0, 0, 0]),
        }
    }

    fn base_mint() -> [u8; MINT_LEN] {
        let mut data = [0u8; MINT_LEN];
        write_coption_pubkey(&mut data, MINT_AUTHORITY_OFFSET, Some([9u8; PUBKEY_LEN]));
        write_u64(&mut data, MINT_SUPPLY_OFFSET, 1_000_000);
        data[MINT_DECIMALS_OFFSET] = 6;
        data[MINT_INITIALIZED_OFFSET] = 1;
        write_coption_pubkey(&mut data, MINT_FREEZE_AUTHORITY_OFFSET, None);
        data
    }

    fn base_account() -> [u8; TOKEN_ACCOUNT_LEN] {
        let mut data = [0u8; TOKEN_ACCOUNT_LEN];
        data[ACCOUNT_MINT_OFFSET..ACCOUNT_MINT_OFFSET + PUBKEY_LEN]
            .copy_from_slice(&[1u8; PUBKEY_LEN]);
        data[ACCOUNT_AUTHORITY_OFFSET..ACCOUNT_AUTHORITY_OFFSET + PUBKEY_LEN]
            .copy_from_slice(&[2u8; PUBKEY_LEN]);
        write_u64(&mut data, ACCOUNT_AMOUNT_OFFSET, 42);
        write_coption_pubkey(&mut data, ACCOUNT_DELEGATE_OFFSET, None);
        data[ACCOUNT_STATE_OFFSET] = TOKEN_STATE_INITIALIZED;
        write_u64(&mut data, ACCOUNT_DELEGATED_AMOUNT_OFFSET, 0);
        write_coption_pubkey(
            &mut data,
            ACCOUNT_CLOSE_AUTHORITY_OFFSET,
            Some([3u8; PUBKEY_LEN]),
        );
        data
    }

    fn token_2022_mint_with_tlv(extension_type: u16, payload: &[u8]) -> Vec<u8> {
        let mut data = vec![0u8; TOKEN_2022_TLV_START + TLV_HEADER_LEN + payload.len()];
        data[..MINT_LEN].copy_from_slice(&base_mint());
        data[TOKEN_2022_ACCOUNT_TYPE_OFFSET] = TOKEN_2022_ACCOUNT_TYPE_MINT;
        write_u16(&mut data, TOKEN_2022_TLV_START, extension_type);
        write_u16(&mut data, TOKEN_2022_TLV_START + 2, payload.len() as u16);
        data[TOKEN_2022_TLV_START + TLV_HEADER_LEN..].copy_from_slice(payload);
        data
    }

    fn token_2022_account_with_tlv(entries: &[(u16, &[u8])]) -> Vec<u8> {
        let payload_len = entries.iter().fold(0usize, |sum, (_, payload)| {
            sum + TLV_HEADER_LEN + payload.len()
        });
        let mut data = vec![0u8; TOKEN_2022_TLV_START + payload_len];
        data[..TOKEN_ACCOUNT_LEN].copy_from_slice(&base_account());
        data[TOKEN_2022_ACCOUNT_TYPE_OFFSET] = TOKEN_2022_ACCOUNT_TYPE_ACCOUNT;

        let mut offset = TOKEN_2022_TLV_START;
        for (extension_type, payload) in entries {
            write_u16(&mut data, offset, *extension_type);
            write_u16(&mut data, offset + 2, payload.len() as u16);
            offset += TLV_HEADER_LEN;
            data[offset..offset + payload.len()].copy_from_slice(payload);
            offset += payload.len();
        }

        data
    }

    fn transfer_fee_config_payload() -> [u8; TRANSFER_FEE_CONFIG_LEN] {
        let mut payload = [0u8; TRANSFER_FEE_CONFIG_LEN];
        payload[0..PUBKEY_LEN].copy_from_slice(&[4u8; PUBKEY_LEN]);
        payload[32..64].copy_from_slice(&[5u8; PUBKEY_LEN]);
        write_u64(&mut payload, 64, 7);
        write_u64(&mut payload, 72, 1);
        write_u64(&mut payload, 80, 10);
        write_u16(&mut payload, 88, 100);
        write_u64(&mut payload, 90, 10);
        write_u64(&mut payload, 98, 5);
        write_u16(&mut payload, 106, 250);
        payload
    }

    #[test]
    fn parses_canonical_tokenkeg_mint_and_account() {
        let mint_data = base_mint();
        let mint = parse_mint(&mint_data, TokenProgramKind::Tokenkeg).unwrap();
        assert_eq!(mint.supply, 1_000_000);
        assert_eq!(mint.decimals, 6);
        assert!(mint.extensions.canonical_entry(0).is_none());

        let account_data = base_account();
        let account =
            parse_token_account(&account_data, TokenProgramKind::Tokenkeg, false).unwrap();
        assert_eq!(account.mint, [1u8; PUBKEY_LEN]);
        assert_eq!(account.authority, [2u8; PUBKEY_LEN]);
        assert_eq!(account.amount, 42);
        assert_eq!(account.state, TOKEN_STATE_INITIALIZED);
        assert_eq!(account.close_authority.value(), Some([3u8; PUBKEY_LEN]));
    }

    #[test]
    fn rejects_tokenkeg_accounts_with_extension_bytes() {
        let mut account = vec![0u8; TOKEN_ACCOUNT_LEN + 1];
        account[..TOKEN_ACCOUNT_LEN].copy_from_slice(&base_account());
        assert!(parse_token_account(&account, TokenProgramKind::Tokenkeg, false).is_err());

        let mut mint = vec![0u8; MINT_LEN + 1];
        mint[..MINT_LEN].copy_from_slice(&base_mint());
        assert!(parse_mint(&mint, TokenProgramKind::Tokenkeg).is_err());
    }

    #[test]
    fn parses_token_2022_transfer_fee_config() {
        let payload = transfer_fee_config_payload();
        let data = token_2022_mint_with_tlv(EXTENSION_TYPE_TRANSFER_FEE_CONFIG, &payload);
        let mint = parse_mint(&data, TokenProgramKind::Token2022).unwrap();
        let config = mint.extensions.transfer_fee_config.unwrap();

        assert_eq!(mint.extensions.entry_count, 1);
        assert_eq!(
            config.transfer_fee_config_authority.value(),
            Some([4u8; PUBKEY_LEN])
        );
        assert_eq!(
            config.withdraw_withheld_authority.value(),
            Some([5u8; PUBKEY_LEN])
        );
        assert_eq!(config.withheld_amount, 7);
        assert_eq!(config.calculate_epoch_fee(11, 1_000), Some(5));
    }

    #[test]
    fn transfer_fee_amount_requires_transfer_fee_mint() {
        let mut payload = [0u8; TRANSFER_FEE_AMOUNT_LEN];
        write_u64(&mut payload, 0, 3);
        let data = token_2022_account_with_tlv(&[(EXTENSION_TYPE_TRANSFER_FEE_AMOUNT, &payload)]);

        assert!(parse_token_account(&data, TokenProgramKind::Token2022, false).is_err());

        let account = parse_token_account(&data, TokenProgramKind::Token2022, true).unwrap();
        assert_eq!(account.extensions.transfer_fee_withheld_amount(), 3);
        assert!(!account.extensions.is_closable());
    }

    #[test]
    fn accepts_account_extensions_and_reports_canonical_order() {
        let mut fee_payload = [0u8; TRANSFER_FEE_AMOUNT_LEN];
        write_u64(&mut fee_payload, 0, 0);
        let immutable_payload: [u8; 0] = [];
        let data = token_2022_account_with_tlv(&[
            (EXTENSION_TYPE_IMMUTABLE_OWNER, &immutable_payload),
            (EXTENSION_TYPE_TRANSFER_FEE_AMOUNT, &fee_payload),
        ]);

        let account = parse_token_account(&data, TokenProgramKind::Token2022, true).unwrap();
        assert!(account.extensions.has_immutable_owner());
        assert!(account.extensions.has_transfer_fee_amount());
        assert!(account.extensions.is_closable());
        assert_eq!(
            account
                .extensions
                .canonical_entry(0)
                .unwrap()
                .extension_type,
            EXTENSION_TYPE_TRANSFER_FEE_AMOUNT
        );
        assert_eq!(
            account
                .extensions
                .canonical_entry(1)
                .unwrap()
                .extension_type,
            EXTENSION_TYPE_IMMUTABLE_OWNER
        );
    }

    #[test]
    fn rejects_non_initialized_and_frozen_token_accounts() {
        let mut uninitialized = base_account();
        uninitialized[ACCOUNT_STATE_OFFSET] = TOKEN_STATE_UNINITIALIZED;
        assert!(parse_token_account(&uninitialized, TokenProgramKind::Tokenkeg, false).is_err());

        let mut frozen = base_account();
        frozen[ACCOUNT_STATE_OFFSET] = TOKEN_STATE_FROZEN;
        assert!(parse_token_account(&frozen, TokenProgramKind::Tokenkeg, false).is_err());
    }

    #[test]
    fn rejects_malformed_duplicate_and_unsupported_tlv() {
        let mut fee_payload = [0u8; TRANSFER_FEE_AMOUNT_LEN];
        write_u64(&mut fee_payload, 0, 0);

        let duplicate = token_2022_account_with_tlv(&[
            (EXTENSION_TYPE_TRANSFER_FEE_AMOUNT, &fee_payload),
            (EXTENSION_TYPE_TRANSFER_FEE_AMOUNT, &fee_payload),
        ]);
        assert!(parse_token_account(&duplicate, TokenProgramKind::Token2022, true).is_err());

        let unsupported = token_2022_account_with_tlv(&[(9, &[])]);
        assert!(parse_token_account(&unsupported, TokenProgramKind::Token2022, true).is_err());

        let mut trailing = token_2022_account_with_tlv(&[(EXTENSION_TYPE_IMMUTABLE_OWNER, &[])]);
        trailing.extend_from_slice(&[0, 1]);
        assert!(parse_token_account(&trailing, TokenProgramKind::Token2022, true).is_err());

        let mut malformed = token_2022_account_with_tlv(&[(EXTENSION_TYPE_IMMUTABLE_OWNER, &[])]);
        malformed.push(1);
        assert!(parse_token_account(&malformed, TokenProgramKind::Token2022, true).is_err());
    }

    #[test]
    fn accepts_zero_trailing_tlv_padding() {
        let mut data = token_2022_account_with_tlv(&[(EXTENSION_TYPE_IMMUTABLE_OWNER, &[])]);
        data.extend_from_slice(&[0, 0, 0]);

        let account = parse_token_account(&data, TokenProgramKind::Token2022, false).unwrap();
        assert!(account.extensions.has_immutable_owner());
    }
}
