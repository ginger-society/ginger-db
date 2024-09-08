use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
};

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
