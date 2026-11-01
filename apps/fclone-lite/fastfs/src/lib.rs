//! fastfs: file hashing for `fclone_fast.pyr`, called through Pythrust's
//! Rust interop (`pythrust build --rust fastfs=...`).

use std::fs::File;
use std::io::{self, Read};

use xxhash_rust::xxh3::Xxh3;

/// Bytes read per chunk while hashing.
const CHUNK: usize = 1 << 16;

/// The xxh3-128 digest of the file at `path`, streamed in chunks, as 32
/// lowercase hex digits.
pub fn file_xxh3(path: &str) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Xxh3::new();
    let mut buf = vec![0_u8; CHUNK];
    loop {
        let n = match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:032x}", hasher.digest128()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digests_match_one_shot_xxh3() {
        let dir = std::env::temp_dir().join(format!("fastfs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("data");
        let data: Vec<u8> = (0..CHUNK * 2 + 7).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &data).unwrap();
        let want = format!("{:032x}", xxhash_rust::xxh3::xxh3_128(&data));
        assert_eq!(file_xxh3(path.to_str().unwrap()).unwrap(), want);
        std::fs::write(&path, b"").unwrap();
        assert_eq!(file_xxh3(path.to_str().unwrap()).unwrap().len(), 32);
        assert!(file_xxh3(dir.join("missing").to_str().unwrap()).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
