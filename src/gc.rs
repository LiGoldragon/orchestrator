//! Typed wrapper around the `gc` CLI.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{AgentName, BeadId, CascadeBead, CascadeId, Error, EventBatch, EventSequence, Result};

#[derive(Debug, Clone)]
pub struct GcClient {
    city_path: PathBuf,
}

impl GcClient {
    pub fn new(city_path: impl Into<PathBuf>) -> Self {
        Self {
            city_path: city_path.into(),
        }
    }

    pub fn city_path(&self) -> &Path {
        &self.city_path
    }

    pub fn current_sequence(&self) -> Result<EventSequence> {
        let output = self.output(["events", "--seq"])?;
        let sequence = output
            .trim()
            .parse::<u64>()
            .map_err(|_| Error::InvalidMetadata {
                bead_id: "gc events".to_owned(),
                field: "seq",
                value: output,
            })?;
        Ok(EventSequence::new(sequence))
    }

    pub fn events_after(&self, sequence: EventSequence) -> Result<EventBatch> {
        let output = self.output(["events", "--after", &sequence.value().to_string()])?;
        EventBatch::from_json_lines(&output)
    }

    pub fn bead(&self, bead_id: &BeadId) -> Result<CascadeBead> {
        let output = match self.output(["bd", "show", bead_id.as_str(), "--json"]) {
            Ok(output) => output,
            Err(error) if error.is_missing_bead_command() => {
                return Err(Error::MissingBead {
                    bead_id: bead_id.to_string(),
                });
            }
            Err(error) => return Err(error),
        };
        CascadeBead::from_show_json(&output)
    }

    pub fn sling(&self, target_agent: &AgentName, bead_id: &BeadId) -> Result<()> {
        self.output(["sling", target_agent.as_str(), bead_id.as_str()])?;
        Ok(())
    }

    pub fn mail_cascade_complete(
        &self,
        cascade_id: &CascadeId,
        final_bead_id: &BeadId,
    ) -> Result<()> {
        let subject = format!("cascade complete: {cascade_id}");
        let message = format!("Final bead {final_bead_id} closed.");
        self.output([
            "mail",
            "send",
            "--notify",
            "mayor",
            "-s",
            subject.as_str(),
            "-m",
            message.as_str(),
        ])?;
        Ok(())
    }

    fn output<const ARGUMENT_COUNT: usize>(
        &self,
        arguments: [&str; ARGUMENT_COUNT],
    ) -> Result<String> {
        let output = Command::new("gc")
            .arg("--city")
            .arg(&self.city_path)
            .args(arguments)
            .output()?;

        if output.status.success() {
            String::from_utf8(output.stdout).map_err(Error::from)
        } else {
            Err(Error::GcCommandFailed {
                command: format!(
                    "gc --city {} {}",
                    self.city_path.display(),
                    arguments.join(" ")
                ),
                status: output.status.code(),
                stderr: String::from_utf8(output.stderr)?,
            })
        }
    }
}
