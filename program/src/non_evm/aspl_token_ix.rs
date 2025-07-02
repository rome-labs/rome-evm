use {
    solana_program::{
        instruction::{Instruction, AccountMeta,}, pubkey::Pubkey, program_pack::Pack,
        system_program,
        rent::Rent, sysvar::Sysvar, system_instruction::create_account,
    },
    crate::{
        error::Result, origin::Origin, error::RomeProgramError::*, non_evm::Bind,
    },
    super::{Program, next, SplToken, System, spl_pda, len_eq, get_account_mut},
    std::convert::TryFrom,
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
        meta: &Vec<AccountMeta>,
        binds: &mut Vec<Bind>
    ) -> Result<()> {
        let iter = &mut meta.iter();

        let signer = next(iter)?;
        let new = next(iter)?;
        let owner = next(iter)?;
        let mint = next(iter)?;
        let _ = next(iter)?;
        let spl_program = next(iter)?;

        let (key, _) = spl_pda(&owner, &mint, &spl_program);
        if key != new {
            return Err(AccountsMismatch(new, key))
        }

        {
            let acc = get_account_mut(&new, binds)?;
            if acc.owner != system_program::ID {
                return Err(AccountAlreadyExists(new))
            }
        }

        let len = spl_token::state::Account::LEN;
        let rent = Rent::get()?.minimum_balance(len);

        let ix = create_account(&signer, &new, rent, len as u64, &spl_program);
        System::new(state).emulate(&ix, binds)?;

        let ix = initialize_account3(&spl_token::ID, &new, &mint, &owner)?;
        SplToken::new(state).emulate(&ix, binds)?;

        Ok(())
    }
}