use inquire::{Confirm, CustomType, Select, Text};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::{fs, io::Write, str::FromStr};
use toml;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct GingerDBConfig {
    pub branch: String,
    pub organization_id: String,
    pub database: Vec<DatabaseConfig>, // Unified all db types in one vector
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct DatabaseConfig {
    pub db_type: DbType, // Use DbType enum
    pub description: String,
    pub enable: bool,
    pub id: Option<String>,
    pub name: String,
    pub port: String,
    pub studio_port: Option<String>,
}

impl fmt::Display for DatabaseConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")] // This will map the enum to/from lowercase strings
pub enum DbType {
    Rdbms,
    DocumentDb,
    Cache,
}

impl fmt::Display for DbType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let db_type_str = match self {
            DbType::Rdbms => "rdbms",
            DbType::DocumentDb => "documentdb",
            DbType::Cache => "cache",
        };
        write!(f, "{}", db_type_str)
    }
}

impl FromStr for DbType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rdbms" => Ok(DbType::Rdbms),
            "documentdb" => Ok(DbType::DocumentDb),
            "cache" => Ok(DbType::Cache),
            _ => Err(format!("'{}' is not a valid DbType", s)),
        }
    }
}

pub fn read_config(file_path: &str) -> Result<GingerDBConfig, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(file_path)?;
    let config: GingerDBConfig = toml::from_str(&contents)?;
    Ok(config)
}

pub fn write_config(
    file_path: &str,
    config: &GingerDBConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let toml_string = toml::to_string(config)?;
    let mut file = fs::File::create(file_path)?;
    file.write_all(toml_string.as_bytes())?;
    Ok(())
}
pub fn alter_db(config: &mut GingerDBConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Create a list of database names for selection
    let db_names: Vec<String> = config.database.iter().map(|db| db.name.clone()).collect();

    // Select the database by its name from the list
    let selected_db_name = Select::new("Select a database to alter:", db_names).prompt()?;

    // Find the selected database by name
    if let Some(db) = config
        .database
        .iter_mut()
        .find(|db| db.name == selected_db_name)
    {
        // Prompts to alter the selected database
        let new_name = Text::new("New Name:").with_default(&db.name).prompt()?;
        let new_port = Text::new("New Port:").with_default(&db.port).prompt()?;
        let new_studio_port = Text::new("New Studio Port (optional):")
            .with_default(&db.studio_port.clone().unwrap_or_default())
            .prompt()?;
        let new_description = Text::new("New Description:")
            .with_default(&db.description)
            .prompt()?;

        // Update database configuration with new values
        db.name = new_name;
        db.port = new_port;
        db.studio_port = if new_studio_port.is_empty() {
            None
        } else {
            Some(new_studio_port)
        };
        db.description = new_description;

        // Write updated configuration back to the file
        write_config("db-compose.toml", &config)?;
        println!("Database configuration updated successfully!");
    } else {
        println!("Selected database not found!");
    }

    Ok(())
}
pub fn add_db(config: &mut GingerDBConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Ask for database type with a Select option
    let db_type_options = vec![DbType::Rdbms, DbType::DocumentDb, DbType::Cache];
    let db_type =
        Select::new("Which database type do you want to add?", db_type_options).prompt()?;

    // Get database details
    let description = Text::new("Description:")
        .with_validator(inquire::required!("This field is required"))
        .prompt()?;

    let enable = Confirm::new("Enable this database?")
        .with_default(true)
        .prompt()?;

    let id = Text::new("ID (optional):").prompt().ok();

    let name = Text::new("Name:")
        .with_validator(inquire::required!("This field is required"))
        .prompt()?;

    let port = CustomType::new("Port:")
        .with_formatter(&|i: i32| format!("{i}"))
        .with_error_message("Please type a valid port number")
        .prompt()?;

    let studio_port = Text::new("Studio Port (optional):").prompt().ok();

    // Add new database config to the unified database list
    config.database.push(DatabaseConfig {
        db_type,
        description,
        enable,
        id,
        name,
        port: port.to_string(),
        studio_port,
    });

    Ok(())
}
