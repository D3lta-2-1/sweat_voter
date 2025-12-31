#[cfg(feature = "server")]
mod app_state;
#[cfg(feature = "server")]
mod commands;
mod common;
#[cfg(feature = "server")]
mod data_server;
mod ui;

#[cfg(feature = "server")]
use {
    crate::app_state::{AppState, CommandOutput, Commands, SaveFormat},
    axum::Extension,
    std::fs::File,
    std::io::stdin,
    std::net::{IpAddr, Ipv4Addr, SocketAddr},
    std::time::Duration,
    structopt::clap::AppSettings,
    structopt::StructOpt,
};

use crate::common::Credentials;
use crate::ui::App;
use dioxus::prelude::*;
use dioxus_fullstack::form::Form;
use serde::{Deserialize, Serialize};
use tracing::info;

extern crate tracing;

#[cfg(feature = "server")]
async fn save_loop(state: AppState, duration: Duration) {
    let mut interval = tokio::time::interval(duration);
    loop {
        interval.tick().await;
        state.save()
    }
}

#[cfg(feature = "server")]
fn wait_for_cmd_input(state: AppState) {
    let mut command = String::new();
    loop {
        // read stdin
        command.clear();
        if let Err(e) = stdin().read_line(&mut command) {
            println!("{}", e);
            continue;
        }

        // parse the command
        let Some(inputs) = shlex::split(&command) else {
            println!("this command could not be parsed, check your quotes");
            continue;
        };

        let clap = Commands::clap().setting(AppSettings::NoBinaryName);
        let command = clap.get_matches_from_safe(inputs.iter().map(|input| input.trim()));
        let command = match command {
            Ok(command) => Commands::from_clap(&command),
            Err(e) => {
                println!("{}", e);
                continue;
            }
        };

        if let Commands::Exit = command {
            return;
        }

        let result = state.execute_command(None, command);
        match result {
            Ok(CommandOutput { message: None, .. }) => println!("action performed successfully!"),
            Ok(CommandOutput {
                message: Some(result),
                ..
            }) => println!("{}", result.trim()),
            Err(e) => println!("{}", e),
        }
    }
}

#[cfg(feature = "server")]
#[derive(Serialize, Deserialize)]
struct ServerConfig {
    address: SocketAddr,
    save_intervals: Duration,
    save_format: SaveFormat,
}

#[cfg(feature = "server")]
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3000),
            save_intervals: Duration::from_secs(300),
            save_format: SaveFormat::Cbor,
        }
    }
}

/// Login function, return true on success
#[post("/api/login", mut auth_session: AuthSession)]
pub async fn login(Form(creds): Form<Credentials>) -> Result<bool> {
    println!("try to log with {:?} logged", creds);

    let user = match auth_session.authenticate(creds.clone()).await? {
        Some(user) => user,
        None => return Ok(false),
    };

    if auth_session.login(&user).await.is_err() {
        return Ok(false);
    }
    println!("success");
    Ok(true)
}

#[post("/api/list_classes", state: Extension<AppState>)]
pub async fn list_classes() -> Result<Vec<String>> {
    let state: AppState = state.0;
    let server = state.data_server.lock().unwrap();
    Ok(server.list_classes())
}

#[cfg(feature = "server")]
type AuthSession = axum_login::AuthSession<AppState>;

#[cfg(feature = "web")]
fn main() {
    dioxus::launch(App);
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() -> std::io::Result<()> {
    use axum_login::AuthManagerLayerBuilder;
    use tower_sessions::{MemoryStore, SessionManagerLayer};

    let Ok(file) = File::open("config.json") else {
        let config = File::create("config.json").expect("failed to create config");
        serde_json::to_writer_pretty(config, &ServerConfig::default())?;
        info!("Config created");
        return Ok(());
    };
    let config: ServerConfig = serde_json::from_reader(file)?;
    let state = AppState::new(config.save_format);

    // Session layer.
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store);

    // Auth service.
    let auth_layer = AuthManagerLayerBuilder::new(state.clone(), session_layer).build();

    let address = dioxus::cli_config::fullstack_address_or_localhost();

    tokio::spawn(save_loop(state.clone(), config.save_intervals));

    // TODO: this doesn't work well with dioxus CLI
    let signal = async |_state| {
        //spawn_blocking(move || wait_for_cmd_input(state))
        tokio::signal::ctrl_c().await.unwrap();
    };

    let router = dioxus::server::router(App)
        .layer(auth_layer)
        .layer(Extension(state.clone()));

    let router = router.into_make_service();
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(signal(state.clone()))
        .await?;

    state.save();
    Ok(())
}
