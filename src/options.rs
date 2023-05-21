use std::{convert::Infallible, path::PathBuf};

use shellexpand::tilde;

#[derive(Debug, clap::Parser)]
#[clap(about = "Manage your git repos with gall.", version = clap::crate_version!())]
#[clap(subcommand_required = true)]
pub(crate) struct Args {
    #[clap(flatten)]
    pub(crate) global: GlobalOptions,

    #[clap(subcommand)]
    pub(crate) cmd: Subcommands,
}

/// Global command line options
#[derive(Debug, clap::Parser)]
pub(crate) struct GlobalOptions {
    /// Do not display verbose messages and progress information
    #[clap(long, short = 'q')]
    pub(crate) quiet: bool,

    /// Bring up a terminal user interface displaying progress visually
    #[clap(long, conflicts_with("quiet"))]
    pub(crate) progress: bool,

    /// The amount of threads to use. If unset, use all cores, if 0 use al physical cores.
    #[clap(short = 't', long)]
    pub(crate) threads: Option<usize>,

    /// The progress TUI will stay up even though the work is already completed.
    ///
    /// Use this to be able to read progress messages or additional information visible in the TUI log pane.
    #[clap(long, conflicts_with("quiet"), requires("progress"))]
    pub(crate) progress_keep_open: bool,

    /// Run as if gall was started in <ROOT> instead of the current working directory.
    ///
    /// Defaults to the current working directory.
    #[clap(short = 'C', value_parser = parse_path)]
    pub(crate) root: Option<PathBuf>,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Subcommands {
    /// Find and list all git repositories in current workspace root
    #[clap(alias = "ls")]
    List,
}

impl std::fmt::Display for Subcommands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Subcommands::List => write!(f, "list"),
        }
    }
}

fn parse_path(value: &str) -> Result<PathBuf, Infallible> {
    Ok(PathBuf::from(tilde(value).to_string()))
}
