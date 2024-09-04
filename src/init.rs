use std::{
    fs::{self, File},
    io::Write,
};

use inquire::{required, Confirm, CustomType, Text};
use tera::{Context, Tera};

use crate::utils::{write_compose_config_file, ComposeConfig};

pub fn main(tera: Tera, repo: String) {
    let schema_id = repo.split('/').nth(0).unwrap_or("");
    let branch = repo.split('/').nth(1).unwrap_or("");

    let db_compose_config = ComposeConfig {
        schema_id: schema_id.to_string(),
        branch: branch.to_string(),
    };

    let _ = write_compose_config_file(&db_compose_config, "db-compose.toml");

    let mut context = Context::new();
    let create_rdms = Confirm::new("Do you want to add PostgresSQL ?")
        .with_default(false)
        .prompt();

    let name = Text::new("DB Name:")
        .with_validator(required!("This field is required"))
        .prompt()
        .unwrap();

    match create_rdms {
        Ok(true) => {
            let port: i32 = CustomType::new("Port:")
            .with_formatter(&|i: i32| format!("{i}"))
            .with_error_message("Please type a valid port number")
            .with_default(5432)
            .with_help_message(
                "This is the port where the database will be available when used in your project",
            )
            .prompt()
            .unwrap();

            let studio_port: i32 = CustomType::new("Studio Port:")
                .with_formatter(&|i: i32| format!("{i}"))
                .with_error_message("Please type a valid port number")
                .with_default(8000)
                .with_help_message("This is the port where the studio will be available")
                .prompt()
                .unwrap();

            let db_username = Text::new("DB Username:")
                .with_validator(required!("This field is required"))
                .with_default("postgres")
                .prompt()
                .unwrap();

            let db_password = Text::new("DB Password:")
                .with_validator(required!("This field is required"))
                .with_default("postgres")
                .prompt()
                .unwrap();
            context.insert("create_rdms", &true);
            context.insert("port", &port);
            context.insert("studio_port", &studio_port);
            context.insert("db_username", &db_username);
            context.insert("db_password", &db_password);
        }
        Ok(false) => {}
        Err(_) => println!("You cancelled!"),
    }

    let create_mongodb = Confirm::new("Do you want to add MongoDB support ?")
        .with_default(false)
        .prompt();

    match create_mongodb {
        Ok(true) => {
            let mongo_port: i32 = CustomType::new("MongoDB Port:")
                .with_formatter(&|i: i32| format!("{i}"))
                .with_error_message("Please type a valid port number")
                .with_default(27017)
                .with_help_message("This is the port where the MongoDB will be available")
                .prompt()
                .unwrap();

            let mongo_studio_port: i32 = CustomType::new("MongoDB Studio Port:")
                .with_formatter(&|i: i32| format!("{i}"))
                .with_error_message("Please type a valid port number")
                .with_default(4321)
                .with_help_message("This is the port where the MongoDB Studio will be available")
                .prompt()
                .unwrap();

            let mongo_username = Text::new("MongoDB Username:")
                .with_validator(required!("This field is required"))
                .with_default("mongo")
                .prompt()
                .unwrap();

            let mongo_password = Text::new("MongoDB Password:")
                .with_validator(required!("This field is required"))
                .with_default("mongo")
                .prompt()
                .unwrap();

            context.insert("create_mongodb", &true);
            context.insert("mongo_port", &mongo_port);
            context.insert("mongo_username", &mongo_username);
            context.insert("mongo_password", &mongo_password);
            context.insert("mongo_studio_port", &mongo_studio_port);
        }
        Ok(false) => {}
        Err(_) => println!("You cancelled"),
    }

    let create_redis = Confirm::new("Do you want to add Redis support ?")
        .with_default(false)
        .prompt();

    match create_redis {
        Ok(true) => {
            let redis_port: i32 = CustomType::new("Redis Port:")
                .with_formatter(&|i: i32| format!("{i}"))
                .with_error_message("Please type a valid port number")
                .with_default(6379)
                .with_help_message("This is the port where the Redis will be available")
                .prompt()
                .unwrap();

            context.insert("create_redis", &true);
            context.insert("redis_port", &redis_port);
        }
        Ok(false) => {}
        Err(_) => println!("You cancelled"),
    }

    fs::create_dir_all(&name).unwrap();

    context.insert("name", &name);

    match tera.render("docker-compose.yml.tpl", &context) {
        Ok(rendered_template) => {
            let mut output_file = File::create(format!("{}/docker-compose.yml", name)).unwrap();
            output_file.write_all(rendered_template.as_bytes()).unwrap();
        }
        Err(e) => {
            println!("{:?}", e)
        }
    };
}
