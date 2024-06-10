use serde::Deserialize;
use serde::Serialize;
use serde_json::Result;
use std::fs::File;
use std::io::{BufReader, Write};
use tera::Context;
use tera::Tera;

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum OnDeleteOptions {
    Cascade,
    Protect,
    SetNull,
    DoNothing,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
enum ColumnType {
    CharField,
    BooleanField,
    DateField,
    ForeignKey,
    BigAutoField,
    PositiveIntegerField,
    FloatField,
    ManyToManyField,
    TextField,
    OneToOneField,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum SchemaType {
    Table,
    Enum,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
struct Schema {
    id: String,
    rows: Vec<Row>,
    data: Data,
    #[serde(rename = "type")]
    schema_type: SchemaType,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
struct Row {
    id: String,
    data: FieldData,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
enum DefaultValue {
    Boolean(bool),
    String(String),
}
#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
struct FieldData {
    name: String,
    #[serde(rename = "field_name")]
    field_name: String,
    #[serde(rename = "type")]
    field_type: ColumnType,
    null: Option<bool>,
    options_target: Option<String>,
    default: Option<DefaultValue>,
    max_length: Option<String>,

    // Forign key related
    target: Option<String>,
    related_name: Option<String>,
    on_delete: Option<OnDeleteOptions>,
}

#[derive(Deserialize, Debug, Serialize)]
struct ForeignKeyData {
    id: String,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
struct Data {
    id: String,
    #[serde(rename = "table_name")]
    table_name: String,
    name: String,
    #[serde(default)]
    options: Option<Vec<OptionData>>,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
struct OptionData {
    value: String,
    label: String,
}

fn main() -> Result<()> {
    println!("Hello, world!");

    // Open the file in read-only mode with buffer.
    let file = File::open("src/db.design.json").unwrap();
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `Schema`.
    let mut schemas: Vec<Schema> = serde_json::from_reader(reader).unwrap();

    schemas.sort_by(|a, b| {
        if a.schema_type == SchemaType::Enum {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });
    // Print the schemas to see if they are deserialized correctly.
    for schema in schemas.clone() {
        println!("{:#?}", schema);
    }

    // Use globbing
    let tera = match Tera::new("templates/**/*.tpl") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    let mut context = Context::new();
    context.insert("modelName", "model1");
    context.insert("schemas", &schemas);

    match tera.render("models.py.tpl", &context) {
        Ok(rendered_template) => {
            println!("{:?}", rendered_template);

            let mut output_file = File::create("output/models.py").unwrap();
            output_file.write_all(rendered_template.as_bytes()).unwrap();
        }
        Err(e) => {
            println!("{:?}", e)
        }
    };

    Ok(())
}
