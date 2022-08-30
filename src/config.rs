use std::fs;

use clap::Parser;
//use clap::{Parser, Subcommand};
use merge::Merge;
use serde::{Deserialize, Serialize};

use crate::{
    file_utils::{file_exists, make_dirs_and_write},
    Result,
};

mod default_path {
    use crate::APP_NAME;
    // Builds a path as ~/APP_NAME/child_path
    #[cfg(target_os = "windows")]
    fn make_path_from_home(child_path: &str) -> String {
        return std::path::Path::new(&dirs::home_dir().unwrap())
            .join(APP_NAME)
            .join(child_path)
            .to_str()
            .unwrap()
            .to_string();
    }

    pub fn config_file() -> String {
        #[cfg(target_os = "windows")]
        let cf = make_path_from_home("config.json");
        #[cfg(not(target_os = "windows"))]
        let cf = format!("/etc/{APP_NAME}/config.json");
        return cf;
    }

    pub fn users_file() -> Option<String> {
        #[cfg(target_os = "windows")]
        let uf = make_path_from_home("users.db");
        #[cfg(not(target_os = "windows"))]
        let uf = format!("/etc/{APP_NAME}/users.db");
        return Some(uf);
    }

    pub fn log_dir() -> Option<String> {
        #[cfg(target_os = "windows")]
        let ld = make_path_from_home("logs");
        #[cfg(not(target_os = "windows"))]
        let ld = format!("/var/log/{APP_NAME}/");
        return Some(ld);
    }
}

mod merge_strategy {
    // Overwrites existing Option<T> with donor Option<T> if donor's short, long
    // contains a value, regardless of the existing short, long
    pub fn overwrite_option<T>(existing: &mut Option<T>, donor: Option<T>) {
        if !donor.is_none() {
            *existing = donor;
        }
    }

    pub fn boolean_or(existing: &mut bool, donor: bool) {
        *existing |= donor;
    }
}

#[derive(Debug, Clone, Merge, Serialize, Deserialize, Parser, PartialEq)]
#[clap(author, version, about, long_about = None)]
#[serde(default)]
/// PotatoAuth
pub struct UserConfig {
    /// Address at which to start server
    #[merge(strategy = merge_strategy::overwrite_option)]
    #[clap(short, long)]
    pub address: Option<String>,

    /// Port on which to listen
    #[merge(strategy = merge_strategy::overwrite_option)]
    #[clap(short, long)]
    pub port: Option<u16>,

    /// Time in seconds for inactive sessions to expire
    #[merge(strategy = merge_strategy::overwrite_option)]
    #[clap(skip)]
    pub session_timeout: Option<i64>,

    /// Location of user/password file for this session
    #[merge(strategy = merge_strategy::overwrite_option)]
    #[clap(short, long)]
    pub user_db: Option<String>,

    /// Directory in which to write logs
    #[merge(strategy = merge_strategy::overwrite_option)]
    #[clap(long)]
    pub log_dir: Option<String>,

    /// Number of days to keep old log files
    #[merge(strategy = merge_strategy::overwrite_option)]
    #[clap(skip)]
    pub log_archive_count: Option<usize>,

    /// Write logs to console
    #[serde(skip)]
    #[merge(strategy = merge_strategy::boolean_or)]
    #[clap(long)]
    pub console: bool,

    /// custom config file location
    #[serde(skip)]
    #[merge(strategy = merge_strategy::overwrite_option)]
    #[clap(short, long)]
    pub cfg_path: Option<String>,
}

impl Default for UserConfig {
    fn default() -> Self {
        return UserConfig {
            address: Some("localhost".to_string()),
            port: Some(8675),
            session_timeout: Some(3600),
            user_db: default_path::users_file(),
            log_dir: default_path::log_dir(),
            log_archive_count: Some(5),
            cfg_path: Some(default_path::config_file()), // only used for passing --config via cmdline args
            console: false,
        };
    }
}

impl UserConfig {
    /// Builds a UserConfig by merging json file contents then cmd line args
    /// over the default values
    pub fn new(args: &Vec<String>) -> Result<Self> {
        let default_conf = UserConfig::default();

        let args: UserConfig = UserConfig::parse_from(args);

        let filepath = args.cfg_path.clone().unwrap_or(default_path::config_file());

        if !file_exists(&filepath) {
            match default_conf.write_to_file(&filepath) {
                Ok(_) => {}
                Err(e) => {
                    return Err(e);
                }
            }
        }

        let mut conf: UserConfig;
        match Self::from_file(&filepath) {
            Ok(c) => conf = c,
            Err(e) => return Err(e),
        };

        conf.merge(args);

        return Ok(conf);
    }

    pub fn write_to_file(&self, filepath: &str) -> Result<()> {
        tracing::info!("Writing config to {}", filepath);
        let json = serde_json::to_string_pretty(self).unwrap();
        return make_dirs_and_write(filepath, json);
    }

    /// Deserializes JSON file into new UserConfig. Fields not included in the
    /// text file will use values in Default impl
    fn from_file(filepath: &str) -> Result<Self> {
        tracing::info!("Loading config from {filepath}");
        let json_string: String;
        match fs::read_to_string(filepath) {
            Ok(j) => {
                json_string = j;
            }
            Err(e) => return Err(crate::Error::convert(e)),
        }

        return match serde_json::from_str::<UserConfig>(json_string.as_str()) {
            Ok(mut cfg) => {
                cfg.cfg_path = Some(filepath.to_string());
                return Ok(cfg);
            }
            Err(e) => Err(crate::Error::convert(e)),
        };
    }
}

#[cfg(test)]
pub mod tests {
    use crate::test_utils::make_tmp_file;

    use super::*;

    #[test]
    fn write_read_file() {
        let fp = make_tmp_file();
        std::fs::remove_file(&fp).expect("Can't remove file");
        let mut cfg = UserConfig::default();
        //let mut cfg = make_config();
        cfg.cfg_path = Some(fp.to_owned());

        assert!(cfg.write_to_file(&fp).is_ok());

        let mut cfg2 = UserConfig::from_file(&fp).expect("Can't read file");

        // Copy non-serialized fields
        cfg2.console = cfg.console;

        assert_eq!(cfg, cfg2);
    }

    #[test]
    fn from_args() {
        let args = [
            "_",
            "-a",
            "address",
            "-p",
            "4242",
            "-u",
            "udb",
            "--log-dir",
            "logdir",
            "--console",
            "-c",
            "cfgpath",
        ]
        .iter()
        .map(|x| x.to_string());

        let cfg = UserConfig::parse_from(args);

        assert_eq!(cfg.address, Some("address".to_string()));
        assert_eq!(cfg.port, Some(4242));
        assert_eq!(cfg.user_db, Some("udb".to_string()));
        assert_eq!(cfg.log_dir, Some("logdir".to_string()));
        assert_eq!(cfg.console, true);
        assert_eq!(cfg.cfg_path, Some("cfgpath".to_string()));
    }

    #[test]
    fn merge_args() {
        let fp = make_tmp_file();
        let f = fp.clone();

        let args: Vec<String> = ["_", "-a", "address", "-p", "4242", "-c", &f]
            .iter()
            .map(|x| x.to_string())
            .collect();

        let mut file_cfg = UserConfig::default();
        file_cfg.session_timeout = Some(9999);
        file_cfg.log_dir = Some("a_log_dir".to_string());
        file_cfg.write_to_file(&fp).expect("Can't write to file");

        let merged_cfg = UserConfig::new(&args).unwrap();

        assert_eq!(merged_cfg.address, Some("address".to_string()));
        assert_eq!(merged_cfg.port, Some(4242));
        assert_eq!(merged_cfg.session_timeout, Some(9999));
        assert_eq!(merged_cfg.user_db, file_cfg.user_db);
        assert_eq!(merged_cfg.log_dir, Some("a_log_dir".to_string()));
        assert_eq!(merged_cfg.cfg_path, Some(f.to_string()));
        assert_eq!(merged_cfg.console, false);
    }
}
