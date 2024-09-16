use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
    process::exit,
};

use serde_json::Value;

use inquire::{required, Confirm, CustomType, Text};
use tera::{Context, Tera};
use MetadataService::{apis::default_api::metadata_get_current_workspace, get_configuration};

use crate::utils::{write_config, GingerDBConfig};

pub async fn main(tera: Tera) {
    let home_dir = match dirs::home_dir() {
        Some(path) => path,
        None => {
            println!("Failed to locate home directory. Exiting.");
            exit(1);
        }
    };

    // Construct the path to the auth.json file
    let auth_file_path: PathBuf = [home_dir.to_str().unwrap(), ".ginger-society", "auth.json"]
        .iter()
        .collect();

    // Read the token from the file
    let mut file = match File::open(&auth_file_path) {
        Ok(f) => f,
        Err(_) => {
            println!("Failed to open {}. Exiting.", auth_file_path.display());
            exit(1);
        }
    };
    let mut contents = String::new();
    if let Err(_) = file.read_to_string(&mut contents) {
        println!("Failed to read the auth.json file. Exiting.");
        exit(1);
    }

    let json: Value = match serde_json::from_str(&contents) {
        Ok(v) => v,
        Err(_) => {
            println!("Failed to parse auth.json as JSON. Exiting.");
            exit(1);
        }
    };

    let token = match json.get("API_TOKEN").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => {
            println!("API_TOKEN not found in auth.json. Exiting.");
            exit(1);
        }
    };

    let metadata_service_config = get_configuration(Some(token));

    match metadata_get_current_workspace(&metadata_service_config).await {
        Ok(resp) => {
            let db_configs = GingerDBConfig {
                branch: "stage".to_string(),
                organization_id: resp.org_id,
                database: vec![],
            };

            match write_config("db-compose.toml", &db_configs) {
                Ok(_) => println!("Initialized successfully, use ginger-db add-db to get started"),
                Err(_) => {
                    println!("Error saving the file, please check if you have all the permissions")
                }
            };
        }
        Err(err) => {
            println!("{:?}", err);
            println!("Error getting the current session. Try logging in again")
        }
    }
}
