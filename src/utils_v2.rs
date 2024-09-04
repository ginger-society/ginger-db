use serde::{Deserialize, Serialize}; // Added Serialize for writing
use std::fs;
use std::io::Write;

#[derive(Debug, Deserialize, Serialize)] // Serialize trait added
pub struct Config {
    // Made public
    pub branch: String,
    pub organization_id: String,
    pub rdbms: Vec<DatabaseConfig>, // Vec to handle multiple entries
    pub documentdb: Vec<DatabaseConfig>, // Vec to handle multiple entries
    pub cache: Vec<DatabaseConfig>, // Vec to handle multiple entries
}

#[derive(Debug, Deserialize, Serialize)] // Serialize trait added
pub struct DatabaseConfig {
    // Made public
    pub description: String,
    pub enable: bool,
    pub id: Option<String>, // id is optional
    pub name: String,
    pub port: String,
    pub studio_port: Option<String>, // Made optional
}

pub fn read_config(file_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    // Read the contents of the TOML file
    let contents = fs::read_to_string(file_path)?;

    // Parse the TOML contents into the Config struct
    let config: Config = toml::from_str(&contents)?;

    Ok(config)
}

pub fn write_config(file_path: &str, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    // Convert the Config struct back to TOML string
    let toml_string = toml::to_string(config)?;

    // Write the TOML string to the specified file
    let mut file = fs::File::create(file_path)?;
    file.write_all(toml_string.as_bytes())?;

    Ok(())
}
