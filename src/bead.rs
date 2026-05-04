//! Typed cascade view over `gc bd show --json`.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use serde_json::Value;

use crate::{AgentName, BeadId, CascadeId, Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CascadeBead {
    bead_id: BeadId,
    labels: BTreeSet<String>,
    metadata: BTreeMap<String, String>,
    status: Option<String>,
}

impl CascadeBead {
    pub fn new(
        bead_id: BeadId,
        labels: impl IntoIterator<Item = String>,
        metadata: impl IntoIterator<Item = (String, String)>,
        status: Option<String>,
    ) -> Self {
        Self {
            bead_id,
            labels: labels.into_iter().collect(),
            metadata: metadata.into_iter().collect(),
            status,
        }
    }

    pub fn from_show_json(show_json: &str) -> Result<Self> {
        let mut documents: Vec<BeadDocument> = serde_json::from_str(show_json)?;
        let document = documents.drain(..).next().ok_or(Error::EmptyBeadResponse)?;
        document.into_cascade_bead()
    }

    pub fn bead_id(&self) -> &BeadId {
        &self.bead_id
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn has_cascade_chain_label(&self) -> bool {
        self.labels.contains("cascade-chain")
    }

    pub fn has_order_tracking_label(&self) -> bool {
        self.labels.contains("order-tracking") || self.labels.contains("gc:order-tracking")
    }

    pub fn is_dispatchable(&self) -> bool {
        self.has_cascade_chain_label() && !self.has_order_tracking_label()
    }

    pub fn cascade_next(&self) -> Result<Option<BeadId>> {
        self.metadata_field("cascade_next")
            .filter(|value| !value.trim().is_empty())
            .map(BeadId::new)
            .transpose()
    }

    pub fn cascade_id(&self) -> Result<Option<CascadeId>> {
        self.metadata_field("cascade_id")
            .filter(|value| !value.trim().is_empty())
            .map(CascadeId::new)
            .transpose()
    }

    pub fn cascade_id_or_bead_id(&self) -> Result<CascadeId> {
        self.cascade_id()
            .map(|cascade_id| cascade_id.unwrap_or_else(|| CascadeId::from_bead_id(&self.bead_id)))
    }

    pub fn is_final(&self) -> bool {
        self.metadata_field("cascade_final") == Some("true")
    }

    pub fn position(&self) -> Result<Option<u64>> {
        self.metadata_field("cascade_position")
            .filter(|value| !value.trim().is_empty())
            .map(|value| {
                value.parse::<u64>().map_err(|_| Error::InvalidMetadata {
                    bead_id: self.bead_id.to_string(),
                    field: "cascade_position",
                    value: value.to_owned(),
                })
            })
            .transpose()
    }

    pub fn routed_to(&self) -> Result<Option<AgentName>> {
        self.metadata_field("gc.routed_to")
            .filter(|value| !value.trim().is_empty())
            .map(AgentName::new)
            .transpose()
    }

    pub fn required_routed_to(&self) -> Result<AgentName> {
        self.routed_to()?
            .ok_or_else(|| Error::MissingCascadeTarget {
                bead_id: self.bead_id.to_string(),
            })
    }

    fn metadata_field(&self, field: &str) -> Option<&str> {
        self.metadata.get(field).map(String::as_str)
    }
}

#[derive(Debug, Deserialize)]
struct BeadDocument {
    id: String,
    labels: Option<Vec<String>>,
    metadata: Option<BTreeMap<String, Value>>,
    status: Option<String>,
}

impl BeadDocument {
    fn into_cascade_bead(self) -> Result<CascadeBead> {
        let metadata = self
            .metadata
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(field, value)| MetadataValue::new(value).into_field(field))
            .collect::<BTreeMap<_, _>>();

        Ok(CascadeBead::new(
            BeadId::new(self.id)?,
            self.labels.unwrap_or_default(),
            metadata,
            self.status,
        ))
    }
}

struct MetadataValue {
    value: Value,
}

impl MetadataValue {
    fn new(value: Value) -> Self {
        Self { value }
    }

    fn into_field(self, field: String) -> Option<(String, String)> {
        match self.value {
            Value::Null => None,
            Value::String(value) => Some((field, value)),
            Value::Bool(value) => Some((field, value.to_string())),
            Value::Number(value) => Some((field, value.to_string())),
            other_value => Some((field, other_value.to_string())),
        }
    }
}
