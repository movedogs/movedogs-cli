use anyhow::{bail, Result};
use clap::Parser;
use reqwest::blocking::{multipart, Client};
use std::{collections::HashMap, fs, path::PathBuf, process::Command as ProcessCommand};
use toml::Parser as TomlParser;

#[derive(Parser)]
#[clap(name = "upload")]
pub struct Upload {
    /// Add short description about the module.
    #[clap(short = 'D', long = "description", value_name = "DESCRIPTION")]
    description: Option<String>,
}
impl Upload {
    pub fn execute(self) -> Result<()> {
        println!("Upload");
        let paste_api = "https://paste.rs";
        // let document_api = "http://localhost:4200/document";
        // let metadata_api = "http://localhost:4200/module";

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
                    tokens[1].replace(':', "/").replace("git@", "https://")
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
        // TODO: show error message & break if Move.toml does not exist

        // Parsing Move.toml to get module info.
        let mut toml_parser = TomlParser::new(move_toml_str);
        let mut filename = String::new();

        match toml_parser.parse() {
            Some(value) => {
                let package = value.get("package").unwrap();
                let package_name = package.lookup("name").unwrap().as_str().unwrap();
                let version = package.lookup("version").unwrap().as_str().unwrap();
                let license = package.lookup("license").unwrap().as_str().unwrap();
                let authors = package.lookup("authors").unwrap().as_slice().unwrap();
                println!("authors: {:?}", authors);
                let addresses = value.get("addresses").unwrap();
                // TODO: key-value lookup 하는 부분 하드코딩되어있음. -> addresses 메타데이터 넘겨줄 필요 x
                let address = addresses.lookup("std").unwrap().as_str().unwrap();
                println!(
                    "name: {}, version: {}, address: {:#?}",
                    package_name, version, address
                );
                filename = package_name.to_string();

                map.insert("name", package_name);
                map.insert("address", address);
                map.insert("version", version);
                map.insert("license", license);
                map.insert("author", authors[0].as_str().unwrap()); // TODO: only support first author; need to support multiple authors.

                if let Some(message) = self.description {
                    description = message;
                    map.insert("description", description.as_str());
                }

                // TODO: change mock api to real server api (metadata_api)
                let res = client.post(paste_api).json(&map).send();

                match res {
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
            }
            None => {
                println!("parse errors: {:?}", toml_parser.errors);
            }
        }
        println!("format: {}", filename);

        // upload md files from /doc directory.
        let paths = fs::read_dir("doc")?;
        for element in paths {
            let path = element.unwrap().path();
            if let Some(extension) = path.extension() {
                if extension == "md" {
                    println!("{:?}", path);
                    let mut filename_of_module = filename.clone();
                    let md_file =
                        Self::read_file_and_module_name(&mut filename_of_module, &path).unwrap();
                    part = multipart::Part::text(md_file).file_name(filename_of_module);
                    form = form.part("file", part);
                }
            }
        }

        println!("content-type");
        // TODO: change mock api to real server api (document_api)
        let response = client.post(paste_api).multipart(form).send();
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

    fn read_file_and_module_name(filename: &mut String, path: &PathBuf) -> Result<String> {
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
                    }
                }
                break;
            }
        }
        Ok(md_file)
    }
}
