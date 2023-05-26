use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::Result;
use gix::Kind;
use gix::{progress, Progress};

/// Simple structure pointing at a Git repository location on disk.
///
/// This struct holds path to the repository on the file system and kind of a repository
/// (that is, whether this is a bare repository, a worktree or a submodule)
#[derive(Debug)]
pub(crate) struct RepoPath {
    pub(crate) path: PathBuf,
    pub(crate) _kind: Kind,
}

impl From<(PathBuf, Kind)> for RepoPath {
    fn from(value: (PathBuf, Kind)) -> Self {
        let (path, _kind) = value;
        RepoPath { path, _kind }
    }
}

/// Returns a list of Paths found within the given `root` path.
///
/// This will recursively search the whole file system tree under the given
/// root path and return a list of paths to the directories that contain a Git repository.
pub(crate) fn find_repositories<P: Progress>(
    source_dir: &impl AsRef<Path>,
    mut progress: P,
) -> Result<Vec<RepoPath>>
where
    <P::SubProgress as Progress>::SubProgress: Sync,
{
    let mut _num_errors = 0usize;
    let it = find_git_repository_workdirs(source_dir, progress.add_child("Searching repositories"));
    Ok(it.map(|(path, _kind)| RepoPath { path, _kind }).collect())
}

fn find_git_repository_workdirs<P: Progress>(
    root: impl AsRef<Path>,
    mut progress: P,
) -> impl Iterator<Item = (PathBuf, Kind)>
where
    P::SubProgress: Sync,
{
    progress.init(
        None,
        progress::count("scanning filesystem for git repositories"),
    );

    fn is_repository(path: &Path) -> Option<Kind> {
        // Can be git dir or worktree checkout (file)
        if path.file_name() != Some(OsStr::new(".git")) {
            return None;
        }

        if path.is_dir() {
            if path.join("HEAD").is_file() && path.join("config").is_file() {
                gix::discover::is_git(path).ok().map(Into::into)
            } else {
                None
            }
        } else {
            // git files are always worktrees
            Some(Kind::WorkTree { is_linked: true })
        }
    }
    fn into_workdir(git_dir: PathBuf) -> PathBuf {
        if gix::discover::is_bare(&git_dir) {
            git_dir
        } else {
            git_dir
                .parent()
                .expect("git is never in the root")
                .to_owned()
        }
    }

    #[derive(Debug, Default)]
    struct State {
        kind: Option<Kind>,
    }

    let walk = jwalk::WalkDirGeneric::<((), State)>::new(root)
        .follow_links(false)
        .sort(true)
        .skip_hidden(false);

    // On macos with apple silicon, the IO subsystem is entirely different and one thread can mostly max it out.
    // Thus using more threads just burns energy unnecessarily.
    // It's notable that `du` is very fast even on a single core and more power efficient than dua with a single core.
    // The default of '4' seems related to the amount of performance cores present in the system.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let walk = walk.parallelism(jwalk::Parallelism::RayonNewPool(4));

    walk.process_read_dir(move |_depth, _path, _read_dir_state, siblings| {
        let mut found_any_repo = false;
        let mut found_bare_repo = false;
        for entry in siblings.iter_mut().flatten() {
            let path = entry.path();
            if let Some(kind) = is_repository(&path) {
                let is_bare = kind.is_bare();
                entry.client_state = State { kind: kind.into() };
                entry.read_children_path = None;

                found_any_repo = true;
                found_bare_repo = is_bare;
            }
        }
        // Only return paths which are repositories are further participating in the traversal
        // Don't let bare repositories cause siblings to be pruned.
        if found_any_repo && !found_bare_repo {
            siblings.retain(|e| {
                e.as_ref()
                    .map(|e| e.client_state.kind.is_some())
                    .unwrap_or(false)
            });
        }
    })
    .into_iter()
    .inspect(move |_| progress.inc())
    .filter_map(Result::ok)
    .filter_map(|mut e| {
        e.client_state
            .kind
            .take()
            .map(|kind| (into_workdir(e.path()), kind))
    })
}
