mod atomic;
mod context_eth_call;
mod iterative;
mod iterative_lock;

pub use atomic::ContextAtomic;
pub use context_eth_call::ContextEthCall;
pub use iterative::ContextIterative;

use {
    crate::state::State,
    rome_evm::{
        accounts::{Data, SignerInfo},
        error::*,
        H160,
    },
    solana_program::account_info::IntoAccountInfo,
};

fn gas_recipient(state: &State) -> Result<Option<H160>> {
    let signer = state.signer.unwrap();
    if let Ok(mut bind) = state.info_signer_info(&signer, false) {
        let info = bind.into_account_info();
        let signer_info = SignerInfo::from_account(&info)?;
        Ok(Some(signer_info.address))
    } else {
        Ok(None)
    }
}
