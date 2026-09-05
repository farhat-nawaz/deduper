# deduper

A fast, content-based file deduplication CLI written in Rust. Point it at a
directory, and it finds files that are byte-for-byte identical — not just
same name or same size — and tells you what it would remove to reclaim the
space.

> ⚠️ **Project status:** this is an active work in progress. Scanning,
> fingerprinting, hashing, and reporting are fully implemented. The `--delete`
> flag is parsed and the deletion *plan* is built and re-verified, but the
> actual filesystem removal call has not been wired in yet — running with
> `--delete` today will print what it *would* remove without touching any
> files. Treat every run as a dry run until this note is removed.

## Features

- **Content-based, not name-based** — duplicates are found by hashing file
  contents, so renamed or relocated copies are still caught.
- **Two-stage detection** — a cheap sampled fingerprint groups candidates
  first; only files that collide on the cheap check get a full byte-for-byte
  hash, so you don't pay full I/O cost for every file up front.
- **Configurable keep policy** — keep the newest or oldest file in each
  duplicate group.
- **Filtering** — exclude by extension, exclude by directory name, restrict
  by min/max file size, opt in to hidden files.
- **Safe by design** — dry-run unless `--delete` is passed, and each file's
  identity is re-verified immediately before removal so a file that changed
  mid-run is never deleted based on stale information.

## How it works

```
walk directory tree
      │  (skip hidden / excluded dirs / excluded extensions / out-of-range sizes)
      ▼
fingerprint every remaining file
      │  size + BLAKE3 hash of the first/middle/last 64 KiB
      │  (whole file if it's smaller than 192 KiB)
      ▼
group files that share a fingerprint  ── these are *candidates*, not confirmed duplicates
      ▼
fully hash every file in a candidate group (BLAKE3, entire contents)
      ▼
group files that share a full hash  ── these are *confirmed* duplicates
      ▼
pick a "keeper" per group (newest or oldest)
      ▼
report the plan  →  (optionally, in future) delete everything else
```

The fingerprint is intentionally cheap: for large files it only reads
192 KiB total instead of the whole file, so it's fast at ruling out files
that clearly aren't duplicates. Because it's a sample rather than the full
content, it can theoretically produce false-positive candidates — two large
files that happen to agree on their sampled regions but differ elsewhere —
which is exactly why the second, full-hash stage exists before anything is
ever reported as a true duplicate.

## Requirements

- **Rust 1.82+** (uses `Option::is_none_or`, stabilized in that release)
- **Unix-like OS** (Linux, macOS, BSD). File identity is tracked via device +
  inode numbers, which isn't available in the same form on Windows — the
  crate will refuse to compile there for now.

## Installation

```bash
git clone https://github.com/farhat-nawaz/deduper/pull/1
cd deduper
cargo build --release
```

The compiled binary will be at `target/release/dedup`. You can also install
it onto your `$PATH`:

```bash
cargo install --path .
```

## Usage

```
dedup <ROOT_DIR> [OPTIONS]
```

| Flag | Description | Default |
|---|---|---|
| `<ROOT_DIR>` | Directory to scan (positional, required) | — |
| `--delete` | Actually remove duplicates (see status note above) | off (dry run) |
| `--include-hidden-files` | Include dotfiles and dot-directories | off |
| `--keep <newest\|oldest>` | Which file in a duplicate group to keep | `newest` |
| `--exclude-ext <a,b,c>` | Comma-separated list of extensions to skip | none |
| `--exclude-dir <a,b,c>` | Comma-separated list of directory names to skip | none |
| `--max-file-size <SIZE>` | Skip files larger than `SIZE` | none |
| `--min-file-size <SIZE>` | Skip files smaller than `SIZE` | none |

**Size format:** an integer immediately followed by a unit — `B`, `KB`/`KiB`,
`MB`/`MiB`, or `GB`/`GiB` (case-insensitive). A unit is required; bare
numbers like `500` are rejected. Examples: `500MB`, `2GiB`, `100kb`.

### Examples

```bash
# Dry run: scan ~/Downloads, prefer keeping the oldest copy in each group
dedup ~/Downloads --keep oldest

# Skip common build/VCS directories
dedup ~/projects --exclude-dir node_modules,target,.git

# Only look at files between 1 MB and 2 GB, including hidden files
dedup ~/backups --include-hidden-files --min-file-size 1MB --max-file-size 2GB

# Ignore video files and disk images entirely
dedup ~/media --exclude-ext mp4,iso
```

### Sample output

```
Duplicate Group:
  KEEP    /photos/2024/vacation.jpg
  DELETE  /backup/vacation.jpg
  DELETE  /old/vacation.jpg

  2 file(s), 8.40 MiB reclaimable

Duplicate Group:
  KEEP    /documents/report.pdf
  DELETE  /downloads/report.pdf

  1 file(s), 2.10 MiB reclaimable

SUMMARY:
  Duplicate Groups:  2
  Files to delete:   3
  Reclaimable Space: 10.50 MiB
```

## Safety notes

- Nothing is removed from disk unless `--delete` is passed (and, currently,
  not even then — see the status note).
- Immediately before removing a file, its device/inode identity, size, and
  modification time are re-checked against what was recorded during the
  scan. If anything has changed — the file was edited, replaced, or moved —
  deletion is aborted rather than proceeding on stale information.
- Named pipes (FIFOs) and other special files are not filtered out before
  fingerprinting; pointing the scanner at one may hang rather than error.
  Avoid scanning directories known to contain them until this is addressed.
- Symbolic links are currently skipped entirely during the directory walk —
  they're neither followed nor reported as duplicates.

## Known limitations / roadmap

- [ ] Actual file deletion (`fs::remove_file`) is not yet wired up —
      `--delete` currently only plans and reports.
- [ ] Symlinks are not followed (`--follow-symlinks`).
- [ ] No confirmation prompt before deletion (`-y`/`--yes` to skip it).
- [ ] Additional keep policies: shortest path, longest path.

## Development

```bash
# run the fast test suite
cargo test

# include slow/ignored tests (multi-gigabyte files, timing-sensitive races)
cargo test -- --ignored
```

### Module layout

| File | Responsibility |
|---|---|
| `main.rs` | Entry point; Unix-only guard; wires CLI parsing to `dedup::run`. |
| `cli.rs` | Argument definitions (`clap`) and the `FileSize` unit parser. |
| `files.rs` | Directory walking, filtering, and `FileInfo`/identity tracking. |
| `fingerprint.rs` | Cheap sampled BLAKE3 fingerprint used as a candidate key. |
| `duplicates.rs` | Groups candidates by fingerprint, then confirms with a full hash. |
| `dedup.rs` | Orchestration, deletion planning, reporting, and (eventually) execution. |
| `error.rs` | The crate's error type (`thiserror`), one variant per failure mode. |

### Built with

[`clap`](https://crates.io/crates/clap) · [`walkdir`](https://crates.io/crates/walkdir) ·
[`blake3`](https://crates.io/crates/blake3) · [`thiserror`](https://crates.io/crates/thiserror) ·
[`anyhow`](https://crates.io/crates/anyhow) · [`tempfile`](https://crates.io/crates/tempfile) (dev-dependency)

## Contributing

Issues and PRs are welcome. If you're fixing a bug, a regression test that
would have caught it is the fastest way to get a review turned around.

## License

_No license has been added yet — until one is, all rights are reserved by
default. Add a `LICENSE` file (MIT or Apache-2.0 are the common choices for
Rust CLI tools) before treating this as open source._
