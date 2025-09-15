use std::{fs::create_dir_all, path::Path};

use definitions::Parameters;

use crate::util::serde::{deserialize_from_json_file, serialize_to_json_file_pretty};

const PARAMETERS_FILE: &str = "parameters.json";
const PARAMETERS_BACKUP_FILE: &str = "parameters.json.bak";


pub fn load_parameters_from_disk(data_dir: &Path) -> Parameters {
    let parameters_file = &data_dir.join(PARAMETERS_FILE);
    match deserialize_from_json_file(parameters_file) {
        Ok(parameters) => parameters,
        Err(e) => {
            let parameters_backup_file = &data_dir.join(PARAMETERS_BACKUP_FILE);
            log::error!("Could not read parameters from {parameters_file:?}, using default parameters: {e}");
            if parameters_file.exists() {
                log::error!("Backupping {parameters_file:?} file to {parameters_backup_file:?}...");
                if let Err(e) = std::fs::rename(parameters_file, parameters_backup_file) {
                    log::error!("Could not create backup {parameters_backup_file:?}: {e:?}");
                }
            }
            Parameters::default()
        }
    }
}

pub fn save_parameters_to_disk(parameters: &Parameters, data_dir: &Path) {
    if let Err(e) = create_dir_all(data_dir) {
        log::error!("Could not create data directory {data_dir:?}: {e:?}");
        return;
    }
    let parameters_file = &data_dir.join(PARAMETERS_FILE);
    if let Err(e) = serialize_to_json_file_pretty(parameters, parameters_file) {
        log::error!("Could not serialize and save parameters to {parameters_file:?}: {e:?}");
    }
}
