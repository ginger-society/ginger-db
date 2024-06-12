use std::{fs::File, io::Write, path::Path};

use crate::types::DBConfig;

pub fn write_db_config<P: AsRef<Path>>(path: P, config: &DBConfig) -> () {
    let toml_string = toml::to_string(config).unwrap();
    let mut file = File::create(path).unwrap();
    file.write_all(toml_string.as_bytes()).unwrap();
}
