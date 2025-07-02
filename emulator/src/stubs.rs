use {
    rome_evm::error::Result,
    solana_client::rpc_client::RpcClient,
    solana_program::{
        entrypoint::SUCCESS,
        pubkey::Pubkey,
        hash::Hash,
        clock::Clock, rent::Rent, sysvar, program_stubs::SyscallStubs,
        slot_hashes::SlotHashes,
    },
    std::sync::Arc,
};

#[derive(Default)]
pub struct Stubs {
    rent: Rent,
    clock: Clock,
    slot_hashes: SlotHashes,
}

impl Stubs {
    pub fn from_chain(rpc: Arc<RpcClient>) -> Result<Box<Self>> {
        // TODO:  optimize: load slot_hashes only for alt instruction
        let keys = [sysvar::clock::ID, sysvar::rent::ID, sysvar::slot_hashes::ID,];
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
                sysvar::slot_hashes::ID => {
                    stubs.slot_hashes = bincode::deserialize(&acc.data)?;
                }
                _ => unreachable!(),
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
    fn sol_get_sysvar(
        &self, 
        sysvar_id_addr: *const u8, 
        var_addr: *mut u8,
        _offset: u64,
        _length: u64,
    ) -> u64 {
        let key = sysvar_id_addr  as *const Pubkey;

        unsafe {
            if *key != sysvar::slot_hashes::ID {
                solana_program::msg!("sol_get_sysvar stub is not implemented for the account: {}", *key);
                unimplemented!()
            }
        }
        
        unsafe {
            #[allow(clippy::cast_ptr_alignment)]
            let ptr = var_addr.cast::<Vec<(u64, Hash)>>();
            *ptr = self.slot_hashes.clone();
        }
        
        SUCCESS
    }
}
