use clap::Parser;
use clap::Subcommand;
use inquire::formatter::MultiOptionFormatter;
use inquire::list_option::ListOption;
use inquire::validator::Validation;
use inquire::MultiSelect;
use schemaClient::apis::configuration::Configuration;
use schemaClient::apis::get_all_models_api;
use schemaClient::apis::render_models_api;
use schemaClient::apis::render_models_api::RenderModelsListParams;
use schemaClient::models::ModelsReponse;
use schemaClient::models::RenderedModelsReponse;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Result;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::{BufReader, Write};
use std::path::Path;
use std::process::exit;
use tera::Context;
use tera::Tera;

mod init;

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a database project
    Init,
    /// Bring up the database up just like docker-compose
    Up,
    /// Configures a new db connection in a project
    Configure,
    /// Generate the ORM models files as per the configuration
    Render,
}

#[derive(Parser, Debug)]
#[command(name = "db-compose")]
#[command(about = "A database composition tool", long_about = None)]
#[command(version, long_about = None)]
struct Args {
    /// name of the command to run
    #[command(subcommand)]
    command: Commands,
}

// Database.toml related start

#[derive(Deserialize, Debug, Serialize)]
struct DBSchema {
    url: String,
    lang: String,
    orm: String,
    root: String,
}

#[derive(Deserialize, Debug, Serialize)]
struct DBTables {
    names: Vec<String>,
}

#[derive(Deserialize, Debug, Serialize)]
struct DBConfig {
    schema: DBSchema,
    tables: DBTables,
}

// Database.toml related structs ends

// Ginger models generator structs starts
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

// Ginger models generator structs ends

fn main() -> Result<()> {
    let args = Args::parse();
    println!("{:?}", args);

    match args.command {
        Commands::Init => init::main(),
        Commands::Render => {}
        Commands::Up => {}
        Commands::Configure => {}
    }

    // Open the file in read-only mode with buffer.
    let file = File::open("runner-main/db.design.json").unwrap();
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
            // println!("{:?}", rendered_template);

            let mut output_file = File::create("runner-main/models.py").unwrap();
            output_file.write_all(rendered_template.as_bytes()).unwrap();
        }
        Err(e) => {
            println!("{:?}", e)
        }
    };

    match tera.render("admin.py.tpl", &context) {
        Ok(rendered_template) => {
            // println!("{:?}", rendered_template);

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

    let db_config_path = Path::new("database.toml");

    // Read the configuration using the read_db_config function
    let mut db_config = read_db_config(db_config_path)?;

    let mut all_tables: Vec<String> = vec![];
    let mut selected_table_indexes: Vec<usize> = vec![];
    for table in app_tables_list.iter() {
        all_tables.push(String::from(&table.name));
    }

    for (itter_count, table_meta) in all_tables.iter().enumerate() {
        if db_config.tables.names.contains(&table_meta) {
            selected_table_indexes.push(itter_count);
        }
    }

    // println!("{:?}", selected_table_indexes);

    let model_selector_validator = |a: &[ListOption<&String>]| {
        if a.len() < 1 {
            return Ok(Validation::Invalid(
                "At least one table is required!".into(),
            ));
        }
        Ok(Validation::Valid)
    };

    let model_selector_formatter: MultiOptionFormatter<'_, String> =
        &|a| format!("{:?}", get_formated_str_selected_models(a));

    let ans = MultiSelect::new(
        "Select the tables you want to add to this project ",
        all_tables,
    )
    .with_validator(model_selector_validator)
    .with_formatter(model_selector_formatter)
    .with_page_size(20)
    .with_default(&selected_table_indexes)
    .prompt();

    // println!("{:?}", ans);

    match ans {
        Ok(selected_tables) => {
            db_config.tables.names = selected_tables.clone();

            write_db_config(db_config_path, &db_config)?;
            println!("Generating models...");

            let mut csv_list = String::from("");
            for (itter_count, selection) in selected_tables.iter().enumerate() {
                if itter_count > 0 {
                    csv_list += &String::from(",");
                }
                csv_list += &selection;
            }

            fetch_and_process_models(&open_api_config, csv_list, db_config)
        }
        Err(error) => eprintln!("{}", error),
    }

    // println!("{:?}", app_tables_list);

    Ok(())
}

#[tokio::main]
async fn get_rendered_tables(
    openapi_configuration: &Configuration,
    language: String,
    framework: String,
    tables: String,
) -> Result<Vec<RenderedModelsReponse>> {
    let render_models_api_parameter = RenderModelsListParams {
        language: Some(language),
        framework: Some(framework),
        models: Some(tables),
    };

    match render_models_api::render_models_list(&openapi_configuration, render_models_api_parameter)
        .await
    {
        Ok(response) => Ok(response),
        Err(e) => {
            eprintln!("Error: {:?}", e);
            exit(1);
        }
    }
}

pub fn remove_dir_contents<P: AsRef<Path>>(path: P) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        fs::remove_file(entry?.path())?;
    }
    Ok(())
}

fn fetch_and_process_models(
    open_api_config: &Configuration,
    csv_list: String,
    db_config: DBConfig,
) {
    match get_rendered_tables(
        open_api_config,
        db_config.schema.lang,
        db_config.schema.orm,
        csv_list,
    ) {
        Ok(models) => {
            match fs::create_dir_all(&db_config.schema.root) {
                Ok(_) => {}
                Err(err) => println!("{:?}", err),
            };

            let models_folder = db_config.schema.root;
            match fs::create_dir_all(&models_folder) {
                Ok(_) => {}
                Err(err) => println!("{:?}", err),
            };
            match remove_dir_contents(&models_folder) {
                Ok(_) => {
                    for model in models.iter() {
                        let file_path = format!("{}/{}", &models_folder, &model.file_name);
                        let _ = match File::create(file_path) {
                            Ok(mut c) => {
                                println!("Writing {}", model.file_name);
                                c.write_all(model.file_content.as_bytes())
                            }
                            Err(_) => {
                                eprintln!("Unable write the models files");
                                exit(1)
                            }
                        };
                    }
                    println!("Note : Some of the models are added automatically even if you have not selected them, this is because one model can depened upon multiple models in a M2M or ForeignKey relationship");
                }
                Err(error) => {
                    println!("{:?}", error)
                }
            };
        }
        Err(error) => {
            eprintln!("Error writing model files : {}", error)
        }
    };
}

fn write_db_config<P: AsRef<Path>>(path: P, config: &DBConfig) -> Result<()> {
    let toml_string = toml::to_string(config).unwrap();
    let mut file = File::create(path).unwrap();
    file.write_all(toml_string.as_bytes()).unwrap();
    Ok(())
}

fn get_formated_str_selected_models(a: &[ListOption<&String>]) -> String {
    let mut output = String::from("");
    for (itter_count, selection) in a.iter().enumerate() {
        println!("{:?}", selection);
        if itter_count > 0 {
            output += &String::from(",");
        }
        output += &selection.value;
    }
    return output;
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

fn read_db_config<P: AsRef<Path>>(path: P) -> Result<DBConfig> {
    // Open the file
    let mut file = File::open(path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    // Deserialize the TOML contents into the DBConfig struct
    let config: DBConfig = toml::from_str(&contents).unwrap();

    Ok(config)
}
