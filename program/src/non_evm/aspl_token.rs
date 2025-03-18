use {
    solana_program::{
        instruction::Instruction, pubkey::Pubkey,
    },
    crate::{
        H160, pda::Seed, error::Result, origin::Origin, non_evm::Bind,
    },
    borsh::{BorshDeserialize,},
    super::{
        Program,
        aspl_token_ix::{Create, },
    },
    spl_associated_token_account::instruction::{
        AssociatedTokenAccountInstruction as ATAI,
    },
};

//  0x89a569f4      create_associated_token_account(bytes32,bytes32)
//  0x77764881       program_id()

pub const PROGRAM_ID_ID: &[u8] = &[0x77, 0x76, 0x48, 0x81];
pub const CREATE_ID: &[u8] = &[0x89, 0xa5, 0x69, 0xf4];

pub struct ASplToken<'a, T: Origin> {
    state: &'a T,
}

impl<'a, T: Origin> ASplToken<'a, T> {
    pub const ADDRESS: H160 = H160([
        0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x06,
    ]);
    pub fn  new(state: &'a T) -> Self {
        Self {
            state
        }
    }
    pub fn pda(owner: &Pubkey, mint: &Pubkey, token_program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                &owner.to_bytes(),
                &token_program_id.to_bytes(),
                &mint.to_bytes(),
            ],
            &spl_associated_token_account::ID,
        )
    }
}

impl<'a, T: Origin> Program for ASplToken<'a, T> {
    fn emulate(&self, ix: &Instruction, binds: Vec<Bind>) -> Result<Vec<u8>>  {

        match ATAI::try_from_slice(&ix.data)? {
            ATAI::Create => Create::emulate(self.state, binds),
            _ => unimplemented!(),
        }
    }

    fn ix_from_abi(&self, input: &[u8], _: H160) -> Result<(Instruction, Seed)> {
        let (func, rest) = input.split_at(4);

        match func {
            CREATE_ID => {
                let ix = Create::new_from_abi(&self.state.signer(), rest)?;

                solana_program::msg!("SEED  {:?}", Seed::default().items);
                Ok((ix, Seed::default()))
            },
            _ => unimplemented!()
        }
    }

    fn eth_call(&self, input: &[u8]) -> Result<Vec<u8>> {
        let (func, _) = input.split_at(4);

        match func {
            PROGRAM_ID_ID => Ok(spl_associated_token_account::ID.to_bytes().to_vec()),
            _ => unimplemented!()
        }

    }

    fn found_eth_call(&self, input: &[u8]) -> bool {
        let (func, _) = input.split_at(4);

        match func {
            CREATE_ID => false,
            PROGRAM_ID_ID => true,
            _ => unimplemented!()
        }
    }
}
