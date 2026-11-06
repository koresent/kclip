use crate::error::CoreError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ItemId(Uuid);

impl ItemId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ItemId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ItemId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl FromStr for ItemId {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::parse_str(s).map_err(|e| CoreError::InvalidId(e.to_string()))?;
        Ok(Self(uuid))
    }
}
