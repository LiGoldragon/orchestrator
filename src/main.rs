//! orchestrator daemon binary entry point.

use clap::Parser;
use orchestrator::{CommandLine, Orchestrator, Result};

fn main() -> Result<()> {
    let command_line = CommandLine::parse();
    let configuration = command_line.into_configuration();
    Orchestrator::new(configuration)?.run()
}
