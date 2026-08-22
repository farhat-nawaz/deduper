use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    path::Path,
};

use blake3::{Hash, Hasher};

use crate::error::DedupError;
use crate::files::FileInfo;
use crate::fingerprint::{Fingerprint, fingerprint_file};

pub(crate) struct CandidateGroup<'a> {
    files: Vec<&'a FileInfo>,
    fingerprint: Fingerprint,
}

pub(crate) struct DuplicateGroup<'a> {
    pub(crate) files: Vec<&'a FileInfo>,
}

pub fn find_candidates(files: &[FileInfo]) -> Result<Vec<CandidateGroup<'_>>, DedupError> {
    let mut fingerprints: HashMap<Fingerprint, Vec<&FileInfo>> = HashMap::new();

    for file in files {
        let fingerprint = fingerprint_file(file)?;
        fingerprints.entry(fingerprint).or_default().push(file);
    }

    fingerprints.retain(|_, paths| paths.len() > 1);
    Ok(fingerprints
        .into_iter()
        .map(|(fingerprint, files)| CandidateGroup { files, fingerprint })
        .collect())
}

pub fn find_duplicates<'a>(
    candidates: &[CandidateGroup<'a>],
) -> Result<Vec<DuplicateGroup<'a>>, DedupError> {
    let mut duplicates: HashMap<Hash, Vec<&FileInfo>> = HashMap::new();
    for candidate_group in candidates {
        for file in &*candidate_group.files {
            print!("\r\x1b[2KProcessing: {}", file.path.display());
            std::io::stdout().flush().unwrap();

            let hash = hash_file(&file.path)?;
            duplicates.entry(hash).or_default().push(file);
        }
    }
    duplicates.retain(|_, paths| paths.len() > 1);

    Ok(duplicates
        .into_values()
        .map(|files| DuplicateGroup { files })
        .collect())
}

// TODO: pass size as well to make sure the file hasn't changed in between
pub fn hash_file(path: &Path) -> Result<Hash, DedupError> {
    let mut hasher = Hasher::new();
    let mut file = fs::File::open(path)?;
    io::copy(&mut file, &mut hasher)?;

    Ok(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    const CHUNK_SIZE: usize = 64 * 1024;

    // ---- helpers -------------------------------------------------------

    fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> FileInfo {
        let path = dir.join(name);
        std::fs::write(&path, bytes).expect("write test file");

        FileInfo::try_from(path).unwrap()
    }

    fn sparse_file(dir: &Path, name: &str, len: u64) -> PathBuf {
        let path = dir.join(name);
        let f = std::fs::File::create(&path).expect("create sparse file");
        f.set_len(len).expect("set sparse file length");
        path
    }

    fn pattern(byte: u8, len: usize) -> Vec<u8> {
        vec![byte; len]
    }

    /// Sorts each group and then the outer list, so results can be
    /// compared without depending on HashMap iteration order.
    fn normalize_groups(groups: Vec<Vec<&FileInfo>>) -> Vec<Vec<&FileInfo>> {
        let mut groups = groups;

        for group in &mut groups {
            group.sort_by_key(|file| &file.path);
        }

        groups.sort_by(|a, b| a.iter().map(|f| &f.path).cmp(b.iter().map(|f| &f.path)));

        groups
    }

    #[cfg(test)]
    impl Clone for FileInfo {
        fn clone(&self) -> Self {
            Self {
                path: self.path.clone(),
                size: self.size,
                modified: self.modified,
            }
        }
    }

    // ====================================================================
    // 1. hash_file
    // ====================================================================

    #[test]
    fn hashes_a_normal_file_successfully() {
        let dir = tempdir().unwrap();
        let file_info = write_file(dir.path(), "normal.bin", b"just some ordinary content");
        assert!(hash_file(&file_info.path).is_ok());
    }

    #[test]
    fn same_content_produces_same_hash() {
        let dir = tempdir().unwrap();
        let content = pattern(0x88, 50 * 1024);
        let a = write_file(dir.path(), "a.bin", &content);
        let b = write_file(dir.path(), "b.bin", &content);
        assert_eq!(hash_file(&a.path).unwrap(), hash_file(&b.path).unwrap());
    }

    #[test]
    fn different_content_produces_different_hash() {
        let dir = tempdir().unwrap();
        let a = write_file(dir.path(), "a.bin", b"content A");
        let b = write_file(dir.path(), "b.bin", b"content B, not the same");
        assert_ne!(hash_file(&a.path).unwrap(), hash_file(&b.path).unwrap());
    }

    #[test]
    fn empty_file_hashes_successfully() {
        let dir = tempdir().unwrap();
        let file_info = write_file(dir.path(), "empty.bin", b"");
        assert!(hash_file(&file_info.path).is_ok());
    }

    #[test]
    fn errors_on_nonexistent_file() {
        let path = PathBuf::from("/definitely/does/not/exist/abc123.bin");
        assert!(hash_file(&path).is_err());
    }

    #[test]
    fn hashing_does_not_modify_the_file() {
        let dir = tempdir().unwrap();
        let content = pattern(0x33, 50 * 1024);
        let file_info = write_file(dir.path(), "untouched.bin", &content);
        let before = std::fs::read(&file_info.path).unwrap();

        let _ = hash_file(&file_info.path).unwrap();

        let after = std::fs::read(&file_info.path).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn hashing_is_deterministic() {
        let dir = tempdir().unwrap();
        let content = pattern(0x77, 10 * 1024);
        let file_info = write_file(dir.path(), "stable.bin", &content);
        assert_eq!(
            hash_file(&file_info.path).unwrap(),
            hash_file(&file_info.path).unwrap()
        );
    }

    #[test]
    #[ignore = "slow: allocates a multi-GB sparse file"]
    fn hashes_multi_gigabyte_file_without_panicking() {
        let dir = tempdir().unwrap();
        let path = sparse_file(dir.path(), "huge.bin", 4 * 1024 * 1024 * 1024);
        assert!(hash_file(&path).is_ok());
    }

    // ====================================================================
    // 2. find_candidates
    // ====================================================================

    #[test]
    fn empty_input_returns_empty_map() {
        let files: Vec<FileInfo> = vec![];
        let result = find_candidates(&files).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn all_unique_files_produce_no_candidates() {
        let dir = tempdir().unwrap();
        let files = vec![
            write_file(dir.path(), "a.bin", b"content A"),
            write_file(dir.path(), "b.bin", b"content B, different"),
            write_file(dir.path(), "c.bin", b"content C, also different"),
        ];
        let result = find_candidates(&files).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn duplicate_pair_is_grouped_and_isolated_from_uniques() {
        let dir = tempdir().unwrap();
        let content = pattern(0x55, 10 * 1024);
        let a = write_file(dir.path(), "a.bin", &content);
        let b = write_file(dir.path(), "b.bin", &content);
        let c = write_file(dir.path(), "c.bin", b"totally different, unique");
        let files = vec![a.clone(), b.clone(), c];

        let result = find_candidates(&files).unwrap();

        assert_eq!(result.len(), 1);
        let group = result.iter().next().unwrap();
        let mut names: Vec<_> = group
            .files
            .iter()
            .map(|p| p.path.file_name().unwrap())
            .collect();
        names.sort();
        let mut expected = vec![a.path.file_name().unwrap(), b.path.file_name().unwrap()];
        expected.sort();
        assert_eq!(names, expected);
    }

    #[test]
    fn group_of_five_identical_files() {
        let dir = tempdir().unwrap();
        let content = pattern(0xAA, 5 * 1024);
        let files: Vec<FileInfo> = (0..5)
            .map(|i| write_file(dir.path(), &format!("dup_{i}.bin"), &content))
            .collect();

        let result = find_candidates(&files).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result.iter().next().unwrap().files.len(), 5);
    }

    #[test]
    fn two_independent_duplicate_groups_stay_separate() {
        let dir = tempdir().unwrap();
        let content1 = pattern(0x11, 4096);
        let content2 = pattern(0x22, 8192); // different size -> different fingerprint
        let a1 = write_file(dir.path(), "a1.bin", &content1);
        let a2 = write_file(dir.path(), "a2.bin", &content1);
        let b1 = write_file(dir.path(), "b1.bin", &content2);
        let b2 = write_file(dir.path(), "b2.bin", &content2);
        let files = vec![a1, a2, b1, b2];

        let result = find_candidates(&files).unwrap();
        assert_eq!(result.len(), 2);
        for group in result {
            assert_eq!(group.files.len(), 2);
        }
    }

    #[test]
    fn same_path_listed_twice_is_its_own_group() {
        // Documents current behavior rather than necessarily "correct"
        // behavior: if the same path appears twice in the input, it's
        // fingerprinted twice and both entries land in the same group,
        // since it's trivially identical to itself.
        let dir = tempdir().unwrap();
        let path = write_file(dir.path(), "only.bin", b"just one file");
        let files = vec![path.clone(), path.clone()];

        let result = find_candidates(&files).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result.iter().next().unwrap().files.len(), 2);
    }

    #[test]
    #[ignore = "enable once find_candidates propagates errors instead of unwrapping"]
    fn errors_gracefully_when_a_file_is_missing() {
        let dir = tempdir().unwrap();
        let good = write_file(dir.path(), "good.bin", b"fine");
        let missing = dir.path().join("does_not_exist.bin");
        let files = vec![good, missing.as_path().into()];

        let result = find_candidates(&files);
        assert!(matches!(result, Err(DedupError::FileRead(_))));
    }

    // ====================================================================
    // 3. find_duplicates
    // ====================================================================

    #[test]
    fn empty_candidates_returns_empty_duplicates() {
        let candidates: Vec<CandidateGroup> = Vec::new();
        let result = find_duplicates(&candidates).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn single_member_groups_produce_no_duplicates() {
        let dir = tempdir().unwrap();
        let file_info = write_file(dir.path(), "solo.bin", b"only one");
        let fp = fingerprint_file(&file_info).unwrap();
        let mut candidates: Vec<CandidateGroup> = Vec::new();
        candidates.push(CandidateGroup {
            files: vec![&file_info],
            fingerprint: fp,
        });

        let result = find_duplicates(&candidates).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn true_duplicates_in_a_single_candidate_group_are_reported() {
        let dir = tempdir().unwrap();
        let content = pattern(0x66, 2048);
        let a = write_file(dir.path(), "a.bin", &content);
        let b = write_file(dir.path(), "b.bin", &content);
        let fp = fingerprint_file(&a).unwrap();
        let mut candidates: Vec<CandidateGroup> = Vec::new();
        candidates.push(CandidateGroup {
            files: vec![&a, &b],
            fingerprint: fp,
        });

        let result = find_duplicates(&candidates).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].files.len(), 2);
    }

    #[test]
    fn group_of_three_true_duplicates_stays_together() {
        let dir = tempdir().unwrap();
        let content = pattern(0x99, 2048);
        let a = write_file(dir.path(), "a.bin", &content);
        let b = write_file(dir.path(), "b.bin", &content);
        let c = write_file(dir.path(), "c.bin", &content);
        let fp = fingerprint_file(&a).unwrap();
        let mut candidates: Vec<CandidateGroup> = Vec::new();
        candidates.push(CandidateGroup {
            files: vec![&a, &b, &c],
            fingerprint: fp,
        });

        let result = find_duplicates(&candidates).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].files.len(), 3);
    }

    #[test]
    fn multiple_candidate_groups_are_all_processed() {
        let dir = tempdir().unwrap();
        let content1 = pattern(0x11, 1024);
        let content2 = pattern(0x22, 2048);
        let a1 = write_file(dir.path(), "a1.bin", &content1);
        let a2 = write_file(dir.path(), "a2.bin", &content1);
        let b1 = write_file(dir.path(), "b1.bin", &content2);
        let b2 = write_file(dir.path(), "b2.bin", &content2);

        let fp_a = fingerprint_file(&a1).unwrap();
        let fp_b = fingerprint_file(&b1).unwrap();
        let mut candidates: Vec<CandidateGroup> = Vec::new();
        candidates.push(CandidateGroup {
            files: vec![&a1, &a2],
            fingerprint: fp_a,
        });
        candidates.push(CandidateGroup {
            files: vec![&b1, &b2],
            fingerprint: fp_b,
        });

        let result = find_duplicates(&candidates).unwrap();
        let normalized = normalize_groups(
            result
                .iter()
                .map(|group| group.files.iter().copied().collect())
                .collect(),
        );
        let expected = normalize_groups(vec![vec![&a1, &a2], vec![&b1, &b2]]);

        assert_eq!(normalized, expected);
    }

    #[test]
    fn splits_apart_a_fingerprint_false_positive() {
        // The key correctness check for the two-stage design: a
        // candidate group that only matches because of
        // fingerprint_file's sampling must be split apart here, since
        // this function checks the FULL file content via hash_file.
        let dir = tempdir().unwrap();
        let size = 400 * 1024;
        let mut a = pattern(0x66, size);
        let mut b = pattern(0x66, size);
        let gap = CHUNK_SIZE as usize + 500; // outside START/MIDDLE/END
        a[gap] = 0x01;
        b[gap] = 0x02;
        let path_a = write_file(dir.path(), "a.bin", &a);
        let path_b = write_file(dir.path(), "b.bin", &b);

        let fp_a = fingerprint_file(&path_a).unwrap();
        // let fp_b = fingerprint_file(&path_b).unwrap();
        // assert_eq!(
        //     fp_a.partial_hash, fp_b.partial_hash,
        //     "test setup assumption broken: expected a fingerprint collision here"
        // );

        let mut candidates: Vec<CandidateGroup> = Vec::new();
        candidates.push(CandidateGroup {
            files: vec![&path_a, &path_b],
            fingerprint: fp_a,
        });

        let result = find_duplicates(&candidates).unwrap();
        assert!(
            result.is_empty(),
            "false-positive candidates should be split apart, not reported as duplicates"
        );
    }

    #[test]
    fn propagates_error_when_a_candidate_file_vanishes_before_hashing() {
        // Contrast with find_candidates's `.unwrap()` bug: find_duplicates
        // itself does the right thing with `hash_file(...)?`. If a file
        // that was present when the candidates map was built disappears
        // before find_duplicates re-reads it (a realistic race between
        // the two phases), the error propagates as Err(_), not a panic.
        let dir = tempdir().unwrap();
        let content = pattern(0x22, 5 * 1024);
        let a = write_file(dir.path(), "a.bin", &content);
        let b = write_file(dir.path(), "b.bin", &content);
        let fp = fingerprint_file(&a).unwrap();
        let mut candidates: Vec<CandidateGroup> = Vec::new();
        candidates.push(CandidateGroup {
            files: vec![&a, &b],
            fingerprint: fp,
        });

        std::fs::remove_file(&a.path).unwrap();

        let result = find_duplicates(&candidates);
        assert!(result.is_err());
    }

    // ====================================================================
    // 4. Integration: find_candidates -> find_duplicates
    // ====================================================================

    #[test]
    fn integration_genuine_duplicates_survive_both_stages() {
        let dir = tempdir().unwrap();
        let content = pattern(0x77, 400 * 1024);
        let a = write_file(dir.path(), "a.bin", &content);
        let b = write_file(dir.path(), "b.bin", &content);
        let files = vec![a, b];

        let candidates = find_candidates(&files).unwrap();
        let duplicates = find_duplicates(&candidates).unwrap();

        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].files.len(), 2);
    }

    #[test]
    fn integration_fingerprint_false_positive_ends_up_empty() {
        let dir = tempdir().unwrap();
        let size = 400 * 1024;
        let mut a = pattern(0x66, size);
        let mut b = pattern(0x66, size);
        let gap = CHUNK_SIZE as usize + 500;
        a[gap] = 0x01;
        b[gap] = 0x02;
        let files = vec![
            write_file(dir.path(), "a.bin", &a),
            write_file(dir.path(), "b.bin", &b),
        ];

        let candidates = find_candidates(&files).unwrap();
        assert_eq!(
            candidates.len(),
            1,
            "should still collide at the cheap fingerprint stage"
        );

        let duplicates = find_duplicates(&candidates).unwrap();
        assert!(
            duplicates.is_empty(),
            "full-content hash should have told them apart"
        );
    }

    #[test]
    fn integration_mixed_batch_finds_only_genuine_duplicates() {
        let dir = tempdir().unwrap();

        let unique1 = write_file(dir.path(), "unique1.bin", b"nothing else looks like this");
        let unique2 = write_file(dir.path(), "unique2.bin", &pattern(0x9A, 20 * 1024));

        let small_dup_content = pattern(0x44, 3 * 1024);
        let small_a = write_file(dir.path(), "small_a.bin", &small_dup_content);
        let small_b = write_file(dir.path(), "small_b.bin", &small_dup_content);

        let large_dup_content = pattern(0x55, 400 * 1024);
        let large_a = write_file(dir.path(), "large_a.bin", &large_dup_content);
        let large_b = write_file(dir.path(), "large_b.bin", &large_dup_content);

        let size = 400 * 1024;
        let mut fake_a = pattern(0x66, size);
        let mut fake_b = pattern(0x66, size);
        let gap = CHUNK_SIZE as usize + 500;
        fake_a[gap] = 0x01;
        fake_b[gap] = 0x02;
        let fake_a_path = write_file(dir.path(), "fake_a.bin", &fake_a);
        let fake_b_path = write_file(dir.path(), "fake_b.bin", &fake_b);

        let files = vec![
            unique1,
            unique2,
            small_a.clone(),
            small_b.clone(),
            large_a.clone(),
            large_b.clone(),
            fake_a_path,
            fake_b_path,
        ];

        let candidates = find_candidates(&files).unwrap();
        let duplicates = find_duplicates(&candidates).unwrap();
        let normalized = normalize_groups(
            duplicates
                .iter()
                .map(|group| group.files.iter().copied().collect())
                .collect(),
        );
        let expected = normalize_groups(vec![vec![&small_a, &small_b], vec![&large_a, &large_b]]);

        assert_eq!(normalized, expected);
    }
}
