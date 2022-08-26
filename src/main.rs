#![forbid(unsafe_code)]
use std::fmt::Display;

use config::UserConfig;

use crate::logging::Logging;

mod app;
mod config;
mod file_utils;
mod logging;
mod middleware;
mod routes;
mod sessions;
mod shared_data;
mod userdb;

const APP_NAME: &str = "PotatoAuth";

#[tokio::main]
async fn main() {
    let mut l = Logging::new();
    let args: Vec<String> = std::env::args().map(|x| x.to_string()).collect();
    let cfg = match UserConfig::new(&args) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Unable to create config: {}", e);
            std::process::exit(1);
        }
    };

    l.start_file_log(
        cfg.log_dir.as_ref().unwrap(),
        *cfg.log_archive_count.as_ref().unwrap(),
        cfg.console,
    );

    app::run_server(cfg).await;
}

/*
General purpose error and result type
*/

#[macro_export]
/// Creates an Err() with message
macro_rules! err {
    ($($arg:tt)*) => {{
        Err(crate::Error::new(format!($($arg)*)))
    }};
}

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    pub fn new<T>(message: T) -> Self
    where
        T: ToString,
    {
        return Error {
            message: message.to_string(),
        };
    }

    // Convert any impl of std::error::Error to crate::Error
    pub fn convert(err: impl std::error::Error) -> Self {
        return Error {
            message: err.to_string(),
        };
    }
}

impl Default for Error {
    fn default() -> Self {
        Self {
            message: Default::default(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

#[allow(unused)]
#[cfg(test)]
mod test_utils {
    use rand::{distributions::Alphanumeric, Rng};
    use std::path::PathBuf;

    fn tmp_path() -> PathBuf {
        let mut rndm = rand::thread_rng();
        let tmp_dir = std::env::temp_dir();

        let id: String = (0..16).map(|_| rndm.sample(Alphanumeric) as char).collect();
        let mut tmp_file = tmp_dir.join(format!("test_file_{id}"));

        loop {
            if !tmp_file.exists() {
                break;
            }
            let id: String = (0..16).map(|_| rndm.sample(Alphanumeric) as char).collect();
            tmp_file = tmp_dir.join(format!("test_file_{id}"));
        }
        return tmp_file;
    }

    pub fn make_tmp_file() -> String {
        let path = tmp_path();
        std::fs::File::create(&path).expect(format!("Can't make file {:#?}", path).as_str());
        return path.into_os_string().into_string().expect("");
    }

    pub fn make_tmp_dir() -> String {
        let path = tmp_path();
        std::fs::create_dir_all(&path).expect(format!("Can't make dir {:#?}", path).as_str());
        return path.into_os_string().into_string().expect("");
    }
}
