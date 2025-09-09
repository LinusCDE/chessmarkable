use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Savestates {
    pub slot_1: Option<String>,
    pub slot_2: Option<String>,
    pub slot_3: Option<String>,
}

impl Default for Savestates {
    fn default() -> Self {
        Self {
            slot_1: None,
            slot_2: None,
            slot_3: None,
        }
    }
}

pub fn read() -> Result<Savestates> {
    let ref file_path = crate::CLI_OPTS.savestates_file;

    if !file_path.exists() {
        info!("Savestates file doesn't exist (yet).");
        Ok(Savestates::default())
    } else {
        let file = std::fs::File::open(file_path).context("Open file")?;
        let savestates: Savestates = serde_yaml::from_reader(file).context("Deserialize file")?;
        Ok(savestates)
    }
}

pub fn write(savestates: &Savestates) -> Result<()> {
    let directory = crate::CLI_OPTS
        .savestates_file
        .parent()
        .ok_or(anyhow!("No parent directory"))?;
    if !directory.exists() {
        std::fs::create_dir_all(directory).context("Create directory for file")?;
        info!("Created directory for savestate file.");
    }

    let file = std::fs::File::create(&crate::CLI_OPTS.savestates_file).context("Create file")?;
    serde_yaml::to_writer(file, savestates).context("Serialize and writing file")
}
