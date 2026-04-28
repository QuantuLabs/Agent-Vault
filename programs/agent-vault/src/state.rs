use crate::{
    constants::{
        DISCRIMINATOR_GLOBAL_CONFIG, DISCRIMINATOR_VAULT_CONFIG, DISCRIMINATOR_WALLET, LABEL_LEN,
        VERSION_V0, WALLET_FLAG_ACTIVE, WALLET_FLAG_RECOVERY_ONLY,
    },
    error::AgentVaultError,
};
use pinocchio::error::ProgramError;

pub const PUBKEY_LEN: usize = 32;

pub const GLOBAL_CONFIG_LEN: usize = 160;
pub const VAULT_CONFIG_LEN: usize = 24;
pub const WALLET_LEN: usize = 32;

pub const GLOBAL_CONFIG_INITIALIZER_OFFSET: usize = 10;
pub const GLOBAL_CONFIG_REGISTRY_PROGRAM_OFFSET: usize = 42;
pub const GLOBAL_CONFIG_COLLECTION_OFFSET: usize = 74;
pub const GLOBAL_CONFIG_FEE_TREASURY_OFFSET: usize = 106;
pub const GLOBAL_CONFIG_FEE_OFFSET: usize = 138;
pub const GLOBAL_CONFIG_RESERVED_OFFSET: usize = 146;

pub const VAULT_CONFIG_WALLET_COUNT_OFFSET: usize = 10;
pub const VAULT_CONFIG_FLAGS_OFFSET: usize = 12;
pub const VAULT_CONFIG_CREATED_AT_OFFSET: usize = 14;
pub const VAULT_CONFIG_RESERVED_OFFSET: usize = 22;

pub const WALLET_INDEX_OFFSET: usize = 10;
pub const WALLET_FLAGS_OFFSET: usize = 12;
pub const WALLET_LABEL_OFFSET: usize = 14;
pub const WALLET_RESERVED_OFFSET: usize = 30;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlobalConfig {
    pub bump: u8,
    pub initializer: [u8; PUBKEY_LEN],
    pub registry_program: [u8; PUBKEY_LEN],
    pub collection: [u8; PUBKEY_LEN],
    pub fee_treasury: [u8; PUBKEY_LEN],
    pub vault_activation_fee_lamports: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VaultConfig {
    pub bump: u8,
    pub wallet_count: u16,
    pub flags: u16,
    pub created_at: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentWallet {
    pub bump: u8,
    pub index: u16,
    pub flags: u16,
    pub label: [u8; LABEL_LEN],
}

impl AgentWallet {
    #[inline(always)]
    pub fn is_active(&self) -> bool {
        self.flags & WALLET_FLAG_ACTIVE != 0
    }

    #[inline(always)]
    pub fn is_recovery_only(&self) -> bool {
        self.flags & WALLET_FLAG_RECOVERY_ONLY != 0
    }
}

#[inline(always)]
pub fn read_u16_le(input: &[u8], offset: usize) -> Result<u16, ProgramError> {
    let bytes = input
        .get(offset..offset + 2)
        .ok_or(AgentVaultError::InvalidAccountData)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

#[inline(always)]
pub fn read_u64_le(input: &[u8], offset: usize) -> Result<u64, ProgramError> {
    let bytes = input
        .get(offset..offset + 8)
        .ok_or(AgentVaultError::InvalidAccountData)?;
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

#[inline(always)]
pub fn read_i64_le(input: &[u8], offset: usize) -> Result<i64, ProgramError> {
    Ok(read_u64_le(input, offset)? as i64)
}

#[inline(always)]
pub fn read_pubkey(input: &[u8], offset: usize) -> Result<[u8; PUBKEY_LEN], ProgramError> {
    let bytes = input
        .get(offset..offset + PUBKEY_LEN)
        .ok_or(AgentVaultError::InvalidAccountData)?;
    let mut out = [0u8; PUBKEY_LEN];
    out.copy_from_slice(bytes);
    Ok(out)
}

#[inline(always)]
fn validate_header(
    data: &[u8],
    expected_len: usize,
    discriminator: &[u8; 8],
) -> Result<u8, ProgramError> {
    if data.len() != expected_len {
        return Err(AgentVaultError::InvalidAccountData.into());
    }
    if &data[0..8] != discriminator {
        return Err(AgentVaultError::InvalidDiscriminator.into());
    }
    if data[8] != VERSION_V0 {
        return Err(AgentVaultError::UnsupportedVersion.into());
    }
    Ok(data[9])
}

pub fn unpack_global_config(data: &[u8]) -> Result<GlobalConfig, ProgramError> {
    let bump = validate_header(data, GLOBAL_CONFIG_LEN, &DISCRIMINATOR_GLOBAL_CONFIG)?;
    unpack_global_config_after_header(data, bump)
}

pub fn unpack_global_config_after_header(
    data: &[u8],
    bump: u8,
) -> Result<GlobalConfig, ProgramError> {
    validate_reserved_zero(data, GLOBAL_CONFIG_RESERVED_OFFSET, GLOBAL_CONFIG_LEN)?;
    Ok(GlobalConfig {
        bump,
        initializer: read_pubkey(data, GLOBAL_CONFIG_INITIALIZER_OFFSET)?,
        registry_program: read_pubkey(data, GLOBAL_CONFIG_REGISTRY_PROGRAM_OFFSET)?,
        collection: read_pubkey(data, GLOBAL_CONFIG_COLLECTION_OFFSET)?,
        fee_treasury: read_pubkey(data, GLOBAL_CONFIG_FEE_TREASURY_OFFSET)?,
        vault_activation_fee_lamports: read_u64_le(data, GLOBAL_CONFIG_FEE_OFFSET)?,
    })
}

pub fn read_global_config_bump(data: &[u8]) -> Result<u8, ProgramError> {
    validate_header(data, GLOBAL_CONFIG_LEN, &DISCRIMINATOR_GLOBAL_CONFIG)
}

pub fn unpack_vault_config(data: &[u8]) -> Result<VaultConfig, ProgramError> {
    let bump = validate_header(data, VAULT_CONFIG_LEN, &DISCRIMINATOR_VAULT_CONFIG)?;
    unpack_vault_config_after_header(data, bump)
}

pub fn unpack_vault_config_after_header(
    data: &[u8],
    bump: u8,
) -> Result<VaultConfig, ProgramError> {
    let flags = read_u16_le(data, VAULT_CONFIG_FLAGS_OFFSET)?;
    if flags != 0 {
        return Err(AgentVaultError::InvalidAccountData.into());
    }
    validate_reserved_zero(data, VAULT_CONFIG_RESERVED_OFFSET, VAULT_CONFIG_LEN)?;
    Ok(VaultConfig {
        bump,
        wallet_count: read_u16_le(data, VAULT_CONFIG_WALLET_COUNT_OFFSET)?,
        flags,
        created_at: read_i64_le(data, VAULT_CONFIG_CREATED_AT_OFFSET)?,
    })
}

pub fn read_vault_config_bump(data: &[u8]) -> Result<u8, ProgramError> {
    validate_header(data, VAULT_CONFIG_LEN, &DISCRIMINATOR_VAULT_CONFIG)
}

pub fn unpack_wallet(data: &[u8]) -> Result<AgentWallet, ProgramError> {
    let bump = validate_header(data, WALLET_LEN, &DISCRIMINATOR_WALLET)?;
    let flags = read_u16_le(data, WALLET_FLAGS_OFFSET)?;
    let known_flags = WALLET_FLAG_ACTIVE | WALLET_FLAG_RECOVERY_ONLY;
    if flags & !known_flags != 0 {
        return Err(AgentVaultError::InvalidAccountData.into());
    }
    let is_active = flags & WALLET_FLAG_ACTIVE != 0;
    let is_recovery_only = flags & WALLET_FLAG_RECOVERY_ONLY != 0;
    if is_active == is_recovery_only {
        return Err(AgentVaultError::InvalidAccountData.into());
    }
    validate_reserved_zero(data, WALLET_RESERVED_OFFSET, WALLET_LEN)?;

    let mut label = [0u8; LABEL_LEN];
    label.copy_from_slice(
        data.get(WALLET_LABEL_OFFSET..WALLET_LABEL_OFFSET + LABEL_LEN)
            .ok_or(AgentVaultError::InvalidAccountData)?,
    );
    Ok(AgentWallet {
        bump,
        index: read_u16_le(data, WALLET_INDEX_OFFSET)?,
        flags,
        label,
    })
}

pub fn pack_global_config(config: &GlobalConfig, out: &mut [u8]) -> Result<(), ProgramError> {
    if out.len() != GLOBAL_CONFIG_LEN {
        return Err(AgentVaultError::InvalidAccountData.into());
    }
    out.fill(0);
    out[0..8].copy_from_slice(&DISCRIMINATOR_GLOBAL_CONFIG);
    out[8] = VERSION_V0;
    out[9] = config.bump;
    out[GLOBAL_CONFIG_INITIALIZER_OFFSET..GLOBAL_CONFIG_INITIALIZER_OFFSET + PUBKEY_LEN]
        .copy_from_slice(&config.initializer);
    out[GLOBAL_CONFIG_REGISTRY_PROGRAM_OFFSET..GLOBAL_CONFIG_REGISTRY_PROGRAM_OFFSET + PUBKEY_LEN]
        .copy_from_slice(&config.registry_program);
    out[GLOBAL_CONFIG_COLLECTION_OFFSET..GLOBAL_CONFIG_COLLECTION_OFFSET + PUBKEY_LEN]
        .copy_from_slice(&config.collection);
    out[GLOBAL_CONFIG_FEE_TREASURY_OFFSET..GLOBAL_CONFIG_FEE_TREASURY_OFFSET + PUBKEY_LEN]
        .copy_from_slice(&config.fee_treasury);
    out[GLOBAL_CONFIG_FEE_OFFSET..GLOBAL_CONFIG_FEE_OFFSET + 8]
        .copy_from_slice(&config.vault_activation_fee_lamports.to_le_bytes());
    Ok(())
}

pub fn pack_vault_config(config: &VaultConfig, out: &mut [u8]) -> Result<(), ProgramError> {
    if out.len() != VAULT_CONFIG_LEN {
        return Err(AgentVaultError::InvalidAccountData.into());
    }
    out.fill(0);
    out[0..8].copy_from_slice(&DISCRIMINATOR_VAULT_CONFIG);
    out[8] = VERSION_V0;
    out[9] = config.bump;
    out[VAULT_CONFIG_WALLET_COUNT_OFFSET..VAULT_CONFIG_WALLET_COUNT_OFFSET + 2]
        .copy_from_slice(&config.wallet_count.to_le_bytes());
    out[VAULT_CONFIG_FLAGS_OFFSET..VAULT_CONFIG_FLAGS_OFFSET + 2]
        .copy_from_slice(&config.flags.to_le_bytes());
    out[VAULT_CONFIG_CREATED_AT_OFFSET..VAULT_CONFIG_CREATED_AT_OFFSET + 8]
        .copy_from_slice(&config.created_at.to_le_bytes());
    Ok(())
}

pub fn pack_wallet(wallet: &AgentWallet, out: &mut [u8]) -> Result<(), ProgramError> {
    if out.len() != WALLET_LEN {
        return Err(AgentVaultError::InvalidAccountData.into());
    }
    out.fill(0);
    out[0..8].copy_from_slice(&DISCRIMINATOR_WALLET);
    out[8] = VERSION_V0;
    out[9] = wallet.bump;
    out[WALLET_INDEX_OFFSET..WALLET_INDEX_OFFSET + 2].copy_from_slice(&wallet.index.to_le_bytes());
    out[WALLET_FLAGS_OFFSET..WALLET_FLAGS_OFFSET + 2].copy_from_slice(&wallet.flags.to_le_bytes());
    out[WALLET_LABEL_OFFSET..WALLET_LABEL_OFFSET + LABEL_LEN].copy_from_slice(&wallet.label);
    Ok(())
}

#[inline(always)]
fn validate_reserved_zero(data: &[u8], start: usize, end: usize) -> Result<(), ProgramError> {
    let reserved = data
        .get(start..end)
        .ok_or(AgentVaultError::InvalidAccountData)?;
    if reserved.iter().any(|byte| *byte != 0) {
        Err(AgentVaultError::InvalidAccountData.into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{WALLET_FLAG_ACTIVE, WALLET_FLAG_RECOVERY_ONLY};

    #[test]
    fn layout_lengths_match_spec() {
        assert_eq!(GLOBAL_CONFIG_LEN, 160);
        assert_eq!(VAULT_CONFIG_LEN, 24);
        assert_eq!(WALLET_LEN, 32);
    }

    #[test]
    fn global_config_round_trips() {
        let config = GlobalConfig {
            bump: 7,
            initializer: [1u8; 32],
            registry_program: [2u8; 32],
            collection: [3u8; 32],
            fee_treasury: [4u8; 32],
            vault_activation_fee_lamports: 500_000,
        };
        let mut data = [0u8; GLOBAL_CONFIG_LEN];
        pack_global_config(&config, &mut data).unwrap();
        assert_eq!(unpack_global_config(&data).unwrap(), config);
    }

    #[test]
    fn vault_config_round_trips() {
        let config = VaultConfig {
            bump: 9,
            wallet_count: 42,
            flags: 0,
            created_at: -123,
        };
        let mut data = [0u8; VAULT_CONFIG_LEN];
        pack_vault_config(&config, &mut data).unwrap();
        assert_eq!(unpack_vault_config(&data).unwrap(), config);
    }

    #[test]
    fn wallet_round_trips_and_flags() {
        let wallet = AgentWallet {
            bump: 1,
            index: 5,
            flags: WALLET_FLAG_ACTIVE,
            label: *b"trading\0\0\0\0\0\0\0\0\0",
        };
        let mut data = [0u8; WALLET_LEN];
        pack_wallet(&wallet, &mut data).unwrap();
        let decoded = unpack_wallet(&data).unwrap();
        assert_eq!(decoded, wallet);
        assert!(decoded.is_active());
        assert!(!decoded.is_recovery_only());
    }

    #[test]
    fn wallet_flags_are_exclusive() {
        let mut data = [0u8; WALLET_LEN];
        pack_wallet(
            &AgentWallet {
                bump: 1,
                index: 0,
                flags: WALLET_FLAG_ACTIVE | WALLET_FLAG_RECOVERY_ONLY,
                label: [0u8; LABEL_LEN],
            },
            &mut data,
        )
        .unwrap();
        assert!(unpack_wallet(&data).is_err());

        pack_wallet(
            &AgentWallet {
                bump: 1,
                index: 0,
                flags: 0,
                label: [0u8; LABEL_LEN],
            },
            &mut data,
        )
        .unwrap();
        assert!(unpack_wallet(&data).is_err());

        pack_wallet(
            &AgentWallet {
                bump: 1,
                index: 0,
                flags: WALLET_FLAG_ACTIVE | 4,
                label: [0u8; LABEL_LEN],
            },
            &mut data,
        )
        .unwrap();
        assert!(unpack_wallet(&data).is_err());
    }

    #[test]
    fn reserved_bytes_must_be_zero() {
        let global = GlobalConfig {
            bump: 7,
            initializer: [1u8; 32],
            registry_program: [2u8; 32],
            collection: [3u8; 32],
            fee_treasury: [4u8; 32],
            vault_activation_fee_lamports: 500_000,
        };
        let mut global_data = [0u8; GLOBAL_CONFIG_LEN];
        pack_global_config(&global, &mut global_data).unwrap();
        global_data[GLOBAL_CONFIG_RESERVED_OFFSET] = 1;
        assert!(unpack_global_config(&global_data).is_err());

        let vault = VaultConfig {
            bump: 9,
            wallet_count: 42,
            flags: 0,
            created_at: -123,
        };
        let mut vault_data = [0u8; VAULT_CONFIG_LEN];
        pack_vault_config(&vault, &mut vault_data).unwrap();
        vault_data[VAULT_CONFIG_RESERVED_OFFSET] = 1;
        assert!(unpack_vault_config(&vault_data).is_err());

        let wallet = AgentWallet {
            bump: 1,
            index: 0,
            flags: WALLET_FLAG_ACTIVE,
            label: [0u8; LABEL_LEN],
        };
        let mut wallet_data = [0u8; WALLET_LEN];
        pack_wallet(&wallet, &mut wallet_data).unwrap();
        wallet_data[WALLET_RESERVED_OFFSET] = 1;
        assert!(unpack_wallet(&wallet_data).is_err());
    }

    #[test]
    fn vault_flags_are_reserved_in_v0() {
        let vault = VaultConfig {
            bump: 9,
            wallet_count: 42,
            flags: 1,
            created_at: -123,
        };
        let mut data = [0u8; VAULT_CONFIG_LEN];
        pack_vault_config(&vault, &mut data).unwrap();
        assert!(unpack_vault_config(&data).is_err());
    }
}
