use pinocchio::error::ProgramError;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentVaultError {
    InvalidInstruction = 0,
    InvalidInstructionData = 1,
    InvalidAccountData = 2,
    InvalidPda = 3,
    InvalidDiscriminator = 4,
    UnsupportedVersion = 5,
    InvalidBump = 6,
    InvalidOwner = 7,
    InvalidSigner = 8,
    InvalidWritable = 9,
    InvalidProgramId = 10,
    InvalidSysvar = 11,
    InvalidCoreAsset = 12,
    InvalidAgentAccount = 13,
    InvalidHolder = 14,
    InvalidCollection = 15,
    InvalidGlobalConfig = 16,
    InvalidVaultConfig = 17,
    InvalidWallet = 18,
    WalletInactive = 19,
    WalletRecoveryOnly = 20,
    WalletCountOverflow = 21,
    InvalidLabel = 22,
    InvalidFee = 23,
    InvalidTreasury = 24,
    ArithmeticOverflow = 25,
    ArithmeticUnderflow = 26,
    InsufficientLamports = 27,
    RentFloorViolation = 28,
    InvalidTokenProgram = 29,
    InvalidTokenAccount = 30,
    UnsupportedTokenExtension = 31,
    InvalidAta = 32,
    InvalidWsolAccount = 33,
    InvalidCpiTarget = 34,
    InvalidCpiAccounts = 35,
    InvalidPostCheck = 36,
    MissingEconomicPostCheck = 37,
    MissingCustodyPostCheck = 38,
    CustodyChanged = 39,
    PostCheckFailed = 40,
    DuplicateAccount = 41,
    AccountLimitExceeded = 42,
    DataLimitExceeded = 43,
    UnsupportedInstruction = 44,
}

impl From<AgentVaultError> for ProgramError {
    #[inline(always)]
    fn from(error: AgentVaultError) -> Self {
        ProgramError::Custom(error as u32)
    }
}
