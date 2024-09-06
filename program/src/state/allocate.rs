use {
    super::{origin::Origin, State},
    crate::{
        accounts::AccountState,
        error::{Result, RomeProgramError::*},
        Code, Data,
    },
    evm::{H160, U256},
    solana_program::msg,
};

pub trait Allocate {
    fn allocate_balance(&self, address: &H160) -> Result<()>;
    fn allocate_storage(&self, address: &H160, slot: &U256) -> Result<()>;
    fn allocate_contract(&self, address: &H160, code: &[u8], valids: &[u8]) -> Result<bool>;
}

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
        let info = self.info_addr(address, true)?;
        if AccountState::is_contract(info)? {
            return Err(DeployContractToExistingAccount(*address));
        }

        let required = Code::offset(info) + code.len() + valids.len();
        // TODO: implement deallocation
        if info.data_len() > required {
            return Err(Unimplemented("the contract deployment space must be deallocated according to size of the contract".to_string()));
        }

        let difference = required.saturating_sub(info.data_len());
        let max = self.available_for_allocation();

        let len = if difference > max { max } else { difference };

        self.realloc(info, info.data_len() + len)?;
        msg!(
            "contract address: {}, allocated {}, data.len(): {}",
            address,
            len,
            info.data_len()
        );

        Ok(difference <= max)
    }
}
