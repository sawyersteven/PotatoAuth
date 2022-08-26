use crate::config::UserConfig;

use crate::sessions::SessionStore;
use crate::shared_data::Sharable;
use crate::{routes, userdb};

use actix_web::cookie::time::Duration;
use actix_web::dev::{Server, ServerHandle};

use actix_web::{web, App, HttpServer};

use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;

pub async fn run_server(mut cfg: UserConfig) {
    loop {
        let exit_command = start_server(cfg.clone()).await;
        tracing::error!("Got exit command {:#?}", exit_command);
        match exit_command {
            ExitCommand::Restart => {
                tracing::error!("Restarting server");
                match UserConfig::new(&std::env::args().map(|x| x.to_string()).collect()) {
                    Ok(c) => {
                        cfg = c;
                    }
                    Err(e) => {
                        tracing::error!("Unable to reload config: {e}");
                        tracing::warn!("Server will restart with previous valid config");
                    }
                }
                continue;
            }
            ExitCommand::Quit => {
                tracing::error!("Quitting server");
                break;
            }
        }
    }
}

async fn start_server(cfg: UserConfig) -> ExitCommand {
    // Set up shared app_data
    let (signal_sender, signal_recv) = mpsc::channel::<(ExitCommand, bool)>(1);
    let signaller = ServerController::new(signal_sender).to_sharable();

    let session_timeout = Duration::seconds(*cfg.session_timeout.as_ref().unwrap());
    let session_store = SessionStore::new(session_timeout).to_sharable();

    let user_db = match userdb::UserDB::new(cfg.user_db.as_ref().unwrap()) {
        Ok(u) => u.to_sharable(),
        Err(e) => {
            tracing::error!("{}", e);
            return ExitCommand::Quit;
        }
    };

    // Spin up server in setup or normal operating mode
    let addr = (cfg.address.as_ref().unwrap().as_str(), *cfg.port.as_ref().unwrap());
    let srv: Server;
    if user_db.read().unwrap().count() == 0 {
        tracing::info!(
            "Staring server in limited setup mode. Go to {}:{}/setup to create an administrator account",
            addr.0,
            addr.1
        );
        srv = HttpServer::new(move || {
            App::new()
                .app_data(signaller.clone())
                .app_data(user_db.clone())
                .route("/static/{a}", web::get().to(routes::static_dir::get))
                .route("/setup", web::get().to(routes::setup::get))
                .route("/setup", web::post().to(routes::setup::post))
        })
        .bind(addr)
        .unwrap()
        .run();
    } else {
        srv = HttpServer::new(move || {
            App::new()
                .app_data(signaller.clone())
                .app_data(user_db.clone())
                .app_data(session_store.clone())
                .route("/static/{file}", web::get().to(routes::static_dir::get))
                .route("/login", web::get().to(routes::login::get))
                .route("/login", web::post().to(routes::login::post))
                .route("/logout", web::get().to(routes::logout::get))
                .route("/admin", web::get().to(routes::admin::get))
                .route("/authrequest", web::get().to(routes::auth_request::get))
                .route("/rpc/{command}", web::post().to(routes::rpc::post))
                .route("/rpc/{command}", web::get().to(routes::rpc::get))
        })
        .bind(addr)
        .unwrap()
        .run();
    }

    let handle = srv.handle();
    let task = tokio::spawn(srv);

    let exit_sig = wait_for_exit_signal(signal_recv, handle).await;
    _ = tokio::join!(task);
    tracing::error!("Returning {:#?}", exit_sig);
    return exit_sig;
}

async fn wait_for_exit_signal(mut rx: Receiver<(ExitCommand, bool)>, handle: ServerHandle) -> ExitCommand {
    match rx.recv().await {
        Some((command, force)) => {
            if force {
                tracing::info!("Forcefully stopping server");
                handle.stop(false).await;
            } else {
                tracing::info!("Requesting server shutdown");
                handle.stop(true).await;
            };
            return command;
        }
        None => {
            tracing::error!("Failed to receive exit signal. Forcefully shutting down server...");
            return ExitCommand::Quit;
        }
    }
}

pub struct ServerController {
    exit_signal: tokio::sync::mpsc::Sender<(ExitCommand, bool)>,
}

impl ServerController {
    pub fn new(exit_sig: tokio::sync::mpsc::Sender<(ExitCommand, bool)>) -> Self {
        return ServerController { exit_signal: exit_sig };
    }

    pub async fn send_exit(&self, command: ExitCommand, force: bool) {
        _ = self.exit_signal.send((command, force)).await;
    }
}

impl Sharable for ServerController {
    type Shared = ServerController;

    fn to_sharable(self) -> actix_web::web::Data<Self::Shared> {
        return actix_web::web::Data::new(self);
    }
}

#[derive(Debug, Deserialize)]
pub enum ExitCommand {
    /// Restarts server from scratch, reloading config and userdb
    Restart,
    /// Exits server, ending server task
    Quit,
}

#[cfg(test)]
pub mod tests {
    use http::StatusCode;
    use reqwest;
    use tokio::task::JoinHandle;

    use super::*;
    use crate::routes::tests::make_test_userdb;
    use crate::{config::UserConfig, test_utils::make_tmp_file, Error};

    pub fn start_test_server(cfg: UserConfig) -> JoinHandle<ExitCommand> {
        return tokio::task::spawn(super::start_server(cfg));
    }

    #[tokio::test]
    async fn server_setup_mode() -> crate::Result<()> {
        const PORT: u16 = 8674;

        let udb = make_tmp_file();
        let mut cfg = UserConfig::default();
        cfg.port = Some(8674);
        cfg.user_db = Some(udb);
        start_test_server(cfg);

        match reqwest::get(format!("http://localhost:{}/setup", PORT)).await {
            Ok(r) => assert_eq!(r.status(), StatusCode::OK),
            Err(e) => return Err(Error::convert(e)),
        };

        match reqwest::get(format!("http://localhost:{}/auth_request", PORT)).await {
            Ok(r) => assert_eq!(r.status(), StatusCode::NOT_FOUND),
            Err(e) => return Err(Error::convert(e)),
        };

        return Ok(());
    }

    #[tokio::test]
    async fn server_reg_mode() -> crate::Result<()> {
        const PORT: u16 = 8673;

        let udb = make_test_userdb();
        let mut cfg = UserConfig::default();
        cfg.port = Some(PORT);
        cfg.user_db = Some(udb);
        start_test_server(cfg);

        match reqwest::get(format!("http://localhost:{}/setup", PORT)).await {
            Ok(r) => assert_eq!(r.status(), StatusCode::NOT_FOUND),
            Err(e) => return Err(Error::convert(e)),
        };

        match reqwest::get(format!("http://localhost:{}/admin", PORT)).await {
            Ok(r) => assert_eq!(r.status(), StatusCode::OK),
            Err(e) => return Err(Error::convert(e)),
        };

        return Ok(());
    }
}
