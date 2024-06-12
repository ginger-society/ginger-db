use std::process::exit;

use inquire::{InquireError, Select, Text};

use crate::types::{LANG, ORM};

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
                Ok(selection) => {
                    match Text::new("Where is the schema server running ?")
                        .with_default("http://localhost:8000")
                        .prompt()
                    {
                        Ok(schema_url) => {
                            match Text::new("Where is your models going to be generated").prompt() {
                                Ok(root) => {
                                    println!(
                                        "Language : {:?} ,ORM : {:?} , Schema URL : {:?}, Root: {:?}",
                                        lang_selected, selection, schema_url, root
                                    )
                                }
                                Err(_) => {}
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
