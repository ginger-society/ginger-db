use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::exit;
use std::{fs, io};

use inquire::formatter::MultiOptionFormatter;
use inquire::list_option::ListOption;
use inquire::validator::Validation;
use inquire::MultiSelect;
use schema_gen_service::apis::configuration::Configuration;
use schema_gen_service::apis::get_all_models_api;
use schema_gen_service::apis::render_models_api::{self, RenderModelsListParams};
use schema_gen_service::models::{ModelsReponse, RenderedModelsReponse};

use crate::types::{DBConfig, LANG, ORM};
use crate::utils::write_db_config;

pub fn main(
    open_api_config: &Configuration,
    mut db_config: DBConfig,
    db_config_path: &Path,
    skip: bool,
) {
    if skip {
        let selected_tables = &db_config.tables;
        let mut csv_list = String::from("");
        for (itter_count, selection) in selected_tables.names.iter().enumerate() {
            if itter_count > 0 {
                csv_list += &String::from(",");
            }
            csv_list += &selection;
        }
        fetch_and_process_models(&open_api_config, csv_list, db_config);
        return;
    }

    let app_tables_list = match get_namespace_tables(&open_api_config) {
        Ok(d) => d,
        Err(error) => {
            println!(
                "Unable to connect to the service, Are you connected to the  intranet ? : {}",
                error
            );

            exit(1);
        }
    };

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

            write_db_config(db_config_path, &db_config);
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
}

#[tokio::main]
async fn get_rendered_tables(
    openapi_configuration: &Configuration,
    language: LANG,
    framework: ORM,
    tables: String,
) -> Result<Vec<RenderedModelsReponse>, Box<dyn std::error::Error>> {
    let render_models_api_parameter = RenderModelsListParams {
        language: Some(language.to_string()),
        framework: Some(framework.to_string()),
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
            // match remove_dir_contents(&models_folder) {
            //     Ok(_) => {

            //     }
            //     Err(error) => {
            //         println!("{:?}", error)
            //     }
            // };
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
            eprintln!("Error writing model files : {}", error)
        }
    };
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
async fn get_namespace_tables(
    openapi_configuration: &Configuration,
) -> Result<Vec<ModelsReponse>, Box<dyn std::error::Error>> {
    match get_all_models_api::get_all_models_list(&openapi_configuration).await {
        Ok(response) => Ok(response),
        Err(error) => {
            println!("{:?}", error);
            exit(1);
        }
    }
}
