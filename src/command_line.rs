//! Command-line interface for the orchestrator daemon.

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;

use crate::OrchestratorConfiguration;

#[derive(Debug, Clone, Parser)]
#[command(author, version, about)]
pub struct CommandLine {
    #[arg(long, default_value = ".")]
    city: PathBuf,

    #[arg(long)]
    state: Option<PathBuf>,

    #[arg(long, default_value_t = 5)]
    idle_sleep_seconds: u64,

    #[arg(long)]
    once: bool,
}

impl CommandLine {
    pub fn into_configuration(self) -> OrchestratorConfiguration {
        let idle_sleep = Duration::from_secs(self.idle_sleep_seconds);
        match self.state {
            Some(state_path) => {
                OrchestratorConfiguration::new(self.city, state_path, idle_sleep, self.once)
            }
            None => {
                OrchestratorConfiguration::with_default_state_path(self.city, idle_sleep, self.once)
            }
        }
    }
}
