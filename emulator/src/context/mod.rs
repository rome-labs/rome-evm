mod atomic;
mod context_eth_call;
mod estimate_gas;
mod iterative;
mod iterative_lock;

pub use atomic::ContextAtomic;
pub use context_eth_call::ContextEthCall;
pub use estimate_gas::ContextEstimateGas;
pub use iterative::ContextIterative;
use solana_program::pubkey::Pubkey;

pub trait LockOverrides {
    fn lock_overrides(&self) -> Vec<Pubkey>;
}
