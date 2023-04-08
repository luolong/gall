use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[clap(about = "Manage your git repos with gall.", version = clap::crate_version!())]
#[clap(subcommand_required = true)]
pub(crate) struct Args {
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

    #[clap(short = 'C')]
    /// Run as if gall was started in <ROOT> instead of the current working directory.
    ///
    /// Defaults to the current working directory.
    pub(crate) root: Option<PathBuf>,

    #[clap(subcommand)]
    pub(crate) cmd: Subcommands,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Subcommands {
    /// Find and list all git repositories in current workspace root
    #[clap(alias = "ls")]
    List,
}
