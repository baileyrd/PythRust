//! fastfs: file hashing for `fclone_fast.pyr`, called through Pythrust's
//! Rust interop (`pythrust build --rust fastfs=...`).

use std::fs::File;
use std::io::{self, Read};
use std::num::NonZero;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

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

/// The [`file_xxh3`] digest of each of `paths`, in order, or `""` for
/// one that cannot be read (the caller skips it, as fclone-lite skips
/// an `OSError`). Hashed on one scoped thread per core, each taking the
/// next path as it finishes one, so uneven sizes balance.
pub fn group_xxh3(paths: &[String]) -> Vec<String> {
    let cores = thread::available_parallelism().map_or(1, NonZero::get);
    let workers = cores.min(paths.len()).max(1);
    let next = AtomicUsize::new(0);
    let hash_some = || {
        let mut done = Vec::new();
        loop {
            let i = next.fetch_add(1, Ordering::Relaxed);
            let Some(path) = paths.get(i) else {
                return done;
            };
            done.push((i, file_xxh3(path).unwrap_or_default()));
        }
    };
    let parts: Vec<Vec<(usize, String)>> = thread::scope(|s| {
        let handles: Vec<_> = (0..workers).map(|_| s.spawn(&hash_some)).collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic)))
            .collect()
    });
    let mut out = vec![String::new(); paths.len()];
    for (i, digest) in parts.into_iter().flatten() {
        if let Some(slot) = out.get_mut(i) {
            *slot = digest;
        }
    }
    out
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

    #[test]
    fn a_group_hashes_each_path_in_order() {
        let dir = std::env::temp_dir().join(format!("fastfs-group-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let paths: Vec<String> = (0..20)
            .map(|i| {
                let path = dir.join(format!("f{i}"));
                std::fs::write(&path, vec![i as u8; i * 1000]).unwrap();
                path.to_str().unwrap().to_string()
            })
            .chain([dir.join("missing").to_str().unwrap().to_string()])
            .collect();
        let got = group_xxh3(&paths);
        let want: Vec<String> = paths
            .iter()
            .map(|p| file_xxh3(p).unwrap_or_default())
            .collect();
        assert_eq!(got, want);
        assert_eq!(got.last().map(String::as_str), Some(""));
        assert!(group_xxh3(&[]).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
