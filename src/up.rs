use std::io::Write;
use std::process::Command;
use std::{fs::File, io::BufReader};

use tera::{Context, Tera};

use crate::types::{Schema, SchemaType};

pub fn main(tera: Tera) {
    // Open the file in read-only mode with buffer.
    let file = File::open("db.design.json").unwrap();
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `Schema`.
    let mut schemas: Vec<Schema> = serde_json::from_reader(reader).unwrap();

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
            // println!("{:?}", rendered_template);

            let mut output_file = File::create("models.py").unwrap();
            output_file.write_all(rendered_template.as_bytes()).unwrap();
        }
        Err(e) => {
            println!("{:?}", e)
        }
    };

    match tera.render("admin.py.tpl", &context) {
        Ok(rendered_template) => {
            // println!("{:?}", rendered_template);

            let mut output_file = File::create("admin.py").unwrap();
            output_file.write_all(rendered_template.as_bytes()).unwrap();
        }
        Err(e) => {
            println!("{:?}", e)
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
