use crate::{constants::METAPLEX_CORE_PROGRAM_ID, error::AgentVaultError, state::PUBKEY_LEN};
use pinocchio::{error::ProgramError, AccountView, Address};

pub const CORE_ASSET_MIN_LEN: usize = 66;
pub const CORE_ASSET_KEY_OFFSET: usize = 0;
pub const CORE_ASSET_OWNER_OFFSET: usize = 1;
pub const CORE_ASSET_COLLECTION_TAG_OFFSET: usize = 33;
pub const CORE_ASSET_COLLECTION_OFFSET: usize = 34;

pub const CORE_ASSET_V1_KEY: u8 = 1;
pub const CORE_ASSET_COLLECTION_TAG: u8 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoreAsset {
    pub owner: [u8; PUBKEY_LEN],
    pub collection: [u8; PUBKEY_LEN],
}

pub fn parse_core_asset(data: &[u8]) -> Result<CoreAsset, ProgramError> {
    if data.len() < CORE_ASSET_MIN_LEN {
        return Err(AgentVaultError::InvalidCoreAsset.into());
    }
    if data[CORE_ASSET_KEY_OFFSET] != CORE_ASSET_V1_KEY {
        return Err(AgentVaultError::InvalidCoreAsset.into());
    }
    if data[CORE_ASSET_COLLECTION_TAG_OFFSET] != CORE_ASSET_COLLECTION_TAG {
        return Err(AgentVaultError::InvalidCollection.into());
    }

    let mut owner = [0u8; PUBKEY_LEN];
    owner.copy_from_slice(&data[CORE_ASSET_OWNER_OFFSET..CORE_ASSET_OWNER_OFFSET + PUBKEY_LEN]);
    let mut collection = [0u8; PUBKEY_LEN];
    collection.copy_from_slice(
        &data[CORE_ASSET_COLLECTION_OFFSET..CORE_ASSET_COLLECTION_OFFSET + PUBKEY_LEN],
    );
    Ok(CoreAsset { owner, collection })
}

pub fn read_core_asset(account: &AccountView) -> Result<CoreAsset, ProgramError> {
    if !account.owned_by(&METAPLEX_CORE_PROGRAM_ID) {
        return Err(AgentVaultError::InvalidCoreAsset.into());
    }
    let data = account.try_borrow()?;
    parse_core_asset(&data)
}

pub fn assert_core_asset_owner_and_collection(
    holder: &AccountView,
    agent_asset: &AccountView,
    expected_collection: &[u8; PUBKEY_LEN],
) -> Result<(), ProgramError> {
    let asset = read_core_asset(agent_asset)?;
    if asset.owner != address_bytes(holder.address()) {
        return Err(AgentVaultError::InvalidHolder.into());
    }
    if &asset.collection != expected_collection {
        return Err(AgentVaultError::InvalidCollection.into());
    }
    Ok(())
}

#[inline(always)]
fn address_bytes(address: &Address) -> [u8; PUBKEY_LEN] {
    let mut out = [0u8; PUBKEY_LEN];
    out.copy_from_slice(address.as_ref());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_core_asset_offsets_from_spec() {
        let mut data = [0u8; CORE_ASSET_MIN_LEN];
        data[0] = CORE_ASSET_V1_KEY;
        data[CORE_ASSET_OWNER_OFFSET..CORE_ASSET_OWNER_OFFSET + PUBKEY_LEN]
            .copy_from_slice(&[7u8; 32]);
        data[CORE_ASSET_COLLECTION_TAG_OFFSET] = CORE_ASSET_COLLECTION_TAG;
        data[CORE_ASSET_COLLECTION_OFFSET..CORE_ASSET_COLLECTION_OFFSET + PUBKEY_LEN]
            .copy_from_slice(&[9u8; 32]);

        let asset = parse_core_asset(&data).unwrap();
        assert_eq!(asset.owner, [7u8; 32]);
        assert_eq!(asset.collection, [9u8; 32]);
    }

    #[test]
    fn rejects_missing_collection_tag() {
        let mut data = [0u8; CORE_ASSET_MIN_LEN];
        data[0] = CORE_ASSET_V1_KEY;
        assert!(parse_core_asset(&data).is_err());
    }
}
