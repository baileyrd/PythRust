//! SHA-256: what `hashlib.sha256(data).hexdigest()` computes, by the
//! `sha2` crate (RustCrypto), which uses the CPU's SHA instructions where
//! it has them. The ABI that exposes it lives with the objects it hashes;
//! the interpreter includes this file, so both backends hash alike.

use sha2::{Digest, Sha256};

/// The SHA-256 digest of `data`.
pub(crate) fn digest(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
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
}
