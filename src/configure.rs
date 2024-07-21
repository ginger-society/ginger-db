use std::{path::Path, process::exit};

use inquire::{InquireError, Select, Text};

use crate::{
    types::{DBConfig, DBSchema, DBTables, LANG, ORM},
    utils::write_db_config,
};

pub fn main() {
    let options = LANG::all();

    let ans: Result<LANG, InquireError> =
        Select::new("Please select the language used in this project", options).prompt();

    match ans {
        Ok(lang_selected) => {
            let options: Vec<ORM> = match lang_selected {
                LANG::Python => {
                    vec![ORM::SQLAlchemy, ORM::DjangoORM]
                }
                LANG::Rust => {
                    vec![ORM::Diesel]
                }
                LANG::TS => {
                    vec![ORM::TypeORM]
                }
            };

            let orm_selection: Result<ORM, InquireError> =
                Select::new("Which ORM you are using ?", options).prompt();

            match orm_selection {
                Ok(orm_selected) => {
                    match Text::new("Where is the schema server running ?")
                        .with_default("http://localhost:8000")
                        .prompt()
                    {
                        Ok(schema_url) => {
                            let schema_id = Text::new("Enter schema_id").prompt().unwrap();

                            match Text::new("Where is your models going to be generated").prompt() {
                                Ok(root) => {
                                    let db_config_path = Path::new("database.toml");

                                    let db_config = DBConfig {
                                        schema: DBSchema {
                                            url: schema_url,
                                            lang: lang_selected,
                                            orm: orm_selected,
                                            root: root,
                                            schema_id: schema_id,
                                            branch: "main".to_string(),
                                        },
                                        tables: DBTables { names: vec![] },
                                    };
                                    write_db_config(db_config_path, &db_config);
                                    println!("Success!")
                                }
                                Err(_) => {
                                    println!("Unable to gather all the information needed for initialization");
                                    exit(1);
                                }
                            };
                        }
                        Err(_) => {}
                    };
                }
                Err(_) => println!("There was an error, please try again"),
            }
        }
        Err(_) => {
            println!("You must select a language to proceed. Exiting!");
            exit(1);
        }
    };

    ()
}
