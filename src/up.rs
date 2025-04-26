use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

use ginger_shared_rs::{read_db_config, DbType};
use tera::{Context, Tera};
use MetadataService::apis::default_api::{
    metadata_get_dbschema_by_id, MetadataGetDbschemaByIdParams,
};
use MetadataService::get_configuration;

use serde_json::Value;

use crate::types::{Schema, SchemaType};

/// Generates `models.py` and `admin.py` files for a given database.
pub fn generate_python_files_for_db(db_name: &str, schemas: &[Schema], tera: &Tera) {
    // Sort schemas to prioritize Enums
    let mut sorted_schemas = schemas.to_vec();
    sorted_schemas.sort_by(|a, _b| {
        if a.schema_type == SchemaType::Enum {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    let mut context = Context::new();
    context.insert("schemas", &sorted_schemas);

    // Render models.py
    let models_path = format!("{}/models.py", db_name);
    match tera.render("models.py.tpl", &context) {
        Ok(rendered_template) => {
            if let Err(err) = fs::write(&models_path, rendered_template) {
                eprintln!("Error writing to models.py: {:?}", err);
            }
        }
        Err(e) => {
            eprintln!("Error rendering models.py template: {:?}", e);
        }
    }

    // Render admin.py if it doesn't exist
    let admin_path = format!("{}/admin.py", db_name);
    if !Path::new(&admin_path).exists() {
        match tera.render("admin.py.tpl", &context) {
            Ok(rendered_template) => {
                if let Err(err) = fs::write(&admin_path, rendered_template) {
                    eprintln!("Error writing to admin.py: {:?}", err);
                }
            }
            Err(e) => {
                eprintln!("Error rendering admin.py template: {:?}", e);
            }
        }
    } else {
        println!("admin.py already exists, skipping creation.");
    }
}

pub async fn up(tera: Tera, skip: bool) {
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

    let open_api_config = get_configuration(Some(token));

    let db_compose_config = read_db_config("db-compose.toml").unwrap();

    for db in db_compose_config
        .database
        .iter()
        .filter(|db| db.db_type == DbType::Rdbms)
    {
        println!("Processing RDBMS database: {}", db.name);

        // Fetch schemas for each database
        let schemas: Vec<Schema> = match metadata_get_dbschema_by_id(
            &open_api_config,
            MetadataGetDbschemaByIdParams {
                schema_id: db.clone().id.unwrap(),
                branch: Some(db_compose_config.branch.to_string()),
            },
        )
        .await
        {
            Ok(response) => {
                println!("{:?}", response);
                match serde_json::from_str(&response.data.unwrap().unwrap()) {
                    Ok(schemas) => schemas,
                    Err(err) => {
                        eprintln!("Error parsing schema from response: {:?}", err);
                        return;
                    }
                }
            }
            Err(e) => {
                println!("{:?}", e);
                eprintln!("Error getting the schema, please check your network");
                return;
            }
        };

        // Save the schema JSON to a file
        let schema_json_path = format!("{}/schema.json", db.clone().name);
        match File::create(&schema_json_path) {
            Ok(mut file) => {
                if let Err(err) = file.write_all(
                    serde_json::to_string_pretty(&schemas).unwrap().as_bytes(),
                ) {
                    eprintln!("Error writing schema.json: {:?}", err);
                }
            }
            Err(err) => {
                eprintln!("Error creating schema.json: {:?}", err);
            }
        }

        // Generate Python files
        generate_python_files_for_db(&db.name, &schemas, &tera);

        println!("Finished processing RDBMS database: {}", db.name);
    }

    let mut tera_context = Context::new();

    // Insert the list of databases directly into the Tera context
    tera_context.insert("databases", &db_compose_config.database);

    match tera.render("docker-compose.yml.tpl", &tera_context) {
        Ok(rendered_template) => {
            println!("rendered");
            if skip {
                exit(0);
            }
            let mut output_file = match File::create("docker-compose.yml") {
                Ok(file) => file,
                Err(err) => {
                    eprintln!("Error creating docker-compose.yml: {:?}", err);
                    return;
                }
            };
            if let Err(err) = output_file.write_all(rendered_template.as_bytes()) {
                eprintln!("Error writing to docker-compose.toml: {:?}", err);
            }
            // Run docker-compose up as a blocking command to allow terminal takeover
            let status = Command::new("docker-compose")
                .arg("up")
                .status()
                .expect("Failed to start docker-compose up");

            if !status.success() {
                eprintln!("docker-compose exited with status: {:?}", status);
            }
        }
        Err(_) => {
            println!("Error")
        }
    }
}
