use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::types::DBConfig;

pub fn write_db_config<P: AsRef<Path>>(path: P, config: &DBConfig) -> () {
    let toml_string = toml::to_string(config).unwrap();
    let mut file = File::create(path).unwrap();
    file.write_all(toml_string.as_bytes()).unwrap();
}

pub fn read_db_config<P: AsRef<Path>>(path: P) -> Result<DBConfig, Box<dyn std::error::Error>> {
    // Open the file
    let mut file = File::open(path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    // Deserialize the TOML contents into the DBConfig struct
    match toml::from_str(&contents) {
        Ok(config) => Ok(config),
        Err(err) => Err(Box::new(err)),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ComposeConfig {
    pub branch: String,
    pub schema_id: String,
}

pub fn read_compose_config_file(
    file_path: &str,
) -> Result<ComposeConfig, Box<dyn std::error::Error>> {
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: ComposeConfig = toml::from_str(&contents)?;
    Ok(config)
}

pub fn write_compose_config_file(config: &ComposeConfig, file_path: &str) -> io::Result<()> {
    let toml_str = toml::to_string(config).expect("Failed to serialize config");
    let mut file = File::create(file_path)?;
    file.write_all(toml_str.as_bytes())
}
