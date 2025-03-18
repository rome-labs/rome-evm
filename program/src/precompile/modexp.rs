use {
    evm::{H160, U256},
    solana_program::big_mod_exp::big_mod_exp,
    solana_program::msg,
    std::convert::TryInto,
    super::impl_contract,
};

impl_contract!(Modexp, [0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5,]);

const INPUT_LEN: usize = 96;

fn contract(input: &[u8]) -> Vec<u8> {
    msg!("modexp");
    if input.len() < INPUT_LEN {
        return vec![];
    }

    let (base_len, rest) = input.split_at(32);
    let Ok(base_len) = U256::from_big_endian(base_len).try_into() else {
        return vec![];
    };

    let (exponent_len, rest) = rest.split_at(32);
    let Ok(exponent_len) = U256::from_big_endian(exponent_len).try_into() else {
        return vec![];
    };

    let (modulus_len, rest) = rest.split_at(32);
    let Ok(modulus_len) = U256::from_big_endian(modulus_len).try_into() else {
        return vec![];
    };

    if base_len == 0 && modulus_len == 0 {
        return vec![0; 32];
    }

    let (base, rest) = rest.split_at(base_len);
    let (exponent, rest) = rest.split_at(exponent_len);
    let (modulus, _) = rest.split_at(modulus_len);

    big_mod_exp(base, exponent, modulus)
}
