use std::{fs, path::Path};

use crate::cli::{Action, Options};
use crate::duplicates;
use crate::error::DedupError;
use crate::files;

/// By default, the last modified version of the file is kept
pub fn choose_files_to_delete<'a>(files: &[Vec<&'a Path>]) -> Result<Vec<&'a Path>, DedupError> {
    let mut files_to_remove = Vec::new();
    for group in files {
        let last_modified = group
            .iter()
            .filter_map(|path| {
                let modified = fs::metadata(path).ok()?.modified().ok()?;
                Some((path, modified))
            })
            .max_by_key(|(path, modified)| (*modified, *path))
            .map(|(path, _)| path);
        if let Some(last_modified) = last_modified {
            files_to_remove.extend(group.iter().copied().filter(|file| file != last_modified));
        }
    }
    Ok(files_to_remove)
}

pub fn delete(files: &[&Path]) -> Result<(), DedupError> {
    for file in files {
        // fs::remove_file(file)?;
        println!("Removing '{}'", file.display());
    }

    Ok(())
}

fn dry_run(files: &[&Path]) {
    println!("Dry run — no files will be deleted.");
    println!();

    if files.is_empty() {
        println!("No duplicate files found.");
        return;
    }

    println!("Files that would be deleted:");

    for file in files {
        println!("  {}", file.display());
    }

    println!();
    println!("{} file(s) would be deleted.", files.len());
}

pub fn run(options: Options) -> Result<(), DedupError> {
    let files = files::find_files(&options.root_dir)?;
    let candidates = duplicates::find_candidates(&files)?;
    let duplicates = duplicates::find_duplicates(&candidates)?;
    let files_to_delete = choose_files_to_delete(&duplicates)?;

    match options.action {
        Action::DryRun => dry_run(&files_to_delete),
        Action::Delete => delete(&files_to_delete)?,
    }
    Ok(())
}
