use pinocchio::Address;

pub const VERSION_V0: u8 = 0;

pub const MAX_WALLETS: u16 = u16::MAX;
pub const LABEL_LEN: usize = 16;
pub const MAX_CPI_ACCOUNTS: u8 = 64;
pub const MAX_CPI_IX_DATA_LEN: usize = 1024;
pub const MAX_POST_CHECKS: u8 = 8;

pub const SEED_GLOBAL_CONFIG: &[u8] = b"global_config";
pub const SEED_VAULT_CONFIG: &[u8] = b"vault_config";
pub const SEED_AGENT_WALLET: &[u8] = b"agent_vault";

pub const DISCRIMINATOR_GLOBAL_CONFIG: [u8; 8] = *b"AVGLBCFG";
pub const DISCRIMINATOR_VAULT_CONFIG: [u8; 8] = *b"AVAGTCFG";
pub const DISCRIMINATOR_WALLET: [u8; 8] = *b"AVWALLT0";

pub const WALLET_FLAG_ACTIVE: u16 = 1 << 0;
pub const WALLET_FLAG_RECOVERY_ONLY: u16 = 1 << 1;

pub const SYSTEM_PROGRAM_ID: Address = Address::new_from_array([0u8; 32]);
pub const RENT_SYSVAR_ID: Address = Address::new_from_array([
    6, 167, 213, 23, 25, 44, 92, 81, 33, 140, 201, 76, 61, 74, 241, 127, 88, 218, 238, 8, 155, 161,
    253, 68, 227, 219, 217, 138, 0, 0, 0, 0,
]);
pub const CLOCK_SYSVAR_ID: Address = Address::new_from_array([
    6, 167, 213, 23, 24, 199, 116, 201, 40, 86, 99, 152, 105, 29, 94, 182, 139, 94, 184, 163, 155,
    75, 109, 92, 115, 85, 91, 33, 0, 0, 0, 0,
]);
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Address = Address::new_from_array([
    140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142, 13, 131, 11, 90, 19, 153, 218,
    255, 16, 132, 4, 142, 123, 216, 219, 233, 248, 89,
]);
pub const TOKEN_PROGRAM_ID: Address = Address::new_from_array([
    6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 133, 237,
    95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
]);
pub const TOKEN_2022_PROGRAM_ID: Address = Address::new_from_array([
    6, 221, 246, 225, 238, 117, 143, 222, 24, 66, 93, 188, 228, 108, 205, 218, 182, 26, 252, 77,
    131, 185, 13, 39, 254, 189, 249, 40, 216, 161, 139, 252,
]);
pub const NATIVE_MINT_ID: Address = Address::new_from_array([
    6, 155, 136, 87, 254, 171, 129, 132, 251, 104, 127, 99, 70, 24, 192, 53, 218, 196, 57, 220, 26,
    235, 59, 85, 152, 160, 240, 0, 0, 0, 0, 1,
]);
pub const METAPLEX_CORE_PROGRAM_ID: Address = Address::new_from_array([
    175, 84, 171, 16, 189, 151, 165, 66, 160, 158, 247, 179, 152, 137, 221, 12, 211, 148, 164, 204,
    233, 223, 166, 205, 201, 126, 190, 45, 35, 91, 167, 72,
]);

pub const BPF_LOADER_ID: Address = Address::new_from_array([
    2, 168, 246, 145, 78, 136, 161, 107, 189, 35, 149, 133, 95, 100, 4, 217, 180, 244, 86, 183,
    130, 27, 176, 20, 87, 73, 66, 140, 0, 0, 0, 0,
]);
pub const BPF_LOADER_DEPRECATED_ID: Address = Address::new_from_array([
    2, 168, 246, 145, 78, 136, 161, 110, 57, 90, 225, 40, 148, 143, 250, 105, 86, 147, 55, 104, 24,
    221, 71, 67, 82, 33, 243, 198, 0, 0, 0, 0,
]);
pub const BPF_UPGRADEABLE_LOADER_ID: Address = Address::new_from_array([
    2, 168, 246, 145, 78, 136, 161, 176, 226, 16, 21, 62, 247, 99, 174, 43, 0, 194, 185, 61, 22,
    193, 36, 210, 192, 83, 122, 16, 4, 128, 0, 0,
]);
pub const LOADER_V4_ID: Address = Address::new_from_array([
    5, 18, 180, 17, 81, 81, 227, 122, 173, 10, 139, 197, 211, 136, 46, 123, 127, 218, 76, 243, 210,
    192, 40, 200, 207, 131, 54, 24, 0, 0, 0, 0,
]);
pub const NATIVE_LOADER_ID: Address = Address::new_from_array([
    5, 135, 132, 191, 20, 139, 164, 40, 47, 176, 18, 87, 72, 136, 169, 241, 83, 160, 125, 173, 247,
    101, 192, 69, 92, 154, 151, 3, 128, 0, 0, 0,
]);

pub const AGENT_REGISTRY_DEVNET_ID: Address = Address::new_from_array([
    115, 254, 154, 49, 129, 129, 48, 89, 58, 8, 167, 194, 122, 8, 194, 220, 245, 213, 204, 195,
    104, 134, 39, 243, 10, 135, 13, 68, 20, 212, 2, 39,
]);
pub const AGENT_REGISTRY_MAINNET_ID: Address = Address::new_from_array([
    115, 254, 155, 212, 220, 142, 184, 10, 96, 166, 39, 107, 205, 233, 133, 186, 134, 94, 137, 81,
    196, 110, 150, 62, 1, 188, 184, 198, 20, 135, 24, 131,
]);

#[cfg(feature = "mainnet")]
compile_error!("mainnet release constants must be finalized before building with `mainnet`");

#[cfg(not(feature = "mainnet"))]
pub const EXPECTED_INITIALIZER: Address = Address::new_from_array([
    19, 170, 56, 249, 183, 167, 202, 247, 46, 162, 199, 116, 239, 168, 220, 156, 232, 61, 169, 237,
    64, 134, 251, 103, 51, 129, 151, 239, 5, 181, 190, 57,
]);
#[cfg(not(feature = "mainnet"))]
pub const EXPECTED_COLLECTION: Address = Address::new_from_array([
    77, 58, 81, 151, 27, 212, 47, 66, 205, 172, 141, 99, 34, 10, 199, 181, 61, 194, 128, 143, 163,
    137, 213, 17, 64, 27, 150, 250, 16, 10, 189, 97,
]);
#[cfg(not(feature = "mainnet"))]
pub const EXPECTED_FEE_TREASURY: Address = Address::new_from_array([
    201, 240, 42, 20, 173, 37, 211, 9, 176, 102, 201, 19, 29, 146, 113, 127, 136, 66, 74, 209, 236,
    26, 124, 162, 37, 135, 91, 102, 154, 24, 32, 22,
]);

#[cfg(feature = "mainnet")]
pub const EXPECTED_REGISTRY_PROGRAM: Address = AGENT_REGISTRY_MAINNET_ID;
#[cfg(not(feature = "mainnet"))]
pub const EXPECTED_REGISTRY_PROGRAM: Address = AGENT_REGISTRY_DEVNET_ID;

#[cfg(feature = "mainnet")]
pub const EXPECTED_ACTIVATION_FEE_LAMPORTS: u64 = 500_000;
#[cfg(not(feature = "mainnet"))]
pub const EXPECTED_ACTIVATION_FEE_LAMPORTS: u64 = 500_000;

#[inline(always)]
pub fn is_loader_program(address: &Address) -> bool {
    address == &BPF_LOADER_ID
        || address == &BPF_LOADER_DEPRECATED_ID
        || address == &BPF_UPGRADEABLE_LOADER_ID
        || address == &LOADER_V4_ID
        || address == &NATIVE_LOADER_ID
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_denylist_includes_native_and_v4() {
        assert!(is_loader_program(&BPF_LOADER_ID));
        assert!(is_loader_program(&BPF_LOADER_DEPRECATED_ID));
        assert!(is_loader_program(&BPF_UPGRADEABLE_LOADER_ID));
        assert!(is_loader_program(&NATIVE_LOADER_ID));
        assert!(is_loader_program(&LOADER_V4_ID));
    }

    #[test]
    fn devnet_release_constants_are_non_zero() {
        assert_ne!(EXPECTED_INITIALIZER, Address::new_from_array([0u8; 32]));
        assert_ne!(EXPECTED_COLLECTION, Address::new_from_array([0u8; 32]));
        assert_ne!(EXPECTED_FEE_TREASURY, Address::new_from_array([0u8; 32]));
        assert_eq!(EXPECTED_ACTIVATION_FEE_LAMPORTS, 500_000);
    }
}
