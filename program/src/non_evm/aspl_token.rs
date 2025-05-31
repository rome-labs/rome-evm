use {
    solana_program::{
        instruction::Instruction, pubkey::Pubkey,
    },
    crate::{
        H160, pda::Seed, error::Result, origin::Origin, non_evm::Bind, error::RomeProgramError::*,
    },
    borsh::{BorshDeserialize,},
    super::{
        Program,
        aspl_token_ix::{Create, }, EvmDiff, len_eq,
    },
    spl_associated_token_account::instruction::{
        AssociatedTokenAccountInstruction as ATAI,
    },
    evm::Context,
};
#[cfg(feature = "single-state")]
use {
    crate::non_evm::NonEvmState,
};

//  0x89a569f4      create_associated_token_account(bytes32,bytes32)
//  0x5b4fdca0      create_associated_token_account(address,bytes32)
//  0x77764881       program_id()

#[cfg(feature = "single-state")]
pub const PROGRAM_ID_ID: &[u8] = &[0x77, 0x76, 0x48, 0x81];
pub const CREATE_ID: &[u8] = &[0x89, 0xa5, 0x69, 0xf4]; // pda, mint_id
pub const CREATE_BY_ADDRESS_ID: &[u8] = &[0x5b, 0x4f, 0xdc, 0xa0]; // address, mint_id

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
}

impl<'a, T: Origin> Program for ASplToken<'a, T> {
    fn emulate(&self, ix: &Instruction, binds: Vec<Bind>) -> Result<()>  {
        match ATAI::try_from_slice(&ix.data)? {
            ATAI::Create => Create::emulate(self.state, binds),
            _ => unimplemented!(),
        }
    }

    fn ix_from_abi(&self, input: &[u8], _: &Context) -> Result<(Instruction, Seed, Vec<EvmDiff>)> {
        let (func, rest) = input.split_at(4);

        match func {
            #[cfg(feature = "single-state")]
            CREATE_ID => {
                let ix = Create::new_from_abi(&self.state.signer(), rest)?;
                Ok((ix, Seed::default(), vec![]))
            },
            CREATE_BY_ADDRESS_ID => {
                len_eq!(rest, 32 + 32);
                let (left, right) = rest.split_at(32);
                let caller = H160::from_slice(&left[12..]);
                let (key, _) = self.state.base().pda.balance_key(&caller);
                let input_ = [key.to_bytes().as_slice(), right].concat();
                let ix = Create::new_from_abi(&self.state.signer(), &input_)?;
                
                Ok((ix, Seed::default(), vec![]))
            },
            _ => unimplemented!()
        }
    }

    #[cfg(feature = "single-state")]
    fn eth_call(&self, input: &[u8], _: &NonEvmState) -> Result<Vec<u8>> {
        let (func, _) = input.split_at(4);

        match func {
            PROGRAM_ID_ID => Ok(spl_associated_token_account::ID.to_bytes().to_vec()),
            _ => unimplemented!()
        }

    }

    #[cfg(feature = "single-state")]
    fn found_eth_call(&self, input: &[u8]) -> bool {
        let (func, _) = input.split_at(4);

        match func {
            CREATE_ID => false,
            PROGRAM_ID_ID => true,
            _ => unimplemented!()
        }
    }
}

pub fn spl_pda(owner: &Pubkey, mint: &Pubkey, token_program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            &owner.to_bytes(),
            &token_program_id.to_bytes(),
            &mint.to_bytes(),
        ],
        &spl_associated_token_account::ID,
    )
}
