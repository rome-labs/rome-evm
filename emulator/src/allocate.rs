use {
    crate::state::State,
    rome_evm::{
        error::{Result, RomeProgramError::*},
        state::{allocate::Allocate, origin::Origin},
        AccountState, Code, Data, H160, U256,
    },
    solana_program::account_info::IntoAccountInfo,
};

impl Allocate for State<'_> {
    fn allocate_balance(&self, address: &H160) -> Result<()> {
        let _ = self.info_addr(address, true)?;
        Ok(())
    }
    fn allocate_storage(&self, address: &H160, slot: &U256) -> Result<()> {
        let _ = self.info_slot(address, slot, true)?;
        //TODO: implement slot allocation
        Ok(())
    }
    fn allocate_contract(&self, address: &H160, code: &[u8], valids: &[u8]) -> Result<bool> {
        let mut bind = self.info_addr(address, true)?;

        let difference = {
            let info = bind.into_account_info();

            if AccountState::is_contract(&info)? {
                return Err(DeployContractToExistingAccount(*address));
            }

            let required = Code::offset(&info) + code.len() + valids.len();
            // TODO: implement deallocation
            if info.data_len() > required {
                return Err(Unimplemented("the contract deployment space must be deallocated according to size of the contract".to_string()));
            }

            required.saturating_sub(info.data_len())
        };

        let max = self.available_for_allocation();
        let resize = if difference > max { max } else { difference };

        let len = bind.1.data.len() + resize;
        self.realloc(&mut bind, len)?;
        self.update(bind)?;

        Ok(difference <= max)
    }
}
