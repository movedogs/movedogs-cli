// need to be refactored after implement necessary features.
// split to lib.rs, main.rs, docgen.rs, upload.rs, etc.

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use move_docgen::DocgenOptions;
use move_package::{BuildConfig, ModelConfig};
use reqwest::blocking::{multipart, Client};
use std::{fs, io::Read, path::PathBuf};

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

#[derive(Parser)]
#[clap(name = "docgen")]
pub struct Docgen {
    /// The level where we start sectioning. Often markdown sections are rendered with
    /// unnecessary large section fonts, setting this value high reduces the size
    #[clap(long = "section-level-start", value_name = "HEADER_LEVEL")]
    pub section_level_start: Option<usize>,
    /// Whether to exclude private functions in the generated docs
    #[clap(long = "exclude-private-fun")]
    pub exclude_private_fun: bool,
    /// Whether to exclude specifications in the generated docs
    #[clap(long = "exclude-specs")]
    pub exclude_specs: bool,
    /// Whether to put specifications in the same section as a declaration or put them all
    /// into an independent section
    #[clap(long = "independent-specs")]
    pub independent_specs: bool,
    /// Whether to exclude Move implementations
    #[clap(long = "exclude-impl")]
    pub exclude_impl: bool,
    /// Max depth to which sections are displayed in table-of-contents
    #[clap(long = "toc-depth", value_name = "DEPTH")]
    pub toc_depth: Option<usize>,
    /// Do not use collapsed sections (<details>) for impl and specs
    #[clap(long = "no-collapsed-sections")]
    pub no_collapsed_sections: bool,
    /// In which directory to store output
    #[clap(long = "output-directory", value_name = "PATH")]
    pub output_directory: Option<String>,
    /// A template for documentation generation. Can be multiple
    #[clap(long = "template", short = 't', value_name = "FILE")]
    pub template: Vec<String>,
    /// An optional file containing reference definitions. The content of this file will
    /// be added to each generated markdown doc
    #[clap(long = "references-file", value_name = "FILE")]
    pub references_file: Option<String>,
    /// Whether to include dependency diagrams in the generated docs
    #[clap(long = "include-dep-diagrams")]
    pub include_dep_diagrams: bool,
    /// Whether to include call diagrams in the generated docs
    #[clap(long = "include-call-diagrams")]
    pub include_call_diagrams: bool,
    /// If this is being compiled relative to a different place where it will be stored (output directory)
    #[clap(long = "compile-relative-to-output-dir")]
    pub compile_relative_to_output_dir: bool,
}

impl Docgen {
    /// Calling the Docgen
    pub fn execute(self, path: Option<PathBuf>, config: BuildConfig) -> Result<()> {
        let model = config.move_model_for_package(
            &path.unwrap_or_default(),
            ModelConfig {
                all_files_as_targets: false,
                target_filter: None,
            },
        )?;

        let mut options = DocgenOptions::default();

        if !self.template.is_empty() {
            options.root_doc_templates = self.template;
        }
        if self.section_level_start.is_some() {
            options.section_level_start = self.section_level_start.unwrap();
        }
        if self.exclude_private_fun {
            options.include_private_fun = false;
        }
        if self.exclude_specs {
            options.include_specs = false;
        }
        if self.independent_specs {
            options.specs_inlined = false;
        }
        if self.exclude_impl {
            options.include_impl = false;
        }
        if self.toc_depth.is_some() {
            options.toc_depth = self.toc_depth.unwrap();
        }
        if self.no_collapsed_sections {
            options.collapsed_sections = false;
        }
        if self.output_directory.is_some() {
            options.output_directory = self.output_directory.unwrap();
        }
        if self.references_file.is_some() {
            options.references_file = self.references_file;
        }
        if self.compile_relative_to_output_dir {
            options.compile_relative_to_output_dir = true;
        }

        // We are using the full namespace, since we already use `Docgen` here.
        // Docgen is the most suitable name for both: this Docgen subcommand,
        // and the actual move_docgen::Docgen.
        let generator = move_docgen::Docgen::new(&model, &options);

        for (file, content) in generator.gen() {
            let path = PathBuf::from(&file);
            fs::create_dir_all(path.parent().unwrap())?;
            fs::write(path.as_path(), content)?;
            println!("Generated {:?}", path);
        }

        anyhow::ensure!(
            !model.has_errors(),
            "Errors encountered while generating documentation!"
        );

        println!("\nDocumentation generation successful!");
        Ok(())
    }
}

#[derive(Parser)]
#[clap(name = "upload")]
pub struct Upload {}
impl Upload {
    pub fn execute(self) -> Result<()> {
        println!("Upload");
        let paste_api = "https://paste.rs";
        let mut move_toml = fs::File::open("Move.toml")?;
        // TODO: show error message if Move.toml does not exist

        // TODO: read git info.

        // TODO: iterate files in doc folder and concat them into one json format.
        let form = multipart::Form::new();

        let mut contents = String::new();
        move_toml.read_to_string(&mut contents)?;

        let client = Client::new();
        // TODO: post contents as json format
        let response = client.post(paste_api).body(contents).send();
        match response {
            Ok(response) => {
                if response.status().is_success() {
                    println!(
                        "Your package has been successfully uploaded to {}.",
                        response.text()?
                    );
                } else if response.status().is_client_error() {
                    bail!("{}", response.text()?)
                } else if response.status().is_server_error() {
                    bail!("An unexpected error occurred. Please try again later");
                }
            }
            Err(_) => {
                bail!("An unexpected error occurred. Please try again later");
            }
        }
        Ok(())
    }
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

    // Continued program logic goes here...
}
