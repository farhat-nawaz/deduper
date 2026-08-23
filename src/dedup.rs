use crate::cli::{Action, KeepPolicy, Options};
use crate::duplicates::{DuplicateGroup, find_candidates, find_duplicates};
use crate::error::DedupError;
use crate::files::{FileInfo, find_files};

struct DeletionGroup<'a> {
    keeper: &'a FileInfo,
    to_delete: Vec<&'a FileInfo>,
}

impl<'a> DeletionGroup<'a> {
    fn reclaimable_size(&self) -> u64 {
        self.to_delete.iter().map(|file| file.size).sum()
    }
}

struct DeletionPlan<'a> {
    groups: Vec<DeletionGroup<'a>>,
}

pub fn run(options: Options) -> Result<(), DedupError> {
    let files = find_files(&options.root_dir, &options.exclude)?;
    let candidates = find_candidates(&files)?;
    let duplicates = find_duplicates(&candidates)?;
    let deletion_plan = plan_deletion(&duplicates, options.keep);

    report_stats(&deletion_plan);

    if options.delete {
        execute_deletion_plan(&deletion_plan)?;
    }

    Ok(())
}

fn report_stats(plan: &DeletionPlan) {
    let mut total_deletable_files = 0_usize;
    let mut reclaimable_bytes = 0_u64;

    println!();
    for group in &plan.groups {
        println!("Duplicate Group:");
        println!("  KEEP    {}", group.keeper.path.display());

        for file in &group.to_delete {
            println!("  DELETE  {}", file.path.display());
        }

        let group_reclaimable_bytes = group.reclaimable_size();
        reclaimable_bytes += group_reclaimable_bytes;
        total_deletable_files += group.to_delete.len();

        println!();
        println!(
            "  {} file(s), {} reclaimable",
            group.to_delete.len(),
            format_size(group_reclaimable_bytes)
        );
        println!();
    }

    println!("SUMMARY:");
    println!("  Duplicate Groups:  {}", plan.groups.len());
    println!("  Files to delete:   {}", total_deletable_files);
    println!("  Reclaimable Space: {}", format_size(reclaimable_bytes));
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];

    let mut size = bytes as f64;
    let mut unit = 0;

    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    format!("{:.2} {}", size, UNITS[unit])
}

fn plan_deletion<'a>(duplicates: &[DuplicateGroup<'a>], policy: KeepPolicy) -> DeletionPlan<'a> {
    let mut groups = Vec::new();
    for group in duplicates {
        // TODO: remove unwrap
        let keeper = choose_keeper(group, policy);
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

fn choose_keeper<'a>(group: &DuplicateGroup<'a>, policy: KeepPolicy) -> &'a FileInfo {
    match policy {
        KeepPolicy::Newest => group
            .files
            .iter()
            .max_by_key(|file| (file.modified, &file.path))
            .copied()
            .expect("duplicate group must contain at least one file"),
        KeepPolicy::Oldest => group
            .files
            .iter()
            .min_by_key(|file| (file.modified, &file.path))
            .copied()
            .expect("duplicate group must contain at least one file"),
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
