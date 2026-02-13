use std::{convert::Infallible, path::PathBuf};

use shellexpand::tilde;

#[derive(Debug, clap::Parser)]
#[clap(about = "Manage your git repos with gall.", version = clap::crate_version!())]
#[clap(subcommand_required = true)]
pub(crate) struct Args {
    /// Run as if gall was started in <ROOT> instead of the current working directory.
    ///
    /// Defaults to the current working directory.
    #[clap(short = 'C', value_parser = parse_path)]
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
