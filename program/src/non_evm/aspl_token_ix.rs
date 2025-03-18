use {
    solana_program::{
        instruction::Instruction, pubkey::Pubkey, program_pack::Pack,
        system_program,
        rent::Rent, sysvar::Sysvar, system_instruction::create_account,
    },
    crate::{
        error::Result, origin::Origin, error::RomeProgramError::*, non_evm::Bind,
    },
    super::{Program, next, SplToken, System, ASplToken, len_eq},
    std::{
        convert::TryFrom,
    },
    spl_associated_token_account::instruction::{
        create_associated_token_account,
    },
    spl_token::instruction::initialize_account3,
};

pub struct Create();

impl Create {
    pub fn new_from_abi(signer: &Pubkey, abi: &[u8]) -> Result<Instruction> {
        len_eq!(abi, 32*2);

        let (wallet, mint) = abi.split_at(32);
        let wallet = Pubkey::try_from(wallet).unwrap();
        let mint = Pubkey::try_from(mint).unwrap();

        let ix = create_associated_token_account(
            signer, &wallet, &mint, &spl_token::ID);

        Ok(ix)
    }

    pub fn emulate<T: Origin>(
        state: &T,

        binds: Vec<Bind>
    ) -> Result<Vec<u8>> {
        let iter = &mut binds.into_iter();
        let signer = next(iter)?;
        let new = next(iter)?;
        let owner = next(iter)?.0;
        let mint = next(iter)?;
        let _ = next(iter)?;
        let spl_program = next(iter)?.0;

        let (key, _) = ASplToken::<T>::pda(owner, mint.0, spl_program);

        if key != *new.0 {
            return Err(AccountsMismatch(*new.0, key))
        }

        if new.1.owner != system_program::ID {
            return Err(AccountAlreadyExists(*new.0))
        }

        let len = spl_token::state::Account::LEN;
        let rent = Rent::get()?.minimum_balance(len);

        let ix = create_account(signer.0, new.0, rent, len as u64, spl_program);
        System::new(state).emulate(&ix, vec![signer, (new.0, new.1)])?;

        let ix = initialize_account3(&spl_token::ID, new.0, mint.0, owner)?;
        SplToken::new(state).emulate(&ix, vec![(new.0, new.1), mint])?;

        Ok(new.0.to_bytes().to_vec())
    }
}