use crate::{
    constants::{
        ASSOCIATED_TOKEN_PROGRAM_ID, SEED_AGENT_WALLET, SEED_GLOBAL_CONFIG, SEED_VAULT_CONFIG,
    },
    error::AgentVaultError,
};
use pinocchio::{error::ProgramError, AccountView, Address};

pub const AGENT_WALLET_INDEX_SEED_LEN: usize = 2;
pub const REGISTRY_AGENT_ACCOUNT_SEED: &[u8] = b"agent";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pda {
    pub address: Address,
    pub bump: u8,
}

#[inline(always)]
pub fn agent_wallet_index_seed(index: u16) -> [u8; AGENT_WALLET_INDEX_SEED_LEN] {
    index.to_le_bytes()
}

pub fn derive_global_config(program_id: &Address) -> Result<Pda, ProgramError> {
    derive_pda(&[SEED_GLOBAL_CONFIG], program_id)
}

pub fn derive_vault_config(
    program_id: &Address,
    agent_asset: &Address,
) -> Result<Pda, ProgramError> {
    derive_pda(&[SEED_VAULT_CONFIG, agent_asset.as_ref()], program_id)
}

pub fn derive_agent_wallet(
    program_id: &Address,
    agent_asset: &Address,
    index: u16,
) -> Result<Pda, ProgramError> {
    let index_seed = agent_wallet_index_seed(index);
    derive_pda(
        &[SEED_AGENT_WALLET, agent_asset.as_ref(), &index_seed],
        program_id,
    )
}

pub fn derive_registry_agent_account(
    registry_program_id: &Address,
    agent_asset: &Address,
) -> Result<Pda, ProgramError> {
    derive_pda(
        &[REGISTRY_AGENT_ACCOUNT_SEED, agent_asset.as_ref()],
        registry_program_id,
    )
}

pub fn derive_associated_token_account(
    wallet: &Address,
    mint: &Address,
    token_program: &Address,
) -> Result<Pda, ProgramError> {
    derive_pda(
        &[wallet.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    )
}

pub fn validate_global_config_pda(
    address: &Address,
    stored_bump: u8,
    program_id: &Address,
) -> Result<(), ProgramError> {
    let bump_seed = [stored_bump];
    validate_bumped_pda(
        address,
        &[SEED_GLOBAL_CONFIG, &bump_seed],
        &[SEED_GLOBAL_CONFIG],
        program_id,
    )
}

pub fn validate_vault_config_pda(
    address: &Address,
    stored_bump: u8,
    program_id: &Address,
    agent_asset: &Address,
) -> Result<(), ProgramError> {
    let bump_seed = [stored_bump];
    validate_bumped_pda(
        address,
        &[SEED_VAULT_CONFIG, agent_asset.as_ref(), &bump_seed],
        &[SEED_VAULT_CONFIG, agent_asset.as_ref()],
        program_id,
    )
}

pub fn validate_agent_wallet_pda(
    address: &Address,
    stored_bump: u8,
    program_id: &Address,
    agent_asset: &Address,
    index: u16,
) -> Result<(), ProgramError> {
    let index_seed = agent_wallet_index_seed(index);
    let bump_seed = [stored_bump];
    validate_bumped_pda(
        address,
        &[
            SEED_AGENT_WALLET,
            agent_asset.as_ref(),
            &index_seed,
            &bump_seed,
        ],
        &[SEED_AGENT_WALLET, agent_asset.as_ref(), &index_seed],
        program_id,
    )
}

#[inline(always)]
pub fn assert_global_config_pda(
    account: &AccountView,
    stored_bump: u8,
    program_id: &Address,
) -> Result<(), ProgramError> {
    validate_global_config_pda(account.address(), stored_bump, program_id)
}

#[inline(always)]
pub fn assert_vault_config_pda(
    account: &AccountView,
    stored_bump: u8,
    program_id: &Address,
    agent_asset: &Address,
) -> Result<(), ProgramError> {
    validate_vault_config_pda(account.address(), stored_bump, program_id, agent_asset)
}

#[inline(always)]
pub fn assert_agent_wallet_pda(
    account: &AccountView,
    stored_bump: u8,
    program_id: &Address,
    agent_asset: &Address,
    index: u16,
) -> Result<(), ProgramError> {
    validate_agent_wallet_pda(
        account.address(),
        stored_bump,
        program_id,
        agent_asset,
        index,
    )
}

fn validate_bumped_pda(
    address: &Address,
    bumped_seeds: &[&[u8]],
    canonical_seeds: &[&[u8]],
    program_id: &Address,
) -> Result<(), ProgramError> {
    if let Ok(created) = create_pda(bumped_seeds, program_id) {
        if address == &created {
            return Ok(());
        }
    }

    let expected = derive_pda(canonical_seeds, program_id)?;
    if address == &expected.address {
        Err(AgentVaultError::InvalidBump.into())
    } else {
        Err(AgentVaultError::InvalidPda.into())
    }
}

#[inline(always)]
fn derive_pda(seeds: &[&[u8]], program_id: &Address) -> Result<Pda, ProgramError> {
    let (address, bump) =
        find_program_address(seeds, program_id).ok_or(AgentVaultError::InvalidPda)?;
    Ok(Pda { address, bump })
}

#[cfg(any(target_os = "solana", target_arch = "bpf"))]
#[inline(always)]
fn create_pda(seeds: &[&[u8]], program_id: &Address) -> Result<Address, ProgramError> {
    Address::create_program_address(seeds, program_id).map_err(|_| ProgramError::InvalidSeeds)
}

#[cfg(not(any(target_os = "solana", target_arch = "bpf")))]
#[inline(always)]
fn create_pda(seeds: &[&[u8]], program_id: &Address) -> Result<Address, ProgramError> {
    host_pda::create_program_address(seeds, program_id)
}

#[cfg(any(target_os = "solana", target_arch = "bpf"))]
#[inline(always)]
fn find_program_address(seeds: &[&[u8]], program_id: &Address) -> Option<(Address, u8)> {
    Address::try_find_program_address(seeds, program_id)
}

#[cfg(not(any(target_os = "solana", target_arch = "bpf")))]
#[inline(always)]
fn find_program_address(seeds: &[&[u8]], program_id: &Address) -> Option<(Address, u8)> {
    host_pda::find_program_address(seeds, program_id)
}

#[cfg(not(any(target_os = "solana", target_arch = "bpf")))]
mod host_pda {
    use pinocchio::{error::ProgramError, Address};

    const PDA_MARKER: &[u8; 21] = b"ProgramDerivedAddress";
    const MAX_SEEDS: usize = 16;
    const MAX_SEED_LEN: usize = 32;

    pub fn find_program_address(seeds: &[&[u8]], program_id: &Address) -> Option<(Address, u8)> {
        if seeds.len() > MAX_SEEDS || seeds.iter().any(|seed| seed.len() > MAX_SEED_LEN) {
            return None;
        }

        let mut bump = u8::MAX;
        loop {
            let bump_seed = [bump];
            let mut hasher = Sha256::new();
            for seed in seeds {
                hasher.update(seed);
            }
            hasher.update(&bump_seed);
            hasher.update(program_id.as_ref());
            hasher.update(PDA_MARKER);
            let bytes = hasher.finalize();

            if !ed25519::is_on_curve(&bytes) {
                return Some((Address::new_from_array(bytes), bump));
            }
            if bump == 0 {
                return None;
            }
            bump -= 1;
        }
    }

    pub fn create_program_address(
        seeds: &[&[u8]],
        program_id: &Address,
    ) -> Result<Address, ProgramError> {
        if seeds.len() > MAX_SEEDS || seeds.iter().any(|seed| seed.len() > MAX_SEED_LEN) {
            return Err(ProgramError::MaxSeedLengthExceeded);
        }

        let mut hasher = Sha256::new();
        for seed in seeds {
            hasher.update(seed);
        }
        hasher.update(program_id.as_ref());
        hasher.update(PDA_MARKER);
        let bytes = hasher.finalize();

        if ed25519::is_on_curve(&bytes) {
            Err(ProgramError::InvalidSeeds)
        } else {
            Ok(Address::new_from_array(bytes))
        }
    }

    struct Sha256 {
        state: [u32; 8],
        buffer: [u8; 64],
        buffer_len: usize,
        bytes_len: u64,
    }

    impl Sha256 {
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];

        fn new() -> Self {
            Self {
                state: [
                    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c,
                    0x1f83d9ab, 0x5be0cd19,
                ],
                buffer: [0; 64],
                buffer_len: 0,
                bytes_len: 0,
            }
        }

        fn update(&mut self, mut input: &[u8]) {
            self.bytes_len = self.bytes_len.wrapping_add(input.len() as u64);

            if self.buffer_len != 0 {
                let needed = 64 - self.buffer_len;
                if input.len() < needed {
                    self.buffer[self.buffer_len..self.buffer_len + input.len()]
                        .copy_from_slice(input);
                    self.buffer_len += input.len();
                    return;
                }

                self.buffer[self.buffer_len..64].copy_from_slice(&input[..needed]);
                let block = self.buffer;
                self.compress(&block);
                self.buffer_len = 0;
                input = &input[needed..];
            }

            while input.len() >= 64 {
                let mut block = [0u8; 64];
                block.copy_from_slice(&input[..64]);
                self.compress(&block);
                input = &input[64..];
            }

            if !input.is_empty() {
                self.buffer[..input.len()].copy_from_slice(input);
                self.buffer_len = input.len();
            }
        }

        fn finalize(mut self) -> [u8; 32] {
            let bit_len = self.bytes_len.wrapping_mul(8);
            self.buffer[self.buffer_len] = 0x80;
            self.buffer_len += 1;

            if self.buffer_len > 56 {
                self.buffer[self.buffer_len..64].fill(0);
                let block = self.buffer;
                self.compress(&block);
                self.buffer_len = 0;
            }

            self.buffer[self.buffer_len..56].fill(0);
            self.buffer[56..64].copy_from_slice(&bit_len.to_be_bytes());
            let block = self.buffer;
            self.compress(&block);

            let mut out = [0u8; 32];
            for (chunk, word) in out.chunks_exact_mut(4).zip(self.state) {
                chunk.copy_from_slice(&word.to_be_bytes());
            }
            out
        }

        fn compress(&mut self, block: &[u8; 64]) {
            let mut w = [0u32; 64];
            for (word, chunk) in w[..16].iter_mut().zip(block.chunks_exact(4)) {
                *word = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }

            let mut a = self.state[0];
            let mut b = self.state[1];
            let mut c = self.state[2];
            let mut d = self.state[3];
            let mut e = self.state[4];
            let mut f = self.state[5];
            let mut g = self.state[6];
            let mut h = self.state[7];

            for (i, word) in w.iter().enumerate() {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let temp1 = h
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(Self::K[i])
                    .wrapping_add(*word);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let temp2 = s0.wrapping_add(maj);

                h = g;
                g = f;
                f = e;
                e = d.wrapping_add(temp1);
                d = c;
                c = b;
                b = a;
                a = temp1.wrapping_add(temp2);
            }

            self.state[0] = self.state[0].wrapping_add(a);
            self.state[1] = self.state[1].wrapping_add(b);
            self.state[2] = self.state[2].wrapping_add(c);
            self.state[3] = self.state[3].wrapping_add(d);
            self.state[4] = self.state[4].wrapping_add(e);
            self.state[5] = self.state[5].wrapping_add(f);
            self.state[6] = self.state[6].wrapping_add(g);
            self.state[7] = self.state[7].wrapping_add(h);
        }
    }

    mod ed25519 {
        const MASK: u128 = (1u128 << 51) - 1;
        const MODULUS_BYTES: [u8; 32] = [
            0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0x7f,
        ];
        const D: FieldElement = FieldElement([
            929955233495203,
            466365720129213,
            1662059464998953,
            2033849074728123,
            1442794654840575,
        ]);
        const SQRT_M1: FieldElement = FieldElement([
            1718705420411056,
            234908883556509,
            2233514472574048,
            2117202627021982,
            765476049583133,
        ]);
        const ZERO: FieldElement = FieldElement([0, 0, 0, 0, 0]);
        const ONE: FieldElement = FieldElement([1, 0, 0, 0, 0]);

        #[derive(Clone, Copy)]
        struct FieldElement([u64; 5]);

        pub fn is_on_curve(bytes: &[u8; 32]) -> bool {
            let sign = bytes[31] >> 7;
            let Some(y) = FieldElement::from_compressed_y(bytes) else {
                return false;
            };

            let yy = y.square();
            let u = yy.sub(&ONE);
            let v = D.mul(&yy).add(&ONE);
            let Some(mut x) = sqrt_ratio(&u, &v) else {
                return false;
            };

            if x.is_negative() != (sign != 0) {
                x = x.neg();
            }

            !(x.is_zero() && sign != 0)
        }

        fn sqrt_ratio(u: &FieldElement, v: &FieldElement) -> Option<FieldElement> {
            let v2 = v.square();
            let v3 = v2.mul(v);
            let v7 = v3.square().mul(v);
            let mut r = u.mul(&v3).mul(&u.mul(&v7).pow_p58());

            let check = v.mul(&r.square());
            if check.ct_eq(u) {
                return Some(r);
            }

            let neg_u = u.neg();
            if check.ct_eq(&neg_u) {
                r = r.mul(&SQRT_M1);
                return Some(r);
            }

            None
        }

        impl FieldElement {
            fn from_compressed_y(bytes: &[u8; 32]) -> Option<Self> {
                let mut y = *bytes;
                y[31] &= 0x7f;
                if !is_less_than_modulus(&y) {
                    return None;
                }

                Some(Self([
                    load_8(&y, 0) & ((1u64 << 51) - 1),
                    (load_8(&y, 6) >> 3) & ((1u64 << 51) - 1),
                    (load_8(&y, 12) >> 6) & ((1u64 << 51) - 1),
                    (load_8(&y, 19) >> 1) & ((1u64 << 51) - 1),
                    (load_8(&y, 24) >> 12) & ((1u64 << 51) - 1),
                ]))
            }

            fn add(&self, rhs: &Self) -> Self {
                Self::reduce([
                    self.0[0] as u128 + rhs.0[0] as u128,
                    self.0[1] as u128 + rhs.0[1] as u128,
                    self.0[2] as u128 + rhs.0[2] as u128,
                    self.0[3] as u128 + rhs.0[3] as u128,
                    self.0[4] as u128 + rhs.0[4] as u128,
                ])
            }

            fn sub(&self, rhs: &Self) -> Self {
                let p0 = MASK - 18;
                let p = [p0, MASK, MASK, MASK, MASK];
                Self::reduce([
                    self.0[0] as u128 + (p[0] * 2) - rhs.0[0] as u128,
                    self.0[1] as u128 + (p[1] * 2) - rhs.0[1] as u128,
                    self.0[2] as u128 + (p[2] * 2) - rhs.0[2] as u128,
                    self.0[3] as u128 + (p[3] * 2) - rhs.0[3] as u128,
                    self.0[4] as u128 + (p[4] * 2) - rhs.0[4] as u128,
                ])
            }

            fn neg(&self) -> Self {
                ZERO.sub(self)
            }

            fn mul(&self, rhs: &Self) -> Self {
                let f0 = self.0[0] as u128;
                let f1 = self.0[1] as u128;
                let f2 = self.0[2] as u128;
                let f3 = self.0[3] as u128;
                let f4 = self.0[4] as u128;
                let g0 = rhs.0[0] as u128;
                let g1 = rhs.0[1] as u128;
                let g2 = rhs.0[2] as u128;
                let g3 = rhs.0[3] as u128;
                let g4 = rhs.0[4] as u128;

                Self::reduce([
                    f0 * g0 + 19 * (f1 * g4 + f2 * g3 + f3 * g2 + f4 * g1),
                    f0 * g1 + f1 * g0 + 19 * (f2 * g4 + f3 * g3 + f4 * g2),
                    f0 * g2 + f1 * g1 + f2 * g0 + 19 * (f3 * g4 + f4 * g3),
                    f0 * g3 + f1 * g2 + f2 * g1 + f3 * g0 + 19 * (f4 * g4),
                    f0 * g4 + f1 * g3 + f2 * g2 + f3 * g1 + f4 * g0,
                ])
            }

            fn square(&self) -> Self {
                self.mul(self)
            }

            fn pow_p58(&self) -> Self {
                let mut out = ONE;
                for bit in (0..252).rev() {
                    out = out.square();
                    if bit != 1 {
                        out = out.mul(self);
                    }
                }
                out
            }

            fn is_negative(&self) -> bool {
                self.canonical_limbs()[0] & 1 != 0
            }

            fn is_zero(&self) -> bool {
                self.ct_eq(&ZERO)
            }

            fn ct_eq(&self, rhs: &Self) -> bool {
                self.canonical_limbs() == rhs.canonical_limbs()
            }

            fn canonical_limbs(&self) -> [u64; 5] {
                let mut limbs = [
                    self.0[0] as u128,
                    self.0[1] as u128,
                    self.0[2] as u128,
                    self.0[3] as u128,
                    self.0[4] as u128,
                ];
                carry_reduce(&mut limbs);
                let mut out = [
                    limbs[0] as u64,
                    limbs[1] as u64,
                    limbs[2] as u64,
                    limbs[3] as u64,
                    limbs[4] as u64,
                ];

                let p = [
                    (MASK - 18) as u64,
                    MASK as u64,
                    MASK as u64,
                    MASK as u64,
                    MASK as u64,
                ];
                if ge_limbs(&out, &p) {
                    let reduced = Self(out).sub(&Self(p));
                    out = reduced.0;
                }
                out
            }

            fn reduce(mut limbs: [u128; 5]) -> Self {
                carry_reduce(&mut limbs);
                Self([
                    limbs[0] as u64,
                    limbs[1] as u64,
                    limbs[2] as u64,
                    limbs[3] as u64,
                    limbs[4] as u64,
                ])
            }
        }

        fn carry_reduce(limbs: &mut [u128; 5]) {
            for _ in 0..2 {
                let carry0 = limbs[0] >> 51;
                limbs[0] &= MASK;
                limbs[1] += carry0;

                let carry1 = limbs[1] >> 51;
                limbs[1] &= MASK;
                limbs[2] += carry1;

                let carry2 = limbs[2] >> 51;
                limbs[2] &= MASK;
                limbs[3] += carry2;

                let carry3 = limbs[3] >> 51;
                limbs[3] &= MASK;
                limbs[4] += carry3;

                let carry4 = limbs[4] >> 51;
                limbs[4] &= MASK;
                limbs[0] += carry4 * 19;
            }

            let carry0 = limbs[0] >> 51;
            limbs[0] &= MASK;
            limbs[1] += carry0;
        }

        fn ge_limbs(a: &[u64; 5], b: &[u64; 5]) -> bool {
            for i in (0..5).rev() {
                if a[i] > b[i] {
                    return true;
                }
                if a[i] < b[i] {
                    return false;
                }
            }
            true
        }

        fn is_less_than_modulus(bytes: &[u8; 32]) -> bool {
            for i in (0..32).rev() {
                if bytes[i] < MODULUS_BYTES[i] {
                    return true;
                }
                if bytes[i] > MODULUS_BYTES[i] {
                    return false;
                }
            }
            false
        }

        fn load_8(bytes: &[u8; 32], offset: usize) -> u64 {
            u64::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ])
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn sha256_known_vector() {
            let mut hasher = Sha256::new();
            hasher.update(b"abc");
            assert_eq!(
                hasher.finalize(),
                [
                    0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d,
                    0xae, 0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10,
                    0xff, 0x61, 0xf2, 0x00, 0x15, 0xad,
                ]
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_wallet_index_seed_is_little_endian() {
        assert_eq!(agent_wallet_index_seed(0), [0, 0]);
        assert_eq!(agent_wallet_index_seed(1), [1, 0]);
        assert_eq!(agent_wallet_index_seed(0x1234), [0x34, 0x12]);
        assert_eq!(agent_wallet_index_seed(u16::MAX), [0xff, 0xff]);
    }

    #[test]
    fn pda_bump_validation_rejects_wrong_bump() {
        let program_id = Address::new_from_array([8u8; 32]);
        let pda = derive_global_config(&program_id).unwrap();
        let wrong_bump = pda.bump.wrapping_sub(1);

        assert_eq!(
            validate_global_config_pda(&pda.address, wrong_bump, &program_id),
            Err(AgentVaultError::InvalidBump.into())
        );
    }

    #[test]
    fn pda_address_validation_rejects_wrong_address() {
        let program_id = Address::new_from_array([8u8; 32]);
        let pda = derive_global_config(&program_id).unwrap();
        let wrong_address = Address::new_from_array([9u8; 32]);

        assert_eq!(
            validate_global_config_pda(&wrong_address, pda.bump, &program_id),
            Err(AgentVaultError::InvalidPda.into())
        );
    }

    #[test]
    fn host_pda_allows_solana_max_seed_count() {
        let program_id = Address::new_from_array([8u8; 32]);
        let seeds: [&[u8]; 16] = [
            b"0".as_ref(),
            b"1".as_ref(),
            b"2".as_ref(),
            b"3".as_ref(),
            b"4".as_ref(),
            b"5".as_ref(),
            b"6".as_ref(),
            b"7".as_ref(),
            b"8".as_ref(),
            b"9".as_ref(),
            b"10".as_ref(),
            b"11".as_ref(),
            b"12".as_ref(),
            b"13".as_ref(),
            b"14".as_ref(),
            b"15".as_ref(),
        ];

        assert!(derive_pda(&seeds, &program_id).is_ok());
    }
}
