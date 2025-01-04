use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct OptArgs {
    /// download files into destination folder
    #[clap(short, long)]
    pub flex_file: PathBuf,

    /// download files into destination folder
    #[clap(short, long)]
    pub journal_file: Option<PathBuf>,

    #[clap(short, long)]
    pub only_map: bool,

    /// Verbose logging mode (-v, -vv, -vvv)
    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,
}
