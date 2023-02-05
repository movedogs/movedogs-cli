use anyhow::Result;
use clap::{Parser, Subcommand};
// use move_docgen::DocgenOptions;
// use move_package::{BuildConfig, ModelConfig};
use std::{fs, path::PathBuf};

use aptos::common::types::MovePackageDir;
use aptos_framework::{docgen::DocgenOptions, BuildOptions, BuiltPackage};
use async_trait::async_trait;

#[derive(Parser)]
#[clap(name = "docgen")]
pub struct DocumentPackage {
    #[clap(flatten)]
    move_options: MovePackageDir,

    #[clap(flatten)]
    docgen_options: DocgenOptions,
}

impl DocumentPackage {
    /// Calling the Docgen
    pub async fn execute(self) -> Result<()> {
        let DocumentPackage {
            move_options,
            docgen_options,
        } = self;
        let build_options = BuildOptions {
            with_srcs: false,
            with_abis: false,
            with_source_maps: false,
            with_error_map: false,
            with_docs: true,
            install_dir: None,
            named_addresses: move_options.named_addresses(),
            docgen_options: Some(docgen_options),
            skip_fetch_latest_git_deps: true,
            bytecode_version: Some(move_options.bytecode_version_or_detault()),
        };
        BuiltPackage::build(move_options.get_package_path()?, build_options)?;
        Ok(())
    }
}
