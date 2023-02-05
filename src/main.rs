#![forbid(unsafe_code)]

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use anyhow::Result;
use clap::{Parser, Subcommand};
// use move_package::BuildConfig;
use std::path::PathBuf;
use std::process::exit;

mod docgen;
use docgen::DocumentPackage;

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
    /// generate document files (*.md) in /doc directory from move source files in /sources directory.
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// generate document files (*.md) in /doc directory from move source files in /sources directory.
    Docgen(DocumentPackage),
    /// upload document files (*.md) stored in /doc directory to movedogs server.
    Upload(Upload),
}

impl Cli {
    pub async fn execute(self) -> Result<()> {
        use crate::Command::{Docgen, Upload};
        match self.command {
            Docgen(docgen) => docgen.execute().await,
            Upload(upload) => upload.execute().await,
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = cli.execute().await;

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match result {
        Ok(_) => println!("Success"),
        Err(inner) => {
            println!("{}", inner);
            exit(1);
        }
    }
}
