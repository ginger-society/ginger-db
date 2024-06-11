use schemaClient::apis::configuration::Configuration;
use schemaClient::apis::get_all_models_api;
use schemaClient::models::ModelsReponse;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Result;
use std::fs::File;
use std::io::{BufReader, Write};
use std::process::exit;
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
    DateTimeField,
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
    auto_now_add: Option<bool>,
    auto_now: Option<bool>,
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
    docs: Option<String>,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
struct OptionData {
    value: String,
    label: String,
}

fn main() -> Result<()> {
    // Open the file in read-only mode with buffer.
    let file = File::open("runner-main/db.design.json").unwrap();
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
    // for schema in schemas.clone() {
    //     println!("{:#?}", schema);
    // }

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

            let mut output_file = File::create("runner-main/models.py").unwrap();
            output_file.write_all(rendered_template.as_bytes()).unwrap();
        }
        Err(e) => {
            println!("{:?}", e)
        }
    };

    match tera.render("admin.py.tpl", &context) {
        Ok(rendered_template) => {
            println!("{:?}", rendered_template);

            let mut output_file = File::create("runner-main/admin.py").unwrap();
            output_file.write_all(rendered_template.as_bytes()).unwrap();
        }
        Err(e) => {
            println!("{:?}", e)
        }
    };

    let open_api_config = Configuration {
        base_path: String::from("http://localhost:8000"),
        ..Default::default()
    };

    let app_tables_list = match get_namespace_tables(&open_api_config) {
        Ok(d) => d,
        Err(error) => {
            println!(
                "Unable to connect to the service, Are you connected to the internet /intranet ? : {}" , error
            );

            exit(1);
        }
    };
    println!("{:?}", app_tables_list);

    Ok(())
}

#[tokio::main]
async fn get_namespace_tables(openapi_configuration: &Configuration) -> Result<Vec<ModelsReponse>> {
    match get_all_models_api::get_all_models_list(&openapi_configuration).await {
        Ok(response) => Ok(response),
        Err(error) => {
            println!("{:?}", error);
            exit(1);
        }
    }
}
