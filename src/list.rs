use std::path::{Path, PathBuf};

use anyhow::Result;
use gitoxide_core::organize::find_git_repository_workdirs;
use gix::{progress::DoOrDiscard, repository::Kind};

pub(crate) fn find_repositories(
    source_dir: &Path,
    progress: DoOrDiscard<prodash::tree::Item>,
) -> Result<Vec<(PathBuf, Kind)>> {
    let git_repository_workdirs = find_git_repository_workdirs(source_dir, progress, false, None);

    return Ok(git_repository_workdirs.collect::<Vec<(PathBuf, Kind)>>());
}
