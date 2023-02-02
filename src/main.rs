use anyhow::Result;
use clap::{Parser, Subcommand};
use move_package::BuildConfig;
use std::path::PathBuf;

mod docgen;
use docgen::Docgen;

mod upload;
use upload::Upload;

#[derive(Parser)]
#[clap(name = "MoveDogs")]
#[clap(
    author,
    version,
    about = "CLI Documentation application for move language",
    long_about = "CLI Documentation application for move language, corresponding to docs.rs or crates.io in Rust."
)]
pub struct Cli {
    /// Sets a custom config file
    #[clap(short = 'p', long = "path", value_name = "FILE_PATH", global = true)]
    package_path: Option<PathBuf>,

    #[clap(flatten)]
    build_config: BuildConfig,

    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// does testing things
    Docgen(Docgen),
    Upload(Upload),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(config_path) = cli.package_path.as_deref() {
        println!("Value for config: {}", config_path.display());
    }

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match cli.command {
        Some(Command::Docgen(docgen)) => docgen.execute(cli.package_path, cli.build_config),
        Some(Command::Upload(upload)) => upload.execute(),
        None => {
            // TODO: Docgen + Upload 한큐에 돌리기
            println!("No subcommand was used");
            Ok(())
        }
    }
}
