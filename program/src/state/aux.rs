use {
    crate::{pda::Seed, config::REVERT_ERROR, non_evm::aux::slice_to_abi},
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{Account as AccountTrait, AccountInfo},
        clock::Epoch,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
};

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Default)]
pub struct Account {
    pub lamports: u64,
    pub data: Vec<u8>,
    pub owner: Pubkey,
    pub executable: bool,
    pub rent_epoch: Epoch,
    pub writeable: bool,
}

impl Account {
    pub fn from_account_info(info: &AccountInfo) -> Self {
        Self {
            lamports: info.lamports(),
            data: info.data.borrow().to_vec(),
            owner: *info.owner,
            executable: info.executable,
            rent_epoch: info.rent_epoch,
            writeable: info.is_writable,
        }
    }

    pub fn new_executable() -> Self {
        let mut acc = Self::default();
        acc.executable = true;
        acc.writeable = false;
        acc
    }
}

impl AccountTrait for Account {
    fn get(&mut self) -> (&mut u64, &mut [u8], &Pubkey, bool, u64) {
        (
            &mut self.lamports,
            self.data.as_mut_slice(),
            &self.owner,
            self.executable,
            self.rent_epoch,
        )
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct AccountMeta_ {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}
impl From<AccountMeta> for AccountMeta_ {
    fn from(value: AccountMeta) -> Self {
        Self {
            pubkey: value.pubkey,
            is_signer: value.is_signer,
            is_writable: value.is_writable,
        }
    }
}
impl From<AccountMeta_> for AccountMeta {
    fn from(value: AccountMeta_) -> Self {
        Self {
            pubkey: value.pubkey,
            is_signer: value.is_signer,
            is_writable: value.is_writable,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Ix {
    program_id: Pubkey,
    accounts: Vec<AccountMeta_>,
    data: Vec<u8>,
    seed: Seed,
}

impl Ix {
    pub fn new(ix: Instruction, seed: Seed) -> Self {
        let accounts = ix
            .accounts
            .into_iter()
            .map(|a| a.into())
            .collect::<Vec<_>>();

        Self {
            program_id: ix.program_id,
            accounts,
            data: ix.data,
            seed,
        }
    }

    pub fn cast(self) -> (Instruction, Seed) {
        let accounts = self
            .accounts
            .into_iter()
            .map(|a| a.into())
            .collect::<Vec<_>>();

        let ix = Instruction {
            program_id: self.program_id,
            accounts,
            data: self.data,
        };

        (ix, self.seed)
    }
}

pub fn revert_msg(msg: String) -> Vec<u8> {
    let mut abi = slice_to_abi(msg.as_bytes());

    #[allow(unused_assignments)]
    let mut vec = Vec::with_capacity(REVERT_ERROR.len() + abi.len());
    vec = REVERT_ERROR.to_vec();
    vec.append(&mut abi);

    vec
}
