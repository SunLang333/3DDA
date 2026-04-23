use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CsvItemTemplate {
    pub id: String,
    pub type_: String,
    pub name: String,
    pub weight: String,
    pub volume: String,
}

pub fn load_items_csv(path: impl AsRef<Path>) -> Result<Vec<CsvItemTemplate>> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(path)?;
    let mut templates = Vec::new();
    for line in content.lines().skip(1) {
        let mut columns = line.split(',');
        let id = columns.next().unwrap_or_default();
        let type_ = columns.next().unwrap_or_default();
        let name = columns.next().unwrap_or_default();
        let weight = columns.next().unwrap_or_default();
        let volume = columns.next().unwrap_or_default();
        if id.is_empty() && type_.is_empty() && name.is_empty() {
            continue;
        }
        templates.push(CsvItemTemplate {
            id: id.to_string(),
            type_: type_.to_string(),
            name: name.to_string(),
            weight: weight.to_string(),
            volume: volume.to_string(),
        });
    }

    Ok(templates)
}
