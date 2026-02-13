mod command;
mod options;

use std::{
    io::{self, stdout, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use clap::Parser;
use exn::{ErrorExt, Result};
use gitoxide_core::organize::find_git_repository_workdirs;
use gix::{progress::DoOrDiscard, repository::Kind};

use crate::options::Args;

#[derive(Debug, thiserror::Error)]
pub enum GallError {
    #[error("No git repositories found in {0}")]
    NoRepositoriesFound(PathBuf),
    #[error("Failed to initialize interrupt handler")]
    InterruptHandlerInit(#[source] io::Error),
    #[error("IO error")]
    Io(#[from] io::Error),
}

fn main() -> Result<(), GallError> {
    let args: Args = Args::parse_from(gix::env::args_os());

    let should_interrupt = Arc::new(AtomicBool::new(false));
    #[allow(unsafe_code)]
    unsafe {
        // SAFETY: The closure doesn't use mutexes or memory allocation, so it should be safe to call from a signal handler.
        gix::interrupt::init_handler(1, {
            let should_interrupt = Arc::clone(&should_interrupt);
            move || should_interrupt.store(true, Ordering::SeqCst)
        })
        .map_err(|e| GallError::InterruptHandlerInit(e).raise())?;
    }

    let source_dir = args
        .root
        .clone()
        .unwrap_or_else(|| [std::path::Component::CurDir].iter().collect());

    let progress = prodash::tree::Root::new();
    let find_progress = progress.add_child("Discover Git repositories");

    let _renderer = prodash::render::line(
        stdout(),
        std::sync::Arc::downgrade(&progress),
        prodash::render::line::Options::default()
            .auto_configure(prodash::render::line::StreamKind::Stdout),
    );

    let repositories: Vec<(PathBuf, Kind)> = find_git_repository_workdirs(
        &source_dir,
        DoOrDiscard::from(Some(find_progress)),
        false,
        None,
    )
    .collect();

    if repositories.is_empty() {
        return Err(GallError::NoRepositoriesFound(source_dir).raise());
    }

    match args.cmd {
        options::Subcommands::List => {
            let mut out = io::stdout().lock();
            for r in repositories {
                out.write_fmt(format_args!("{}\n", r.0.display()))
                    .map_err(|e| GallError::Io(e).raise())?;
            }
        }
    };

    Ok(())
}
