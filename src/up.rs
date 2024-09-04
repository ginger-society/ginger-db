use std::fs::File;
use std::io::Write;
use std::process::Command;

use tera::{Context, Tera};
use MetadataService::apis::default_api::{
    metadata_get_dbschema_by_id, MetadataGetDbschemaByIdParams,
};
use MetadataService::get_configuration;

use crate::types::{Schema, SchemaType};
use crate::utils::read_compose_config_file;

#[tokio::main]
pub async fn up(tera: Tera) {
    let open_api_config = get_configuration();

    let db_compose_config = read_compose_config_file("db-compose.toml").unwrap();

    println!("{:?}", db_compose_config);

    let mut schemas: Vec<Schema> = match metadata_get_dbschema_by_id(
        &open_api_config,
        MetadataGetDbschemaByIdParams {
            schema_id: db_compose_config.schema_id.to_string(),
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

    schemas.sort_by(|a, _b| {
        if a.schema_type == SchemaType::Enum {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    let mut context = Context::new();
    context.insert("schemas", &schemas);

    match tera.render("models.py.tpl", &context) {
        Ok(rendered_template) => {
            let mut output_file = match File::create("models.py") {
                Ok(file) => file,
                Err(err) => {
                    eprintln!("Error creating models.py: {:?}", err);
                    return;
                }
            };
            if let Err(err) = output_file.write_all(rendered_template.as_bytes()) {
                eprintln!("Error writing to models.py: {:?}", err);
            }
        }
        Err(e) => {
            eprintln!("Error rendering models.py template: {:?}", e);
        }
    };

    match tera.render("admin.py.tpl", &context) {
        Ok(rendered_template) => {
            let mut output_file = match File::create("admin.py") {
                Ok(file) => file,
                Err(err) => {
                    eprintln!("Error creating admin.py: {:?}", err);
                    return;
                }
            };
            if let Err(err) = output_file.write_all(rendered_template.as_bytes()) {
                eprintln!("Error writing to admin.py: {:?}", err);
            }
        }
        Err(e) => {
            eprintln!("Error rendering admin.py template: {:?}", e);
        }
    };

    let status = Command::new("docker-compose")
        .arg("up")
        .status()
        .expect("failed to execute docker-compose up");

    if status.success() {
        println!("docker-compose up executed successfully");
    } else {
        eprintln!("docker-compose up failed");
    }
}
