use anyhow::{bail, Result};
use clap::Parser;
use reqwest::{multipart, Client};
use serde_json;
use std::{collections::HashMap, fs, path::PathBuf, process::Command as ProcessCommand};
use toml::Table;

#[derive(Parser)]
#[clap(name = "upload")]
pub struct Upload {
    /// Add short description about the module.
    #[clap(short = 'D', long = "description", value_name = "DESCRIPTION")]
    description: Option<String>,
}
impl Upload {
    pub async fn execute(self) -> Result<()> {
        println!("Upload");
        // let paste_api = "https://paste.rs";
        let document_api = "https://api.movedogs.org/document";
        let metadata_api = "https://api.movedogs.org/package";

        let mut map = HashMap::new();
        let mut description = String::new();

        let client = Client::new();

        // read github repository url from .git directory
        let output = ProcessCommand::new("git")
            .current_dir(".")
            .args(["remote", "-v"])
            .output()
            .unwrap();
        if !output.status.success() || output.stdout.is_empty() {
            bail!("invalid git repository")
        }

        let mut github_repo_url = String::new();

        let lines = String::from_utf8_lossy(output.stdout.as_slice());
        let lines = lines.split('\n');
        for line in lines {
            if line.contains("github.com") {
                let tokens: Vec<&str> = line.split(&['\t', ' '][..]).collect();
                if tokens.len() != 3 {
                    bail!("invalid remote url")
                }
                // convert ssh url to https
                let https_url = if tokens[1].starts_with("git@github.com") {
                    let mut token: Vec<&str> = tokens[1].split(":").collect();
                    token[0] = "https://github.com/";
                    token.join("")
                    // tokens[1].replace(':', "/").replace("git@", "https://")
                } else {
                    String::from(tokens[1])
                };
                github_repo_url = if https_url.ends_with(".git") {
                    https_url[..https_url.len() - 4].to_string()
                } else {
                    https_url
                };
            }
        }

        println!("github_repo_url: {}", github_repo_url);
        map.insert("github", github_repo_url.as_str());

        let mut form = multipart::Form::new();
        let mut part: multipart::Part;

        let move_toml = fs::read_to_string("Move.toml").expect("Unable to read Move.toml");
        let move_toml_str = move_toml.as_str();

        // Parsing Move.toml to get module info.
        let mut toml_parser = move_toml_str.parse::<Table>().unwrap();
        let mut filename = String::new();

        let package = toml_parser.get("package");
        if package.is_none() {
            bail!("package is not defined in Move.toml")
        }
        let package = package.unwrap();

        let package_name = package.get("name");
        if let Some(package_name) = package_name {
            println!("package_name: {:?}", package_name);
            let package_name = package_name.as_str().unwrap_or_default();
            map.insert("packageName", package_name);
            filename = package_name.to_string();
        } else {
            bail!("package name is not defined in Move.toml")
        }

        let version = package.get("version");
        if let Some(version) = version {
            println!("version: {:?}", version);
            let version = version.as_str().unwrap_or_default();
            map.insert("version", version);
        } else {
            bail!("package version is not defined in Move.toml")
        }

        let license = package.get("license");
        if let Some(license) = license {
            println!("license: {:?}", license);
            let license = license.as_str().unwrap_or_default();
            map.insert("license", license);
        } else {
            map.insert("license", "");
        }

        let authors = package.get("authors");
        let mut stringfied_author = String::new();
        if let Some(authors) = authors {
            println!("authors: {:?}", authors);
            let author = authors.as_array().unwrap();
            stringfied_author = serde_json::to_string(&author).unwrap();
            map.insert("authors", stringfied_author.as_str());
        } else {
            map.insert("authors", "[\"\"]");
        }

        if let Some(message) = self.description {
            description = message;
            map.insert("description", description.as_str());
        } else {
            map.insert("description", "");
        }

        println!("format: {}", filename);

        // upload md files from /doc directory.
        let mut module_name_vec: Vec<String> = Vec::new();
        let paths = fs::read_dir("doc")?;
        for element in paths {
            let path = element.unwrap().path();
            if let Some(extension) = path.extension() {
                if extension == "md" {
                    println!("{:?}", path);
                    let mut filename_of_module = filename.clone();
                    let md_file = Self::read_file_and_module_name(
                        &mut filename_of_module,
                        &mut module_name_vec,
                        &path,
                    )
                    .unwrap();
                    part = multipart::Part::text(md_file).file_name(filename_of_module);
                    form = form.part("files", part);
                }
            }
        }

        let stringfied_module_name = serde_json::to_string(&module_name_vec).unwrap();
        map.insert("moduleNames", stringfied_module_name.as_str());

        // upload metadata of Package
        println!("uploading metadata...");
        let res = client.post(metadata_api).json(&map).send().await;

        match res {
            Ok(response) => {
                if response.status().is_success() {
                    println!(
                        "Your package has been successfully uploaded to {}.",
                        response.text().await?
                    );
                } else if response.status().is_client_error() {
                    bail!("{}", response.text().await?)
                } else if response.status().is_server_error() {
                    bail!("An unexpected error occurred. Please try again later");
                }
            }
            Err(_) => {
                bail!("An unexpected error occurred. Please try again later");
            }
        }

        // upload md document files of Package
        println!("uploading files...");
        let response = client.post(document_api).multipart(form).send().await;
        match response {
            Ok(response) => {
                if response.status().is_success() {
                    println!(
                        "Your package has been successfully uploaded to {}.",
                        response.text().await?
                    );
                } else if response.status().is_client_error() {
                    bail!("{}", response.text().await?)
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

    fn read_file_and_module_name(
        filename: &mut String,
        module_name_vec: &mut Vec<String>,
        path: &PathBuf,
    ) -> Result<String> {
        let md_file = fs::read_to_string(path).expect("Unable to read md file");
        let lines = md_file.split("\n");
        // parse md_file to get module name.
        for line in lines {
            if line.starts_with("# Module") {
                let tokens = line.split("`");
                for token in tokens {
                    if token.contains("::") {
                        let module_name = token.split("::").last().unwrap();
                        println!("line: {}", module_name);
                        filename.push_str("+");
                        filename.push_str(module_name);
                        filename.push_str(".md");
                        module_name_vec.push(module_name.to_string());
                    }
                }
                break;
            }
        }
        Ok(md_file)
    }
}
