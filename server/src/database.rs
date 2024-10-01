use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{bail, Result};
use common::Job;
use serde::{de::DeserializeOwned, Serialize};

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct PackRatSaveData {
    queue: Vec<PackRatJobState>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PackRatJobState {
    job: Job,
    is_complete: bool,
}

pub struct PackRatDatabase {
    save: PackRatSaveData,
    base_path: PathBuf,
    archive_path: PathBuf,
    save_data_path: PathBuf,
}

impl PackRatDatabase {
    /// Loads an existing database at the given path, or creates a new one.
    pub fn new(path: PathBuf) -> Result<Self> {
        if path.is_file() {
            bail!(
                "Expected database directory; found file at {}",
                path.display()
            );
        }

        let archive_path = path.join("archive");
        let save_data_path = path.join("save.dat");
        let base_path = path;

        if !base_path.is_dir() {
            std::fs::create_dir(&base_path)?;
            std::fs::create_dir(&archive_path)?;
        }

        let save = match load_from_file(save_data_path.clone()) {
            Ok(save) => save,
            Err(e)
                if e.downcast_ref::<std::io::Error>()
                    .filter(|e| e.kind() == std::io::ErrorKind::NotFound)
                    .is_some() =>
            {
                let save = PackRatSaveData::default();
                save_to_file(save_data_path.clone(), &save)?;
                save
            }
            Err(other) => return Err(other.into()),
        };

        Ok(Self {
            base_path,
            archive_path,
            save_data_path,
            save,
        })
    }

    /// Atomically writes the current database state to disk
    pub fn save_to_disk(&self) -> Result<()> {
        save_to_file(self.save_data_path.clone(), self.savedata()).map_err(|e| e.into())
    }

    pub fn savedata(&self) -> &PackRatSaveData {
        &self.save
    }

    pub fn savedata_mut(&mut self) -> &mut PackRatSaveData {
        &mut self.save
    }

    /*
    /// Returns a list of archive "pages" named by their dates.
    pub fn get_log(&self) -> Vec<String> {
    std::fs::read_dir(self.archive_path)
    }

    pub fn get_archive_page(&self, page: String) -> Vec<Job> {}
    */
}

impl Drop for PackRatDatabase {
    fn drop(&mut self) {
        let _ = self.save_to_disk();
    }
}

pub async fn autosave(db: Arc<tokio::sync::Mutex<PackRatDatabase>>, interval: Duration) -> Result<()> {
    loop {
        db.lock().await.save_to_disk()?;
        tokio::time::sleep(interval).await;
    }
}

fn load_from_file<T: DeserializeOwned>(path: PathBuf) -> Result<T> {
    Ok(framework::io::decode(&std::fs::read(path)?)?)
}

fn save_to_file<T: Serialize>(path: PathBuf, value: &T) -> Result<()> {
    Ok(std::fs::write(path, framework::io::encode(value)?)?)
}
