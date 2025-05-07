use clap::{Parser, Subcommand};
use ginger_shared_rs::{
    read_consumer_db_config, read_db_config, utils::get_token_from_file_storage, write_db_config,
};
use schema_gen_service::apis::configuration::{ApiKey, Configuration};
use serde_json::Result;
use types::WatchContent;
use IAMService::{apis::default_api::identity_validate_api_token, get_configuration};
use std::path::Path;
use templates::get_renderer;
use ui::render_ui;
use up::up;
use utils::{add_db, alter_db};

use tokio::signal;
use tokio_tungstenite::connect_async;
use futures_util::{stream::StreamExt, SinkExt};

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

        /// Watch for render triggers via WebSocket
        #[arg(long)]
        watch: bool,
    },
    /// Start the terminal UI
    UI,
    /// Add a new DB to db-compose.toml
    AddDB,
    /// Alter existing DB setup in db-compose.toml
    AlterDB,
    /// Render models from a saved schema.json file
    RenderFromFile {
        /// Path to the schema.json file
        #[arg(short, long)]
        path: String,

        /// Target directory where models/admin should be generated
        #[arg(short, long)]
        out: String,
    },
}

#[derive(Parser, Debug)]
#[command(name = "ginger-db")]
#[command(about = "A database composition tool", long_about = None)]
#[command(version, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let tera = get_renderer();
    let db_config_path = Path::new("database.toml");

    match args.command {
        Commands::Init => init::main(tera).await,
        Commands::Up { skip } => up(tera, skip).await,
        Commands::Configure => configure::main(),
        Commands::Render { skip, watch } => {
            let db_config = read_consumer_db_config(db_config_path).unwrap();
            let token = get_token_from_file_storage();

            let open_api_config = Configuration {
                base_path: db_config.schema.url.clone(),
                api_key: Some(ApiKey {
                    key: token.clone(),
                    prefix: Some("".to_string()),
                }),
                ..Default::default()
            };

            if watch {
                watch_for_render(&open_api_config, db_config, db_config_path, token).await;
            } else {
                render::main(&open_api_config, db_config, db_config_path, skip).await;
            }
        }
        Commands::UI => {
            match render_ui().await {
                Ok(_) => println!("Exited!"),
                Err(e) => println!("Unable to exit the expected way: {:?}", e),
            };
        }
        Commands::AddDB => {
            let mut db_conpose_config = read_db_config("db-compose.toml").unwrap();

            match add_db(&mut db_conpose_config) {
                Ok(_) => match write_db_config("db-compose.toml", &db_conpose_config) {
                    Ok(_) => println!("Saved back"),
                    Err(e) => println!("{:?}", e),
                },
                Err(e) => println!("error: {:?}", e),
            };
        }
        Commands::AlterDB => {
            let mut db_conpose_config = read_db_config("db-compose.toml").unwrap();

            match alter_db(&mut db_conpose_config) {
                Ok(_) => match write_db_config("db-compose.toml", &db_conpose_config) {
                    Ok(_) => println!("Saved back"),
                    Err(e) => println!("{:?}", e),
                },
                Err(e) => println!("error: {:?}", e),
            };
        }
        Commands::RenderFromFile { path, out } => {
            let schema_str = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("Failed to read schema file at {}", path));
            let schemas: Vec<types::Schema> =
                serde_json::from_str(&schema_str).expect("Invalid schema JSON");

            up::generate_python_files_for_db(&out, &schemas, &tera);
        }
    }

    Ok(())
}


async fn watch_for_render(
    open_api_config: &Configuration,
    db_config: ginger_shared_rs::ConsumerDBConfig,
    db_config_path: &Path,
    token: String,
) {
    use std::{time::Duration, sync::Arc};
    use tokio::time::sleep;

    let tera = get_renderer();
    let db_config_path_buf = db_config_path.to_path_buf();
    let open_api_config = Arc::new(open_api_config.clone());
    let db_config = Arc::new(db_config.clone());

    // 🚀 Initial render before starting to watch
    println!("Performing initial render...");
    render::main(&open_api_config, (*db_config).clone(), &db_config_path_buf, true).await;

    // Validate token and extract sub
    let iam_config = get_configuration(Some(token.clone()));
    let profile_response = match identity_validate_api_token(&iam_config).await {
        Ok(profile) => profile,
        Err(e) => {
            eprintln!("Token validation failed: {:?}", e);
            return;
        }
    };

    let ws_url = format!(
        "wss://api.gingersociety.org/notification/ws/workspace_{}?token={}",
        profile_response.sub, token
    );

    println!("Listening for live updates at: {}", ws_url);

    let mut attempt = 0;

    loop {
        match connect_async(&ws_url).await {
            Ok((ws_stream, _)) => {
                println!("✅ Connected to WebSocket");

                let (mut write, mut read) = ws_stream.split();
                let open_api_config = open_api_config.clone();
                let db_config = db_config.clone();
                let db_config_path = db_config_path_buf.clone();

                // Tasks to handle incoming messages and CTRL+C
                let read_task = tokio::spawn(async move {
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(msg) => {
                                let content = msg.to_string();

                                match serde_json::from_str::<WatchContent>(&content) {
                                    Ok(watch_content) => {
                                        println!("📨 WS Event: {:?}", watch_content);

                                        if watch_content.event.trim().eq_ignore_ascii_case("RENDER") {
                                            if let Some(schema_id) = &db_config.schema.schema_id {
                                                if &watch_content.resource_id == schema_id {
                                                    println!("🔁 Triggering render...");
                                                    render::main(&open_api_config, (*db_config).clone(), &db_config_path, true).await;
                                                } else {
                                                    println!("❌ Resource ID mismatch: expected {}, got {}", schema_id, watch_content.resource_id);
                                                }
                                            } else {
                                                println!("⚠️ schema_id missing in db_config");
                                            }
                                        } else {
                                            println!("⚠️ Unhandled event type: {}", watch_content.event);
                                        }
                                    }
                                    Err(_) => println!("💬 Non-JSON WS message: {}", content),
                                }
                            }
                            Err(e) => {
                                eprintln!("❌ WebSocket read error: {}", e);
                                break;
                            }
                        }
                    }
                });

                let signal_task = tokio::spawn(async move {
                    signal::ctrl_c().await.expect("Failed to listen for ctrl_c");
                    let _ = write.close().await;
                });

                tokio::select! {
                    _ = read_task => println!("🔌 WebSocket stream ended"),
                    _ = signal_task => {
                        println!("🛑 Shutting down on CTRL+C");
                        return;
                    }
                }

                // If we reach here, connection dropped; try to reconnect
                attempt = 0; // reset retry backoff on a successful session
                println!("🔄 Reconnecting...");
            }
            Err(e) => {
                attempt += 1;
                let delay = Duration::from_secs((2_u64).pow(attempt.min(5))); // exponential backoff up to 32s
                eprintln!("❌ Failed to connect (attempt {}): {}, retrying in {:?}...", attempt, e, delay);
                sleep(delay).await;
            }
        }
    }
}
