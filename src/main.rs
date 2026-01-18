mod command;
mod list;
mod options;

use std::{
    io::{self, stdout, IsTerminal, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::{ensure, Result};
use clap::Parser;
use gix::progress::DoOrDiscard;
use list::find_repositories;

use crate::options::Args;

fn main() -> Result<()> {
    let args: Args = Args::parse_from(gix::env::args_os());

    let should_interrupt = Arc::new(AtomicBool::new(false));
    #[allow(unsafe_code)]
    unsafe {
        // SAFETY: The closure doesn't use mutexes or memory allocation, so it should be safe to call from a signal handler.
        gix::interrupt::init_handler(1, {
            let should_interrupt = Arc::clone(&should_interrupt);
            move || should_interrupt.store(true, Ordering::SeqCst)
        })?;
    }

    let verbose = !args.quiet;
    let progress = args.progress;
    let threads = args.threads;

    let source_dir = args
        .root
        .unwrap_or_else(|| [std::path::Component::CurDir].iter().collect());

    let progress = prodash::tree::Root::new();
    let find_progress = progress.add_child("Discover Git repositories");

    let output_is_terminal = stdout().is_terminal();
    let line_ui = prodash::render::line(
        stdout(),
        std::sync::Arc::downgrade(&progress),
        prodash::render::line::Options {
            output_is_terminal,
            colored: output_is_terminal && true,
            ..Default::default()
        },
    );

    let repositories = find_repositories(&source_dir, DoOrDiscard::from(Some(find_progress)))?;

    ensure!(
        !repositories.is_empty(),
        "No git repositories found in {}",
        source_dir.display()
    );

    match args.cmd {
        options::Subcommands::List => {
            let mut out = io::stdout().lock();
            for r in repositories {
                out.write_fmt(format_args!("{}\n", r.0.display()))?;
            }
        }
    };

    Ok(())
}
