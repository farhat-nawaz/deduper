use crate::cli::{Action, KeepPolicy, Options};
use crate::duplicates::{DuplicateGroup, find_candidates, find_duplicates};
use crate::error::DedupError;
use crate::files::{self, FileInfo};

struct DeletionGroup<'a> {
    keeper: &'a FileInfo,
    to_delete: Vec<&'a FileInfo>,
}

struct DeletionPlan<'a> {
    groups: Vec<DeletionGroup<'a>>,
}

pub fn run(options: Options) -> Result<(), DedupError> {
    let files = files::find_files(&options.root_dir)?;
    let candidates = find_candidates(&files)?;
    let duplicates = find_duplicates(&candidates)?;
    let deletion_plan = plan_deletion(&duplicates, options.keep);

    match options.action {
        Action::DryRun => dry_run(&deletion_plan),
        Action::Delete => execute_deletion_plan(&deletion_plan)?,
    }
    Ok(())
}

fn dry_run(plan: &DeletionPlan) {
    println!("Dry run — no files will be deleted.");
    println!();
    for group in &plan.groups {
        println!("Group Keeper: '{}'", group.keeper.path.display());
        // this check is probably redundant. each duplicate group is guaranteed to have at least
        // one file to delete
        if group.to_delete.is_empty() {
            println!("No duplicate files found.");
            continue;
        }

        println!("Files that would be deleted:");

        for file in &group.to_delete {
            println!("  {}", file.path.display());
        }

        println!();
        println!("{} file(s) would be deleted.", group.to_delete.len());
        println!();
        println!();
    }
}

fn plan_deletion<'a>(duplicates: &[DuplicateGroup<'a>], policy: KeepPolicy) -> DeletionPlan<'a> {
    let mut groups = Vec::new();
    for group in duplicates {
        // TODO: remove unwrap
        let keeper = choose_keeper(group, policy).unwrap();
        let to_delete = group
            .files
            .iter()
            .filter(|file| file.path != keeper.path)
            .copied()
            .collect();

        groups.push(DeletionGroup { keeper, to_delete });
    }
    DeletionPlan { groups }
}

fn choose_keeper<'a>(group: &DuplicateGroup<'a>, policy: KeepPolicy) -> Option<&'a FileInfo> {
    match policy {
        KeepPolicy::Newest => group
            .files
            .iter()
            .max_by_key(|file| (file.modified, &file.path))
            .copied(),
        KeepPolicy::Oldest => group
            .files
            .iter()
            .min_by_key(|file| (file.modified, &file.path))
            .copied(),
    }
}

fn execute_deletion_plan(plan: &DeletionPlan) -> Result<(), DedupError> {
    for group in &plan.groups {
        println!("Group Keeper: '{}'", group.keeper.path.display());
        for file in &group.to_delete {
            println!("  Removing '{}'", file.path.display());
        }
    }
    Ok(())
}
