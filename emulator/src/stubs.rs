use {
    rome_evm::error::Result,
    solana_client::rpc_client::RpcClient,
    solana_program::{clock::Clock, rent::Rent, sysvar, program_stubs::SyscallStubs},
    // solana_sdk::program_stubs::SyscallStubs,
    std::sync::Arc,
};

#[derive(Default)]
pub struct Stubs {
    rent: Rent,
    clock: Clock,
}

impl Stubs {
    pub fn from_chain(rpc: Arc<RpcClient>) -> Result<Box<Self>> {
        let keys = [sysvar::clock::ID, sysvar::rent::ID];
        let accounts = rpc.get_multiple_accounts(&keys)?;

        let mut stubs = Stubs::default();
        for (&key, acc) in keys.iter().zip(accounts.iter()) {
            let Some(acc) = acc else {
                continue;
            };

            match key {
                sysvar::clock::ID => {
                    stubs.clock = bincode::deserialize(&acc.data)?;
                }
                sysvar::rent::ID => {
                    stubs.rent = bincode::deserialize(&acc.data)?;
                }
                _ => unreachable!()
            }
        }

        Ok(Box::new(stubs))
    }
}

impl SyscallStubs for Stubs {
    fn sol_get_rent_sysvar(&self, pointer: *mut u8) -> u64 {
        unsafe {
            #[allow(clippy::cast_ptr_alignment)]
            let rent = pointer.cast::<Rent>();
            *rent = self.rent.clone();
        }
        0
    }
    fn sol_get_clock_sysvar(&self, pointer: *mut u8) -> u64 {
        unsafe {
            #[allow(clippy::cast_ptr_alignment)]
            let clock = pointer.cast::<Clock>();
            *clock = self.clock.clone();
        }
        0
    }
}
