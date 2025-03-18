use {
    evm::H160, solana_program::msg, std::convert::TryInto, super::impl_contract,
};

impl_contract!(Blake2f, [0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9,]);

const SIGMA: [[usize; 16]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

const IV: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

const BLAKE2_INPUT_LEN: usize = 213;

fn g(v: &mut [u64], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(24);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(63);
}

fn round(i: u32, v: &mut [u64; 16], m: &[u64]) {
    let s = &SIGMA[(i % 10) as usize];
    g(v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
    g(v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
    g(v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
    g(v, 3, 7, 11, 15, m[s[6]], m[s[7]]);
    g(v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
    g(v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
    g(v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
    g(v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
}

fn contract(input: &[u8]) -> Vec<u8> {
    msg!("blake2f");
    if input.len() != BLAKE2_INPUT_LEN {
        msg!("input length for BLAKE2 F precompile should be exactly 213 bytes");
        return Vec::new();
    }

    let (rounds, rest) = input.split_at(4);
    let (h_bytes, rest) = rest.split_at(64);
    let (m_bytes, rest) = rest.split_at(128);
    let (t_bytes, f_bytes) = rest.split_at(16);

    let rounds = u32::from_be_bytes(rounds.try_into().unwrap());

    let mut h = [0u64; 8];
    for i in 0..8 {
        h[i] = u64::from_le_bytes(h_bytes[i * 8..(i + 1) * 8].try_into().unwrap());
    }

    let mut m = [0u64; 16];
    for i in 0..16 {
        m[i] = u64::from_le_bytes(m_bytes[i * 8..(i + 1) * 8].try_into().unwrap());
    }

    let mut t = [0u64; 2];
    for i in 0..2 {
        t[i] = u64::from_le_bytes(t_bytes[i * 8..(i + 1) * 8].try_into().unwrap());
    }

    let f = if f_bytes[0] == 1 {
        true
    } else if f_bytes[0] == 0 {
        false
    } else {
        msg!("incorrect final block indicator flag");
        return Vec::new();
    };

    let mut v = [0_u64; 16];
    v[..h.len()].copy_from_slice(&h); // First half from state.
    v[h.len()..].copy_from_slice(&IV); // Second half from IV.

    v[12] ^= t[0];
    v[13] ^= t[1];

    if f {
        v[14] = !v[14]; // Invert all bits if the last-block-flag is set.
    }
    for i in 0..rounds {
        round(i, &mut v, &m);
    }
    for i in 0..8 {
        h[i] ^= v[i] ^ v[i + 8];
    }

    let mut output_buf = [0_u8; 64];
    for (i, state_word) in h.iter().enumerate() {
        output_buf[i * 8..(i + 1) * 8].copy_from_slice(&state_word.to_le_bytes());
    }

    output_buf.to_vec()
}

#[cfg(test)]
mod test {
    // Test cases are from https://github.com/ethereum/EIPs/blob/master/EIPS/eip-152.md

    use crate::precompile::blake2f::contract;
    use hex;

    fn test_case(input_hex: &str, expected_result: Vec<u8>) {
        assert_eq!(
            contract(hex::decode(input_hex).unwrap().as_slice()),
            expected_result
        )
    }

    #[test]
    fn wrong_len1() {
        test_case(
            "00000c48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3a\
            f54fa5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e131\
            9cde05b61626300000000000000000000000000000000000000000000000000\
            000000000000000000000000000000000000000000000000000000000000000\
            000000000000000000000000000000000000000000000000000000000000000\
            000000000000000000000000000000000000000000000000000000000000000\
            000000000000300000000000000000000000000000001",
            Vec::new(),
        )
    }

    #[test]
    fn wrong_len2() {
        test_case(
            "00000000\
            0c48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54f\
            a5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde0\
            5b61626300000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            000300000000000000000000000000000001",
            Vec::new(),
        )
    }

    #[test]
    fn wrong_final_block_indicator() {
        test_case(
            "0000000c\
            48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5\
            d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b\
            6162630000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0300000000000000000000000000000002",
            Vec::new(),
        )
    }

    #[test]
    fn success_case1() {
        test_case(
            "00000000\
            48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5\
            d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b\
            6162630000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0300000000000000000000000000000001",
            hex::decode(
                "08c9bcf367e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5\
                d282e6ad7f520e511f6c3e2b8c68059b9442be0454267ce079217e1319cde05b",
            )
            .unwrap(),
        )
    }

    #[test]
    fn success_case2() {
        test_case(
            "0000000c\
            48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5\
            d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b\
            6162630000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0300000000000000000000000000000001",
            hex::decode(
                "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1\
                7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923",
            )
            .unwrap(),
        )
    }

    #[test]
    fn success_case3() {
        test_case(
            "0000000c\
            48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5\
            d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b\
            6162630000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0300000000000000000000000000000000",
            hex::decode(
                "75ab69d3190a562c51aef8d88f1c2775876944407270c42c9844252c26d28752\
                98743e7f6d5ea2f2d3e8d226039cd31b4e426ac4f2d3d666a610c2116fde4735",
            )
            .unwrap(),
        )
    }

    #[test]
    fn success_case4() {
        test_case(
            "00000001\
            48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5\
            d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b\
            6162630000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0300000000000000000000000000000001",
            hex::decode(
                "b63a380cb2897d521994a85234ee2c181b5f844d2c624c002677e9703449d2fb\
                a551b3a8333bcdf5f2f7e08993d53923de3d64fcc68c034e717b9293fed7a421",
            )
            .unwrap(),
        )
    }
}
