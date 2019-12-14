use anyhow::{Context, Result};
use log::info;
use ron::{
    de::from_str,
    ser::{to_string_pretty, PrettyConfig},
};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use crate::config::Config;
use crate::git;
use crate::package::Packages;

pub struct Repo {
    config: Config,
    path: PathBuf,
}

impl Repo {
    pub fn new(config: Config) -> Result<Self> {
        let repo_directory = config.repo_directory.clone();
        let repo_url = config.repo.url.clone();
        let repo_branch = config.repo.branch.clone();

        let path = Path::new(&repo_directory);
        let path_str = path.to_str().expect("Could not get directory").to_string();

        let repo_exists = path.join(".git").exists();
        if repo_exists {
            info!("Opening existing repo: \"{}\".", path_str);

            git::pull(&path, false)?;
        } else {
            info!("Cloning repo \"{}\" to \"{}\".", repo_url, path_str);

            fs::create_dir_all(path)?;
            git::clone_single_branch(&path, &*repo_url, &*repo_branch, false)?;
        }

        Ok(Repo {
            config,
            path: path.to_path_buf(),
        })
    }

    pub fn read(&self) -> Result<Packages> {
        // Open the file
        let mut file = File::open(&self.config.full_file_path())
            .context("failed opening Emplace mirrors file")?;

        // Read the contents
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        // Deserialize the file into the struct
        Ok(from_str(&*contents)?)
    }

    pub fn mirror(&self, mut commands: Packages) -> Result<()> {
        // Get the message first before the old stuff is added
        let commit_msg = commands.commit_message();

        let full_path = self.config.full_file_path();
        if full_path.exists() {
            // A file already exists, merge the existing one with the current one
            let mut old: Packages = self.read()?;

            // Merge it with the new one
            commands.merge(&mut old);
        }

        // There's no file yet, just serialize everything and write it to a new file
        let toml_string = to_string_pretty(&commands, Repo::pretty_config())?;

        fs::write(&full_path, toml_string)?;

        info!("Commiting with message \"{}\".", commit_msg);
        git::add_file(&self.path, &*self.config.repo.file, false)?;
        git::commit_all(&self.path, &*commit_msg, false, false)?;

        info!("Pushing to remote.");
        git::push(&self.path, false)?;

        Ok(())
    }

    pub fn clean(&self, commands: Packages) -> Result<()> {
        // Overwrite the file
        let toml_string = to_string_pretty(&commands, Repo::pretty_config())?;

        let full_path = self.config.full_file_path();
        fs::write(&full_path, toml_string)?;

        let commit_msg = "Emplace - clean packages";
        info!("Commiting with message \"{}\".", commit_msg);
        git::add_file(&self.path, &*self.config.repo.file, false)?;
        git::commit_all(&self.path, &*commit_msg, false, false)?;

        info!("Pushing to remote.");
        git::push(&self.path, false)?;

        Ok(())
    }

    fn pretty_config() -> PrettyConfig {
        PrettyConfig {
            depth_limit: 2,
            indentor: "".into(),
            ..PrettyConfig::default()
        }
    }
}
