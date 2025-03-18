use {
    crate::{accounts::LockType, AccountType},
    evm::{H160, H256, U256},
    rlp::DecoderError,
    solana_program::{
        program_error::ProgramError,
        pubkey::{ParsePubkeyError, Pubkey, PubkeyError},
    },
    thiserror::Error,
};

#[cfg(not(target_os = "solana"))]
use solana_client::client_error::ClientError;

pub type Result<T> = std::result::Result<T, RomeProgramError>;

pub type ErrBox = Box<dyn std::error::Error>;

#[derive(Debug, Error)]
pub enum RomeProgramError {
    #[error("The AccountInfo parser expected a mutable key where a readonly was found, or vice versa: {0}")]
    InvalidMutability(Pubkey),

    #[error("Signer not found, or more than one signer was found")]
    InvalidSigner,

    #[error("The AccountInfo parser expected a Sysvar, but the key was invalid: {0}")]
    InvalidSysvar(Pubkey),

    #[error(
        "The AccountInfo parser tried to derive the provided key, but it did not match: {0} {1}"
    )]
    InvalidDerive(Pubkey, Pubkey),

    #[error("The AccountInfo has an invalid owner: {0}")]
    InvalidOwner(Pubkey),

    #[error("The AccountInfo is non-writeable where a writeable key was expected: {0}")]
    NonWriteableAccount(Pubkey),

    #[error("An IO error was captured, wrap it up and forward it along {0}")]
    IoError(std::io::Error),

    #[error("An solana program error: {0}")]
    ProgramError(ProgramError),

    #[error("An instruction that wasn't recognised was sent")]
    UnknownInstruction(u8),

    #[error("Custom error: {0}")]
    Custom(String),

    #[error("User does not have sufficient funds: {0} {0}")]
    InsufficientFunds(H160, U256),

    #[error("Payer does not have sufficient SOL: {0}")]
    InsufficientSOLs(Pubkey),

    #[error("RLP Decored error: {0}")]
    RlpDecoderError(#[from] DecoderError),

    #[error("Invalid account state: {0} {1}")]
    InvalidAccountState(H160, String),

    #[error("Invalid account type: {0}")]
    InvalidAccountType(Pubkey),

    #[error("Invalid hash of the tx in the holder account: {0}")]
    InvalidHolderHash(Pubkey),

    #[error("Invalid data length: {0} {1}, {2}")]
    InvalidDataLength(Pubkey, usize, usize),

    #[error("PDA account not found: {0}")]
    PdaNotFound(String),

    #[error("Static Mode Violation: {0}")]
    StaticModeViolation(H160),

    #[error("Deploy a contract to an existing account: {0}")]
    DeployContractToExistingAccount(H160),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Calculation overflow")]
    CalculationOverflow,

    #[error("An solana Pubkey error: {0}")]
    PubkeyError(#[from] PubkeyError),

    #[error("account not found: {0}")]
    AccountNotFound(Pubkey),

    #[error("PDA account not found: {0} account type {1:?}")]
    PdaAccountNotFound(Pubkey, AccountType),

    #[error("attempt to init an initialized account: {0}")]
    AccountInitialized(Pubkey),

    #[error("Invalid Ethereum transaction signature: {0}")]
    InvalidEthereumSignature(String),

    #[error("Invalid instruction data")]
    InvalidInstructionData,

    #[error("Invalid non-EVM instruction data")]
    InvalidNonEvmInstructionData,

    #[cfg(not(target_os = "solana"))]
    #[error("rpc client error {0:?}")]
    RpcClientError(ClientError),

    #[cfg(not(target_os = "solana"))]
    #[error("bincode error {0:?}")]
    BincodeError(bincode::Error),

    #[error("Incorrect chain_id: {0:?} ")]
    IncorrectChainId(Option<(u64, u64)>),

    #[error("Vm fault: {0:?}")]
    VmFault(String),

    #[error("Calculation underflow")]
    CalculationUnderflow,

    #[error("Account is locked: {0} {1:?}")]
    AccountLocked(Pubkey, Option<LockType>),

    #[error("Account to write to read-only locked account: {0}")]
    AttemptWriteRoAccount(Pubkey),

    #[error("StateHolder's iteration cast error: {0}")]
    IterationCastError(String),

    #[error("Invalid transaction nonce for address: {0} {1} {2}")]
    InvalidTxNonce(H160, u64, u64),

    #[error("Allocation/deallocation error: {0}")]
    AllocationError(String),

    #[error("the feature is unimplemented: {0} ")]
    Unimplemented(String),

    #[error("Iterative transaction is finished: {0}")]
    UnnecessaryIteration(H256),

    #[error("parse Pubkey error: {0}")]
    ParsePubkeyError(#[from] ParsePubkeyError),

    #[error("Unregistered chain_id: {0} ")]
    UnregisteredChainId(u64),

    #[error("attempt to create an existing account: {0}")]
    AccountAlreadyExists(Pubkey),

    #[error("attempt to allocate an existing account: {0}")]
    AccountAlreadyInUse(Pubkey),

    #[error("Program modified the data of an account that doesn't belong to it: {0}")]
    ExternalAccountDataModified(Pubkey),

    #[error("attempt to transfer SOL from account with non-empty data: {0}")]
    TransferFromAccountWithData(Pubkey),

    #[error("Provided and calculated accounts mismatch: {0} {1}")]
    AccountsMismatch(Pubkey, Pubkey),

    #[error("Attempt to modity read-only account: {0}")]
    ModifyReadOnlyAccount(Pubkey),

    #[error("Invalid non-evm authority account: {0}")]
    InvalidAuthority(Pubkey),

    #[error("Inconsistent account list")]
    InconsistentAccountList,
}

impl From<ProgramError> for RomeProgramError {
    fn from(e: ProgramError) -> Self {
        RomeProgramError::ProgramError(e)
    }
}

impl From<std::io::Error> for RomeProgramError {
    fn from(e: std::io::Error) -> Self {
        RomeProgramError::IoError(e)
    }
}

impl From<RomeProgramError> for ProgramError {
    fn from(err: RomeProgramError) -> ProgramError {
        match err {
            RomeProgramError::ProgramError(e) => e,
            _ => ProgramError::Custom(0),
        }
    }
}
#[cfg(not(target_os = "solana"))]
impl From<ClientError> for RomeProgramError {
    fn from(e: ClientError) -> RomeProgramError {
        RomeProgramError::RpcClientError(e)
    }
}
#[cfg(not(target_os = "solana"))]
impl From<bincode::Error> for RomeProgramError {
    fn from(e: bincode::Error) -> RomeProgramError {
        RomeProgramError::BincodeError(e)
    }
}
