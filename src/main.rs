mod command;
mod list;
mod options;
mod run;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Result;
use clap::Parser;

use crate::{
    list::find_repositories,
    options::{Args, Subcommands},
};

fn main() -> Result<()> {
    let args: Args = Args::parse_from(gix::env::args_os());

    #[allow(unsafe_code)]
    unsafe {
        // SAFETY: we don't manipulate the environment from any thread
        time::util::local_offset::set_soundness(time::util::local_offset::Soundness::Unsound);
    }
    let should_interrupt = Arc::new(AtomicBool::new(false));
    gix::interrupt::init_handler({
        let should_interrupt = Arc::clone(&should_interrupt);
        move || should_interrupt.store(true, Ordering::SeqCst)
    })?;

    let verbose = !args.quiet;
    let progress = args.progress;
    let threads = args.threads;
    let progress_keep_open = args.progress_keep_open;

    let root = args
        .root
        .unwrap_or_else(|| [std::path::Component::CurDir].iter().collect());

    match args.cmd {
        Subcommands::List => run::prepare_and_run(
            "list",
            verbose,
            progress,
            progress_keep_open,
            run::STANDARD_RANGE,
            |progress, out, _err| {
                for repo in find_repositories(root, progress)? {
                    writeln!(out, "{}", repo.path.display())?;
                }

                Ok(())
            },
        ),
    }
}

pub type ProgressRange = std::ops::RangeInclusive<prodash::progress::key::Level>;
