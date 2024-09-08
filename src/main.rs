use clap::{Arg, Parser, Subcommand};

use schema_gen_service::apis::configuration::Configuration;
use serde_json::Result;
use std::path::Path;
use templates::get_renderer;
use ui::render_ui;
use up::up;
use utils::read_db_config;
use utils_v2::{add_db, alter_db, read_config, write_config};

mod configure;
mod init;
mod render;
mod templates;
mod types;
mod ui;
mod up;
mod utils;
mod utils_v2;

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a database project
    Init,
    /// Bring up the database up just like docker-compose
    Up,
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
        Commands::Up => up(tera).await,
        Commands::Configure => configure::main(),
        Commands::Render { skip } => {
            // Read the configuration using the read_db_config function
            let db_config = read_db_config(db_config_path).unwrap();

            let open_api_config = Configuration {
                base_path: db_config.schema.url.clone(),
                ..Default::default()
            };
            render::main(&open_api_config, db_config, db_config_path, skip).await
        }
        Commands::AlterDB => {
            let mut db_conpose_config = read_config("db-compose.toml").unwrap();

            alter_db(&mut db_conpose_config);

            match write_config("db-compose.toml", &db_conpose_config) {
                Ok(_) => {
                    println!("Saved back")
                }
                Err(e) => {
                    println!("{:?}", e)
                }
            }
        }
        Commands::AddDB => {
            let mut db_conpose_config = read_config("db-compose.toml").unwrap();

            add_db(&mut db_conpose_config);

            match write_config("db-compose.toml", &db_conpose_config) {
                Ok(_) => {
                    println!("Saved back")
                }
                Err(e) => {
                    println!("{:?}", e)
                }
            }
        }
        Commands::UI => {
            render_ui();
        }
    }

    Ok(())
}
