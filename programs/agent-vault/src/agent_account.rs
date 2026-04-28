use crate::{error::AgentVaultError, state::PUBKEY_LEN};
use pinocchio::error::ProgramError;

pub const AGENT_ACCOUNT_DISCRIMINATOR: [u8; 8] = [241, 119, 69, 140, 233, 9, 112, 50];
pub const AGENT_ACCOUNT_MIN_LEN: usize = 137;
pub const AGENT_ACCOUNT_COLLECTION_OFFSET: usize = 8;
pub const AGENT_ACCOUNT_CREATOR_OFFSET: usize = 40;
pub const AGENT_ACCOUNT_OWNER_OFFSET: usize = 72;
pub const AGENT_ACCOUNT_ASSET_OFFSET: usize = 104;
pub const AGENT_ACCOUNT_BUMP_OFFSET: usize = 136;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentAccount {
    pub collection: [u8; PUBKEY_LEN],
    pub creator: [u8; PUBKEY_LEN],
    pub owner: [u8; PUBKEY_LEN],
    pub asset: [u8; PUBKEY_LEN],
    pub bump: u8,
}

pub fn parse_agent_account(data: &[u8]) -> Result<AgentAccount, ProgramError> {
    if data.len() < AGENT_ACCOUNT_MIN_LEN {
        return Err(AgentVaultError::InvalidAgentAccount.into());
    }
    if data[0..8] != AGENT_ACCOUNT_DISCRIMINATOR {
        return Err(AgentVaultError::InvalidAgentAccount.into());
    }

    let mut collection = [0u8; PUBKEY_LEN];
    collection.copy_from_slice(
        &data[AGENT_ACCOUNT_COLLECTION_OFFSET..AGENT_ACCOUNT_COLLECTION_OFFSET + PUBKEY_LEN],
    );
    let mut creator = [0u8; PUBKEY_LEN];
    creator.copy_from_slice(
        &data[AGENT_ACCOUNT_CREATOR_OFFSET..AGENT_ACCOUNT_CREATOR_OFFSET + PUBKEY_LEN],
    );
    let mut owner = [0u8; PUBKEY_LEN];
    owner.copy_from_slice(
        &data[AGENT_ACCOUNT_OWNER_OFFSET..AGENT_ACCOUNT_OWNER_OFFSET + PUBKEY_LEN],
    );
    let mut asset = [0u8; PUBKEY_LEN];
    asset.copy_from_slice(
        &data[AGENT_ACCOUNT_ASSET_OFFSET..AGENT_ACCOUNT_ASSET_OFFSET + PUBKEY_LEN],
    );

    Ok(AgentAccount {
        collection,
        creator,
        owner,
        asset,
        bump: data[AGENT_ACCOUNT_BUMP_OFFSET],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_agent_account_offsets_from_spec() {
        let mut data = [0u8; AGENT_ACCOUNT_MIN_LEN];
        data[0..8].copy_from_slice(&AGENT_ACCOUNT_DISCRIMINATOR);
        data[AGENT_ACCOUNT_COLLECTION_OFFSET..AGENT_ACCOUNT_COLLECTION_OFFSET + PUBKEY_LEN]
            .copy_from_slice(&[3u8; 32]);
        data[AGENT_ACCOUNT_CREATOR_OFFSET..AGENT_ACCOUNT_CREATOR_OFFSET + PUBKEY_LEN]
            .copy_from_slice(&[5u8; 32]);
        data[AGENT_ACCOUNT_OWNER_OFFSET..AGENT_ACCOUNT_OWNER_OFFSET + PUBKEY_LEN]
            .copy_from_slice(&[6u8; 32]);
        data[AGENT_ACCOUNT_ASSET_OFFSET..AGENT_ACCOUNT_ASSET_OFFSET + PUBKEY_LEN]
            .copy_from_slice(&[4u8; 32]);
        data[AGENT_ACCOUNT_BUMP_OFFSET] = 250;

        let account = parse_agent_account(&data).unwrap();
        assert_eq!(account.collection, [3u8; 32]);
        assert_eq!(account.creator, [5u8; 32]);
        assert_eq!(account.owner, [6u8; 32]);
        assert_eq!(account.asset, [4u8; 32]);
        assert_eq!(account.bump, 250);
    }

    #[test]
    fn rejects_bad_discriminator() {
        let data = [0u8; AGENT_ACCOUNT_MIN_LEN];
        assert!(parse_agent_account(&data).is_err());
    }
}
