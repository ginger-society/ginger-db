use std::{
    fs::{self, File},
    io::Write,
};

use inquire::{required, CustomType, Text};
use tera::{Context, Tera};

pub fn main(tera: Tera) {
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

    let name = Text::new("DB Name:")
        .with_validator(required!("This field is required"))
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

    fs::create_dir_all(&name).unwrap();

    let mut context = Context::new();
    context.insert("name", &name);
    context.insert("port", &port);
    context.insert("studio_port", &studio_port);
    context.insert("db_username", &db_username);
    context.insert("db_password", &db_password);

    match tera.render("docker-compose.yml.tpl", &context) {
        Ok(rendered_template) => {
            let mut output_file = File::create(format!("{}/docker-compose.yml", name)).unwrap();
            output_file.write_all(rendered_template.as_bytes()).unwrap();
        }
        Err(e) => {
            println!("{:?}", e)
        }
    };

    match tera.render("db.design.json.tpl", &context) {
        Ok(rendered_template) => {
            let mut output_file = File::create(format!("{}/db.design.json", name)).unwrap();
            output_file.write_all(rendered_template.as_bytes()).unwrap();
        }
        Err(e) => {
            println!("{:?}", e)
        }
    };
}
