use clap::Parser;
use clap::Subcommand;

use schemaClient::apis::configuration::Configuration;
use serde_json::Result;
use std::path::Path;
use tera::Tera;
use utils::read_db_config;

mod configure;
mod init;
mod render;
mod types;
mod up;
mod utils;

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

fn main() -> Result<()> {
    let args = Args::parse();
    // Use globbing
    let tera = match Tera::new("templates/**/*.tpl") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    let db_config_path = Path::new("database.toml");

    // Read the configuration using the read_db_config function
    let mut db_config = read_db_config(db_config_path).unwrap();

    let open_api_config = Configuration {
        base_path: db_config.schema.url.clone(),
        ..Default::default()
    };

    match args.command {
        Commands::Init => init::main(tera),
        Commands::Up => up::main(tera),
        Commands::Configure => configure::main(),
        Commands::Render => render::main(&open_api_config, db_config, db_config_path),
    }

    Ok(())
}
