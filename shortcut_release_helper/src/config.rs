use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::Deserialize;

use crate::types::{RepositoryConfiguration, RepositoryName};

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct AppConfig {
    pub template_file: PathBuf,
    pub repositories: HashMap<RepositoryName, RepositoryConfiguration>,
}

impl AppConfig {
    pub fn parse(config_file_path: &Path) -> Result<Self> {
        let mut file = File::open(config_file_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(toml::from_str(&contents)?)
    }
}
