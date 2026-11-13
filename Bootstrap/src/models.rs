//! Component inventory and version metadata.

use serde::{Deserialize, Serialize};

const MANIFEST_JSON: &str = include_str!("../Models/fixture-manifest.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    Runtime,
    PackageManager,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: ComponentKind,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub note: String,
    pub components: Vec<Component>,
}

impl Manifest {
    /// Load the bundled synthetic manifest.
    pub fn bundled() -> Result<Manifest, String> {
        serde_json::from_str(MANIFEST_JSON).map_err(|e| format!("fixture-manifest.json: {e}"))
    }

    pub fn get(&self, id: &str) -> Option<&Component> {
        self.components.iter().find(|c| c.id == id)
    }
}
