//! SHA-256 (FIPS 180-4), hand-written so the runtime needs no
//! dependency: what `hashlib.sha256(data).hexdigest()` computes. Safe
//! Rust only; the ABI that exposes it lives with the objects it hashes.

/// The first 32 bits of the fractional parts of the cube roots of the
/// first 64 primes.
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// The first 32 bits of the fractional parts of the square roots of the
/// first 8 primes.
const H0: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// Mixes one 64-byte block into `h`, with a 16-word circular schedule.
fn compress(h: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 16];
    for (slot, word) in w.iter_mut().zip(block.as_chunks::<4>().0) {
        *slot = u32::from_be_bytes(*word);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = *h;
    // Each invocation has a constant round number: schedule indexes and K
    // are static. Before round t >= 16, slot t % 16 still holds W[t-16];
    // the other three reads hold W[t-15], W[t-7], W[t-2]. Replace it only
    // after those reads. All additions are modulo 2^32 (FIPS 180-4).
    macro_rules! round {
        ($t:expr, $a:ident, $b:ident, $c:ident, $d:ident,
         $e:ident, $f:ident, $g:ident, $h:ident) => {{
            if $t >= 16 {
                let x = w[($t + 1) % 16];
                let y = w[($t + 14) % 16];
                let s0 = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
                let s1 = y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10);
                w[$t % 16] = w[$t % 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[($t + 9) % 16])
                    .wrapping_add(s1);
            }
            let s1 = $e.rotate_right(6) ^ $e.rotate_right(11) ^ $e.rotate_right(25);
            let ch = ($e & $f) ^ (!$e & $g);
            let t1 = $h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[$t])
                .wrapping_add(w[$t % 16]);
            let s0 = $a.rotate_right(2) ^ $a.rotate_right(13) ^ $a.rotate_right(22);
            let maj = ($a & $b) ^ ($a & $c) ^ ($b & $c);
            $d = $d.wrapping_add(t1);
            $h = t1.wrapping_add(s0.wrapping_add(maj));
        }};
    }
    // Rotate the *names* instead of copying eight state words per round.
    // Eight rounds restore the original name order.
    macro_rules! eight_rounds {
        ($t:expr) => {
            round!($t, a, b, c, d, e, f, g, hh);
            round!($t + 1, hh, a, b, c, d, e, f, g);
            round!($t + 2, g, hh, a, b, c, d, e, f);
            round!($t + 3, f, g, hh, a, b, c, d, e);
            round!($t + 4, e, f, g, hh, a, b, c, d);
            round!($t + 5, d, e, f, g, hh, a, b, c);
            round!($t + 6, c, d, e, f, g, hh, a, b);
            round!($t + 7, b, c, d, e, f, g, hh, a);
        };
    }
    eight_rounds!(0);
    eight_rounds!(8);
    eight_rounds!(16);
    eight_rounds!(24);
    eight_rounds!(32);
    eight_rounds!(40);
    eight_rounds!(48);
    eight_rounds!(56);
    for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
        *x = x.wrapping_add(y);
    }
}

/// The SHA-256 digest of `data`.
pub(crate) fn digest(data: &[u8]) -> [u8; 32] {
    let mut h = H0;
    let (blocks, tail) = data.as_chunks::<64>();
    blocks.iter().for_each(|b| compress(&mut h, b));
    // The tail, a 1 bit, zeros, and the length in bits: one or two blocks.
    let mut last = [0u8; 128];
    last[..tail.len()].copy_from_slice(tail);
    last[tail.len()] = 0x80;
    let len = if tail.len() < 56 { 64 } else { 128 };
    let bits = bit_length(data.len() as u64);
    last[len - 8..len].copy_from_slice(&bits.to_be_bytes());
    let padded = last[..len].as_chunks::<64>().0;
    padded.iter().for_each(|b| compress(&mut h, b));
    let mut out = [0u8; 32];
    for (chunk, word) in out.as_chunks_mut::<4>().0.iter_mut().zip(h) {
        *chunk = word.to_be_bytes();
    }
    out
}

/// SHA's low 64 length bits; separate so overflow is testable without
/// constructing a slice exceeding the address space. Preserves prior wrapping.
fn bit_length(bytes: u64) -> u64 {
    bytes.wrapping_mul(8)
}

/// `hexdigest()`: lowercase hex of `digest`.
pub(crate) fn hex(digest: &[u8]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hexdigest(data: &[u8]) -> String {
        hex(&digest(data))
    }

    #[test]
    fn matches_cpython_and_fips_vectors() {
        let cases: [(Vec<u8>, &str); 4] = [
            (
                b"".to_vec(),
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                b"abc".to_vec(),
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                vec![b'a'; 1_000_000],
                "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
            ),
            (
                (0..768).map(|i| (i % 256) as u8).collect(),
                "f3a25aa93aa2fbba28d79260535bbd6a5eb0fc1c24a8b0f04e12b484c1dfe363",
            ),
        ];
        for (data, want) in cases {
            assert_eq!(hexdigest(&data), want, "{} bytes", data.len());
        }
    }

    #[test]
    fn padding_boundaries_match_cpython() {
        // 55 bytes pad into one block, 56 into two; 64 fills one exactly.
        let want = [
            (
                55,
                "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318",
            ),
            (
                56,
                "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a",
            ),
            (
                64,
                "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb",
            ),
        ];
        for (n, hex) in want {
            assert_eq!(hexdigest(&vec![b'a'; n]), hex, "{n} bytes");
        }
    }
    #[test]
    fn deterministic_patterns_match_cpython() {
        // CPython 3.13 hashlib, byte i = (i * 37 + 11) % 256.
        let cases = [
            (
                0,
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                1,
                "e7cf46a078fed4fafd0b5e3aff144802b853f8ae459a4f0c14add3314b7cc3a6",
            ),
            (
                55,
                "2900465fcb533e05a158fd2b3be0e5e3b03740d83060aa3580e0d98a96bf2384",
            ),
            (
                56,
                "31454ff48ef36af2f08fd511bdc37d9d5855ac23e992e5ff5445cb6b7674a674",
            ),
            (
                63,
                "5f6401b96532c36de4e65beec0409b69b1d181864c8009b7a04f43e5d56350d1",
            ),
            (
                64,
                "94eb5de4943613fd048dc93393ab06877405faa39c11f53e9386083339833e7e",
            ),
            (
                65,
                "fc518669b6eb4b4dd91827ecacef86689c725bd5bab888fd3b26dbb196eec954",
            ),
            (
                119,
                "b0dc41b1a384e2f1203f0351b38fbeaafceef577ce1191d5bfc25da39f721eae",
            ),
            (
                120,
                "5df24dd802ac26132ce608dcb5f09841eef039ee0f152acf98d26d17fe4e88e6",
            ),
            (
                127,
                "0fe729ff19257bd6fec853acc2ea355f6b34b58e6c0f684c3e188fcdfcd9baae",
            ),
            (
                128,
                "0aedd4856f8eba0963627336ad5144a9a7dbe12498e6066f0165fc97d8ddee4c",
            ),
            (
                129,
                "4f1757ae4bffbae86d775b831765b75af154d52f7deaa46dd378051a2d3ad57f",
            ),
            (
                1024,
                "ffbad8f947474cfdd5b2bb22d7e0bf5ee8ba2b7af859d0c2bb28622db6a4be47",
            ),
            (
                65537,
                "3123afca912e156e19bcd2055eb92e944cf6e6fdb0eef16d65464342ad1f5a1a",
            ),
        ];
        for (n, want) in cases {
            let data: Vec<u8> = (0..n).map(|i| ((i * 37 + 11) % 256) as u8).collect();
            assert_eq!(hexdigest(&data), want, "{n} bytes");
        }
    }

    #[test]
    fn bit_length_preserves_wrapping_without_huge_slices() {
        assert_eq!(bit_length(0), 0);
        assert_eq!(bit_length(1), 8);
        assert_eq!(bit_length((1 << 61) - 1), u64::MAX - 7);
        assert_eq!(bit_length(1 << 61), 0);
        assert_eq!(bit_length((1 << 61) + 1), 8);
        assert_eq!(bit_length(u64::MAX), u64::MAX - 7);
    }
}

/// Opt-in ordinary release microbenchmarks, separate from correctness tests.
/// The #809 harness compiles this same module against both source revisions.
#[cfg(test)]
mod microbench {
    use super::*;
    use std::hint::black_box;
    use std::time::Instant;

    #[test]
    #[ignore = "manual release timing; use bench/sha256_pilot.py"]
    fn compression() {
        let block = black_box([0x5au8; 64]);
        let mut state = H0;
        let start = Instant::now();
        for _ in 0..2_097_152 {
            compress(black_box(&mut state), black_box(&block));
        }
        println!("sha256_micro_seconds={:.9}", start.elapsed().as_secs_f64());
        black_box(state);
    }

    #[test]
    #[ignore = "manual release timing; use bench/sha256_pilot.py"]
    fn loaded_digest() {
        let data = black_box(vec![0x5au8; 32 * 1024 * 1024]);
        let start = Instant::now();
        for _ in 0..4 {
            black_box(digest(black_box(&data)));
        }
        println!("sha256_micro_seconds={:.9}", start.elapsed().as_secs_f64());
    }
}
