use crate::cli::{Action, Options};
use crate::duplicates;
use crate::error::DedupError;
use crate::files::{self, FileInfo};

/// By default, the last modified version of the file is kept
pub fn choose_files_to_delete<'a>(
    files: &[Vec<&'a files::FileInfo>],
) -> Result<Vec<&'a files::FileInfo>, DedupError> {
    let mut files_to_remove = Vec::new();
    for group in files {
        let last_modified = group
            .iter()
            .filter_map(|file| Some((&file.path, file.modified)))
            .max_by_key(|(path, modified)| (*modified, *path))
            .map(|(path, _)| path);
        if let Some(last_modified) = last_modified {
            files_to_remove.extend(
                group
                    .iter()
                    .copied()
                    .filter(|file| &file.path != last_modified),
            );
        }
    }
    Ok(files_to_remove)
}

pub fn delete(files: &[&FileInfo]) -> Result<(), DedupError> {
    for file in files {
        // fs::remove_file(file)?;
        println!("Removing '{}'", file.path.display());
    }

    Ok(())
}

fn dry_run(files: &[&FileInfo]) {
    println!("Dry run — no files will be deleted.");
    println!();

    if files.is_empty() {
        println!("No duplicate files found.");
        return;
    }

    println!("Files that would be deleted:");

    for file in files {
        println!("  {}", file.path.display());
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
