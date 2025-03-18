use {
    solana_program::{
        instruction::Instruction,
    },
    crate::{
        H160, pda::Seed, error::Result, origin::Origin, non_evm::Bind, error::RomeProgramError::*,
    },
    super::{
        Program,
        spl_token_ix::{Transfer as Transfer, InitAccount, account_state}, len_ge,
    },
    spl_token::{
        instruction::TokenInstruction::{self, Transfer as TransferIx, InitializeAccount3},
    },
};

//  0x59adf45d      transfer(bytes32,bytes32,uint64,(bytes)[])
//  0x602c2565      account_state(bytes32)
//  0x7292100c      initialize_account3(bytes32,bytes32,bytes32)
//  0x77764881      program_id()

pub const TRANSFER_ID: &[u8] = &[0x59, 0xad, 0xf4, 0x5d];
pub const ACCOUNT_STATE_ID: &[u8] = &[0x60, 0x2c, 0x25, 0x65];
pub const INIT_ACCOUNT: &[u8] = &[0x72, 0x92, 0x10, 0x0c];
pub const PROGRAM_ID_ID: &[u8] = &[0x77, 0x76, 0x48, 0x81];

pub struct SplToken<'a, T: Origin> {
    state: &'a T,
}

impl<'a, T: Origin> SplToken<'a, T> {
    pub const ADDRESS: H160 = H160([
        0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x05,
    ]);

    pub fn new(state: &'a T) -> Self {
        Self {
            state
        }
    }
}

impl <'a, T: Origin>Program for SplToken<'a, T> {
    fn emulate(&self, ix: &Instruction, binds: Vec<Bind>) -> Result<Vec<u8>>  {
        let _ = match TokenInstruction::unpack(&ix.data)? {
            TransferIx { amount } => Transfer::emulate(ix, binds, amount)?,
            InitializeAccount3 { owner } => InitAccount::emulate(ix, binds, &owner)?,
            _ => unimplemented!(),
        };

        Ok(vec![])
    }

    fn ix_from_abi(&self, abi: &[u8], caller: H160) ->Result<(Instruction, Seed)> {
        let (func, rest) = abi.split_at(4);

        match func {
            TRANSFER_ID => {
                // TODO: fix it
                // let auth_seeds = get_vec_slices(rest, Transfer_::ABI_LEN)?;

                let (auth, seed) = self
                    .state
                    .base()
                    .pda
                    // .non_evm_balance_key(&caller, Seed::from_vec(auth_seeds));
                    .balance_key(&caller);

                len_ge!(rest, Transfer::ABI_LEN);
                let (ix_abi,_) = rest.split_at(Transfer::ABI_LEN);
                let ix = Transfer::new_from_abi(ix_abi, &auth)?;

                Ok((ix, seed))
            },
            INIT_ACCOUNT => {
                let ix = InitAccount::new_from_abi(rest)?;
                Ok((ix, Seed::default()))
            }
            _ => unimplemented!()
        }
    }

    fn eth_call(&self, args: &[u8]) -> Result<Vec<u8>> {
        let (func, rest) = args.split_at(4);

        match func {
            PROGRAM_ID_ID => Ok(spl_token::ID.to_bytes().to_vec()),
            ACCOUNT_STATE_ID => account_state(rest, self.state),
            _ => unimplemented!()
        }

    }

    fn found_eth_call(&self, input: &[u8]) -> bool {
        let (func, _) = input.split_at(4);

        match func {
            TRANSFER_ID => false,
            ACCOUNT_STATE_ID | PROGRAM_ID_ID => true,
            _ => unimplemented!()
        }
    }
}
