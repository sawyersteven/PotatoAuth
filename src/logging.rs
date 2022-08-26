use rolling_file::{BasicRollingFileAppender, RollingConditionBasic};
use tracing::metadata::LevelFilter;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::{
    filter::Filtered,
    fmt::{
        format::{DefaultFields, Format, Pretty},
        Layer,
    },
    prelude::*,
    reload::{self, Handle},
    Registry,
};

use crate::file_utils;

pub struct Logging {
    stdout_handle:
        Handle<Option<Filtered<Layer<Registry, Pretty, Format<Pretty>>, LevelFilter, Registry>>, Registry>,
    rolling_file_handle: Handle<Option<Layer<Registry, DefaultFields, Format, NonBlocking>>, Registry>,
    file_write_guard: Option<WorkerGuard>,
}

impl Logging {
    /// Starts tracing logger that writes to stdout
    pub fn new() -> Self {
        let mut layers = Vec::new();

        let (stdout_layer, stdout_reload) = reload::Layer::new(Some(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_filter(tracing::level_filters::LevelFilter::INFO),
        ));

        layers.push(stdout_layer.boxed());

        let opt: Option<Layer<Registry, DefaultFields, Format, NonBlocking>> = None;

        let (rolling_layer, rolling_reload) = reload::Layer::new(opt);
        let rld_layer = rolling_layer.with_filter(tracing::level_filters::LevelFilter::INFO);

        layers.push(rld_layer.boxed());

        tracing_subscriber::registry().with(layers).init();

        return Logging {
            stdout_handle: stdout_reload,
            rolling_file_handle: rolling_reload,
            file_write_guard: None,
        };
    }

    pub fn start_file_log(&mut self, directory: &String, max_files: usize, keep_stdout: bool) {
        let fp = std::path::Path::new(directory).join(format!("{}.{}", crate::APP_NAME, "log"));

        match file_utils::make_parent_dirs(&fp) {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Unable to create log directory: {}", e);
            }
        };

        let file_appender =
            BasicRollingFileAppender::new(&fp, RollingConditionBasic::new().daily(), max_files).unwrap();

        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        self.file_write_guard = Some(guard);

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false);

        tracing::info!("Starting file logging at {}", fp.display());

        if !keep_stdout {
            _ = self.stdout_handle.modify(|l| *l = None);
        }

        self.rolling_file_handle.modify(|l| *l = Some(file_layer)).unwrap();
    }
}
