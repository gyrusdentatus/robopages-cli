use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};

mod create;
mod install;
mod run;
mod serve;
mod view;

pub(crate) use create::*;
pub(crate) use install::*;
pub(crate) use run::*;
pub(crate) use serve::*;
pub(crate) use view::*;

use crate::book::templates::Template;

const DEFAULT_REPO: &str = "dreadnode/robopages";
const DEFAULT_PATH: &str = "~/.robopages/";

#[derive(Debug, Parser)]
#[clap(name = "robopages", version)]
pub(crate) struct Args {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Install robopages from a given repository or ZIP archive.
    Install {
        /// Repository user/name, URL or ZIP archive path.
        #[clap(long, short = 'S', default_value = DEFAULT_REPO)]
        source: String,
        /// Destination path.
        #[clap(long, short = 'P', default_value = DEFAULT_PATH)]
        path: Utf8PathBuf,
    },
    /// Create a new robopage file.
    Create {
        /// Template name.
        #[clap(long, short = 'T', value_enum, default_value = "basic")]
        template: Template,
        /// File name.
        #[clap(long, short = 'N', default_value = "robopage.yml")]
        name: Utf8PathBuf,
    },
    /// View currently installed robopages.
    View {
        /// Base path to search for robopages.
        #[clap(long, short = 'P', default_value = DEFAULT_PATH)]
        path: Utf8PathBuf,
        /// Filter results by this string.
        #[clap(long, short = 'F')]
        filter: Option<String>,
    },
    /// Serve the robopages as a local API.
    Serve {
        /// Base path to search for robopages.
        #[clap(long, short = 'P', default_value = DEFAULT_PATH)]
        path: Utf8PathBuf,
        /// Filter results by this string.
        #[clap(long, short = 'F')]
        filter: Option<String>,
        /// Address to bind to.
        #[clap(long, short = 'A', default_value = "127.0.0.1:8000")]
        address: String,
        /// If set, the tool will not attempt to pre build and pull all containers.
        #[clap(long)]
        lazy: bool,
        /// Maximum number of parallel calls to execute. Leave to 0 to use all available cores.
        #[clap(long, default_value = "0")]
        workers: usize,
    },
    /// Execute a function from the robopages.
    Run {
        /// Base path to search for robopages.
        #[clap(long, short = 'P', default_value = DEFAULT_PATH)]
        path: Utf8PathBuf,
        /// Function name.
        #[clap(long, short = 'F')]
        function: String,
        /// Execute the function without user interaction.
        #[clap(long, short = 'A')]
        auto: bool,
    },
}
