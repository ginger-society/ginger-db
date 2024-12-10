use clap::{Parser, Subcommand};

use ginger_shared_rs::{
    read_consumer_db_config, read_db_config, utils::get_token_from_file_storage, write_db_config,
};
use schema_gen_service::apis::configuration::{ApiKey, Configuration};
use serde_json::Result;
use std::path::Path;
use templates::get_renderer;
use ui::render_ui;
use up::up;
use utils::{add_db, alter_db};

mod configure;
mod init;
mod render;
mod templates;
mod types;
mod ui;
mod up;
mod utils;

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a database project
    Init,
    /// Bring up the database up just like docker-compose
    Up {
        /// Skip the running docker compose up command
        #[arg(short, long)]
        skip: bool,
    },
    /// Configures a new db connection in a project
    Configure,
    /// Generate the ORM models files as per the configuration
    Render {
        /// Skip the rendering of certain files
        #[arg(short, long)]
        skip: bool,
    },
    UI,
    AddDB,
    AlterDB,
}

#[derive(Parser, Debug)]
#[command(name = "ginger-db")]
#[command(about = "A database composition tool", long_about = None)]
#[command(version, long_about = None)]
struct Args {
    /// name of the command to run
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    // Use globbing
    let tera = get_renderer();
    let db_config_path = Path::new("database.toml");

    match args.command {
        Commands::Init => init::main(tera).await,
        Commands::Up { skip } => up(tera, skip).await,
        Commands::Configure => configure::main(),
        Commands::Render { skip } => {
            // Read the configuration using the read_db_config function
            let db_config = read_consumer_db_config(db_config_path).unwrap();

            let token = get_token_from_file_storage();

            let open_api_config = Configuration {
                base_path: db_config.schema.url.clone(),
                api_key: Some(ApiKey {
                    key: token,
                    prefix: Some("".to_string()),
                }),
                ..Default::default()
            };
            render::main(&open_api_config, db_config, db_config_path, skip).await
        }
        Commands::AlterDB => {
            let mut db_conpose_config = read_db_config("db-compose.toml").unwrap();

            match alter_db(&mut db_conpose_config) {
                Ok(_) => match write_db_config("db-compose.toml", &db_conpose_config) {
                    Ok(_) => {
                        println!("Saved back")
                    }
                    Err(e) => {
                        println!("{:?}", e)
                    }
                },
                Err(e) => {
                    println!("error: {:?}", e);
                }
            };
        }
        Commands::AddDB => {
            let mut db_conpose_config = read_db_config("db-compose.toml").unwrap();

            match add_db(&mut db_conpose_config) {
                Ok(_) => match write_db_config("db-compose.toml", &db_conpose_config) {
                    Ok(_) => {
                        println!("Saved back")
                    }
                    Err(e) => {
                        println!("{:?}", e)
                    }
                },
                Err(e) => {
                    println!("error: {:?}", e);
                }
            };
        }
        Commands::UI => {
            match render_ui().await {
                Ok(_) => {
                    println!("Exited!")
                }
                Err(e) => {
                    println!("Unable to exit the expected way {:?}", e)
                }
            };
        }
    }

    Ok(())
}
