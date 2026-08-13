use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use blake3::{Hash, Hasher};

use super::DedupError;

const CHUNK_SIZE: u64 = 64 * 1024;
const SAMPLE_SIZE: u64 = 3 * CHUNK_SIZE;

/// A cheap, content-based fingerprint used to group candidate duplicates.
///
/// The fingerprint consists of the file size and a BLAKE3 hash of
/// 64 KiB samples from the start, middle, and end of sufficiently
/// large files. It is a candidate key, not proof that two files
/// have identical contents.
#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Fingerprint {
    size: u64,
    partial_hash: Hash,
}

pub fn fingerprint_file(path: &Path) -> Result<Fingerprint, DedupError> {
    let metadata = fs::metadata(path)?;
    let size = metadata.len();

    // This makes middle chunk centered around the middle of the file
    let mut file = fs::File::open(path)?;
    let mut hasher = Hasher::new();

    if size >= SAMPLE_SIZE {
        let mut buffer = vec![0; CHUNK_SIZE as usize];
        hasher.update(b"START");
        file.read_exact(&mut buffer)?;
        hasher.update(&buffer);

        hasher.update(b"MIDDLE");
        let middle_chunk = (size - CHUNK_SIZE) / 2;
        file.seek(SeekFrom::Start(middle_chunk))?;
        file.read_exact(&mut buffer)?;
        hasher.update(&buffer);

        hasher.update(b"END");
        let end_chunk = size - CHUNK_SIZE;
        file.seek(SeekFrom::Start(end_chunk))?;
        file.read_exact(&mut buffer)?;
        hasher.update(&buffer);
    } else {
        let mut buffer = vec![0; size as usize];
        file.read_exact(&mut buffer)?;
        hasher.update(&buffer);
    }

    Ok(Fingerprint {
        size,
        partial_hash: hasher.finalize(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::Path;
    use tempfile::{NamedTempFile, tempdir};

    // ---- helpers -----------------------------------------------------

    fn file_with(bytes: &[u8]) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("create temp file");
        f.write_all(bytes).expect("write temp file");
        f.flush().expect("flush temp file");
        f
    }

    fn pattern(byte: u8, len: usize) -> Vec<u8> {
        vec![byte; len]
    }

    // ====================================================================
    // 1. I/O error paths
    // ====================================================================

    #[test]
    fn errors_on_nonexistent_path() {
        let path = Path::new("/definitely/does/not/exist/xyz_abc_123.bin");
        assert!(matches!(
            fingerprint_file(path),
            Err(DedupError::FileRead(_))
        ));
    }

    #[test]
    fn errors_on_directory_path() {
        // On Unix this typically fails inside read_exact (EISDIR);
        // on Windows it usually fails at File::open. Either way the
        // caller should just see FileReadError.
        let dir = tempdir().expect("create temp dir");
        assert!(matches!(
            fingerprint_file(dir.path()),
            Err(DedupError::FileRead(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn errors_on_unreadable_file() {
        use std::os::unix::fs::PermissionsExt;
        let file = file_with(b"top secret");
        let mut perms = std::fs::metadata(file.path()).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(file.path(), perms).unwrap();

        let result = fingerprint_file(file.path());

        // Restore permissions so the tempfile Drop impl can clean up.
        let mut perms = std::fs::metadata(file.path()).unwrap().permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(file.path(), perms).unwrap();

        // Skip/ignore this assertion if your CI runs tests as root --
        // root bypasses POSIX permission bits, so this would spuriously
        // fail there.
        assert!(matches!(result, Err(DedupError::FileRead(_))));
    }

    // ====================================================================
    // 2. Symlinks
    // ====================================================================

    #[cfg(unix)]
    #[test]
    fn errors_on_broken_symlink() {
        let dir = tempdir().unwrap();
        let link = dir.path().join("broken");
        std::os::unix::fs::symlink("/no/such/target", &link).unwrap();
        assert!(matches!(
            fingerprint_file(&link),
            Err(DedupError::FileRead(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn valid_symlink_matches_its_target() {
        let target = file_with(b"hello world, this is the target file");
        let dir = tempdir().unwrap();
        let link = dir.path().join("good_link");
        std::os::unix::fs::symlink(target.path(), &link).unwrap();

        let direct = fingerprint_file(target.path()).unwrap();
        let via_link = fingerprint_file(&link).unwrap();
        assert_eq!(direct.size, via_link.size);
        assert_eq!(direct.partial_hash, via_link.partial_hash);
    }

    // ====================================================================
    // 3. Special files & unusual paths
    // ====================================================================

    #[test]
    fn handles_unicode_and_symbols_in_filename() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("héllo wörld 世界 🚀.bin");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"unicode path test").unwrap();
        f.flush().unwrap();
        assert!(fingerprint_file(&path).is_ok());
    }

    // #[cfg(unix)]
    // #[test]
    // fn dev_null_is_treated_as_a_zero_byte_file() {
    //     // Char devices generally report length 0 via metadata. Included
    //     // mainly to document current behavior for non-regular files --
    //     // NOT a recommendation to fingerprint every special file found
    //     // while walking a directory tree (see FIFO note below).
    //     let result = fingerprint_file(Path::new("/dev/null")).unwrap();
    //     assert_eq!(result.size, 0);
    // }

    // NOTE ON FIFOs (deliberately not a test): opening a named pipe with
    // File::open blocks until a writer connects. If this function (or a
    // caller walking a directory tree) ever points it at a FIFO, it will
    // hang rather than error. Worth filtering to
    // `metadata.file_type().is_file()` before calling this, if that
    // filtering doesn't already happen upstream.

    // ====================================================================
    // 4. Empty / minimal files
    // ====================================================================

    #[test]
    fn empty_file_succeeds_with_size_zero() {
        let file = file_with(b"");
        let result = fingerprint_file(file.path()).expect("empty file should not error");
        assert_eq!(result.size, 0);
    }

    #[test]
    fn single_byte_file_succeeds() {
        let file = file_with(&[0x7F]);
        let result = fingerprint_file(file.path()).expect("1-byte file should not error");
        assert_eq!(result.size, 1);
    }

    // ====================================================================
    // 5. The 192 KiB threshold -- boundary arithmetic
    // ====================================================================

    #[test]
    fn just_below_threshold_takes_whole_file_branch() {
        let bytes = pattern(0xAB, SAMPLE_SIZE as usize - 1);
        let file = file_with(&bytes);
        assert!(fingerprint_file(file.path()).is_ok());
    }

    #[test]
    fn exactly_at_threshold_does_not_panic() {
        // `size - CHUNK_SIZE` is the riskiest line in the function
        // (unsigned subtraction). This is the cheapest possible
        // regression guard against that ever underflowing.
        let bytes = pattern(0xCD, SAMPLE_SIZE as usize);
        let file = file_with(&bytes);
        assert!(fingerprint_file(file.path()).is_ok());
    }

    #[test]
    fn one_byte_above_threshold_does_not_panic() {
        let bytes = pattern(0xEF, SAMPLE_SIZE as usize + 1);
        let file = file_with(&bytes);
        assert!(fingerprint_file(file.path()).is_ok());
    }

    #[test]
    fn threshold_neighbors_produce_different_fingerprints() {
        let below = pattern(0x11, SAMPLE_SIZE as usize - 1);
        let at = pattern(0x11, SAMPLE_SIZE as usize);
        let f_below = file_with(&below);
        let f_at = file_with(&at);

        let r_below = fingerprint_file(f_below.path()).unwrap();
        let r_at = fingerprint_file(f_at.path()).unwrap();

        assert_ne!(r_below.size, r_at.size);
        assert_ne!(r_below.partial_hash, r_at.partial_hash);
    }

    #[test]
    fn middle_offset_formula_centers_the_chunk() {
        // Pure arithmetic sanity check (no file I/O): confirms
        // `(size - CHUNK_SIZE) / 2` really does center a CHUNK_SIZE-wide
        // window on size / 2, matching the function's doc comment.
        let size = 10_000_000u64;
        let middle_chunk = (size - CHUNK_SIZE) / 2;
        let chunk_center = middle_chunk + CHUNK_SIZE / 2;
        let file_center = size / 2;
        assert!(chunk_center.abs_diff(file_center) <= 1);
    }

    // ====================================================================
    // 6. Determinism & content sensitivity
    // ====================================================================

    #[test]
    fn same_content_gives_same_fingerprint() {
        let bytes = pattern(0x42, 500 * 1024);
        let a = file_with(&bytes);
        let b = file_with(&bytes);
        let ra = fingerprint_file(a.path()).unwrap();
        let rb = fingerprint_file(b.path()).unwrap();
        assert_eq!(ra.size, rb.size);
        assert_eq!(ra.partial_hash, rb.partial_hash);
    }

    #[test]
    fn repeated_calls_are_deterministic() {
        let bytes = pattern(0x99, 10 * 1024);
        let file = file_with(&bytes);
        let r1 = fingerprint_file(file.path()).unwrap();
        let r2 = fingerprint_file(file.path()).unwrap();
        assert_eq!(r1.partial_hash, r2.partial_hash);
    }

    #[test]
    fn small_files_one_byte_apart_hash_differently() {
        // Below the threshold the WHOLE file is hashed, so any
        // single-byte difference anywhere must change the result -- no
        // sampling blind spots for small files.
        let mut a = pattern(0x00, 10 * 1024);
        let b = a.clone();
        a[5_000] = 0xFF;

        let fa = file_with(&a);
        let fb = file_with(&b);
        let ra = fingerprint_file(fa.path()).unwrap();
        let rb = fingerprint_file(fb.path()).unwrap();

        assert_eq!(ra.size, rb.size);
        assert_ne!(ra.partial_hash, rb.partial_hash);
    }

    #[test]
    fn large_files_differing_in_start_chunk_hash_differently() {
        let size = 400 * 1024;
        let mut a = pattern(0x01, size);
        let b = a.clone();
        a[0] = 0xFF;
        let fa = file_with(&a);
        let fb = file_with(&b);
        assert_ne!(
            fingerprint_file(fa.path()).unwrap().partial_hash,
            fingerprint_file(fb.path()).unwrap().partial_hash
        );
    }

    #[test]
    fn large_files_differing_in_middle_chunk_hash_differently() {
        let size = 400 * 1024;
        let mut a = pattern(0x01, size);
        let b = a.clone();
        let mid = ((size as u64 - CHUNK_SIZE) / 2) as usize;
        a[mid] = 0xFF;
        let fa = file_with(&a);
        let fb = file_with(&b);
        assert_ne!(
            fingerprint_file(fa.path()).unwrap().partial_hash,
            fingerprint_file(fb.path()).unwrap().partial_hash
        );
    }

    #[test]
    fn large_files_differing_in_end_chunk_hash_differently() {
        let size = 400 * 1024;
        let mut a = pattern(0x01, size);
        let b = a.clone();
        a[size - 1] = 0xFF;
        let fa = file_with(&a);
        let fb = file_with(&b);
        assert_ne!(
            fingerprint_file(fa.path()).unwrap().partial_hash,
            fingerprint_file(fb.path()).unwrap().partial_hash
        );
    }

    #[test]
    fn large_files_differing_only_outside_sampled_regions_collide() {
        // KNOWN LIMITATION, deliberately locked in here: for files at or
        // above the threshold, only 3 * CHUNK_SIZE bytes are ever
        // sampled. Two different files of the same size that agree on
        // START/MIDDLE/END but disagree in the untouched gaps between
        // them get the SAME fingerprint. If the deduper ever treats
        // fingerprint equality as final proof of duplication (rather
        // than a fast pre-filter followed by a full byte-for-byte
        // compare), this is a genuine false-positive source.
        let size = 400 * 1024;
        let mut a = pattern(0x01, size);
        let mut b = pattern(0x01, size);

        let gap_offset = CHUNK_SIZE as usize + 10; // just past the START chunk
        a[gap_offset] = 0xAA;
        b[gap_offset] = 0xBB;

        let fa = file_with(&a);
        let fb = file_with(&b);
        let ra = fingerprint_file(fa.path()).unwrap();
        let rb = fingerprint_file(fb.path()).unwrap();

        assert_eq!(ra.size, rb.size);
        assert_eq!(
            ra.partial_hash, rb.partial_hash,
            "expected the sampling gap to produce a collision here"
        );
    }

    #[test]
    fn swapping_start_and_end_content_still_hashes_differently() {
        // Confirms the "START"/"MIDDLE"/"END" label bytes actually
        // domain-separate the three chunks, so you can't get a hash
        // collision just by moving a chunk's content from one position
        // to another.
        let size = 400 * 1024;
        let chunk = CHUNK_SIZE as usize;

        let mut a = pattern(0x00, size);
        a[0..chunk].fill(0x01);
        a[size - chunk..size].fill(0x02);

        let mut b = pattern(0x00, size);
        b[0..chunk].fill(0x02);
        b[size - chunk..size].fill(0x01);

        let fa = file_with(&a);
        let fb = file_with(&b);
        assert_ne!(
            fingerprint_file(fa.path()).unwrap().partial_hash,
            fingerprint_file(fb.path()).unwrap().partial_hash
        );
    }

    // ====================================================================
    // 7. Safety: fingerprinting must be read-only
    // ====================================================================

    #[test]
    fn fingerprinting_does_not_modify_the_file() {
        let bytes = pattern(0x33, 300 * 1024);
        let file = file_with(&bytes);
        let before = std::fs::read(file.path()).unwrap();

        let _ = fingerprint_file(file.path()).unwrap();

        let after = std::fs::read(file.path()).unwrap();
        assert_eq!(
            before, after,
            "fingerprinting must never mutate file contents"
        );
    }

    // ====================================================================
    // 8. Expensive cases -- run explicitly, not on every `cargo test`
    // ====================================================================

    #[test]
    #[ignore = "slow: allocates a multi-GB sparse file"]
    fn multi_gigabyte_sparse_file_does_not_panic() {
        let file = NamedTempFile::new().unwrap();
        file.as_file().set_len(4 * 1024 * 1024 * 1024).unwrap(); // 4 GiB, sparse
        assert!(fingerprint_file(file.path()).is_ok());
    }

    // ====================================================================
    // 9. Best-effort concurrency
    // ====================================================================

    #[test]
    #[ignore = "timing-dependent; run manually / in a loop, not routine CI"]
    fn shrinking_file_during_fingerprinting_never_panics() {
        // Not deterministic -- this tries to win a race between the
        // function's own reads and a background truncation. The
        // property under test is "never panics, and only Ok(..) or
        // Err(FileReadError) ever comes out," not any specific outcome.
        for _ in 0..20 {
            let bytes = pattern(0x01, 2 * 1024 * 1024); // widen the race window
            let file = file_with(&bytes);
            let path = file.path().to_path_buf();

            let writer = std::thread::spawn({
                let path = path.clone();
                move || {
                    std::thread::sleep(std::time::Duration::from_micros(20));
                    if let Ok(f) = std::fs::OpenOptions::new().write(true).open(&path) {
                        let _ = f.set_len(50);
                    }
                }
            });

            let outcome = fingerprint_file(&path);
            writer.join().unwrap();

            assert!(matches!(outcome, Ok(_) | Err(DedupError::FileRead(_))));
        }
    }
}
