mod command;
mod list;
mod options;

use std::{
    io::{self, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::{ensure, Result};
use clap::Parser;
use list::find_repositories;

use crate::options::Args;
use gix::progress::{prodash, DoOrDiscard};

pub type ProgressRange = std::ops::RangeInclusive<prodash::progress::key::Level>;
pub const STANDARD_RANGE: ProgressRange = 2..=2;

fn main() -> Result<()> {
    let args: Args = Args::parse_from(gix::env::args_os());

    #[allow(unsafe_code)]
    unsafe {
        // SAFETY: we don't manipulate the environment from any thread
        time::util::local_offset::set_soundness(time::util::local_offset::Soundness::Unsound);
    }

    // Make Ctrl+C work
    let should_interrupt = Arc::new(AtomicBool::new(false));
    gix::interrupt::init_handler({
        let should_interrupt = Arc::clone(&should_interrupt);
        move || should_interrupt.store(true, Ordering::SeqCst)
    })?;

    let source_dir = args
        .global
        .root
        .unwrap_or_else(|| PathBuf::from(&std::path::Component::CurDir));

    let progress: std::sync::Arc<prodash::tree::Root> = prodash::tree::root::Options {
        message_buffer_capacity: 200,
        ..Default::default()
    }
    .into();
    let sub_progress = progress.add_child("Searching for git repositories");

    let repositories = find_repositories(&source_dir, DoOrDiscard::from(Some(sub_progress)))?;

    ensure!(
        !repositories.is_empty(),
        "No git repositories found in {}",
        source_dir.display()
    );

    match args.cmd {
        options::Subcommands::List => {
            let mut out = io::stdout().lock();
            for r in repositories {
                let mut path = r.path.display().to_string();
                if let Some(home_dir) = dirs::home_dir().map(|h| h.display().to_string()) {
                    if path.starts_with(&home_dir) {
                        path.replace_range(..home_dir.len(), "~")
                    }
                }

                out.write_fmt(format_args!("{}\n", path))?;
            }
        }
    };

    Ok(())
}
