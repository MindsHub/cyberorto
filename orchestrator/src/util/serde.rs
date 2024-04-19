use std::{fs::File, path::Path};

use serde::{de::DeserializeOwned, Serialize};

pub fn serialize_to_json_file<T: Serialize>(v: &T, file: &Path) -> Result<(), String> {
    let file = File::create(file).map_err(|e| e.to_string())?;
    serde_json::to_writer(file, v).map_err(|e| e.to_string())
}

pub fn deserialize_from_json_file<T: DeserializeOwned>(file: &Path) -> Result<T, String> {
    let file = File::open(file).map_err(|e| e.to_string())?;
    serde_json::from_reader(file).map_err(|e| e.to_string())
}
