//! Independent, fsynced acknowledgement watermark. Never roll it back with a DB backup.
use crate::{DurableHead, StorageError};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static TEMPORARY: AtomicU64 = AtomicU64::new(0);
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub schema_version: u32,
    pub world_key: String,
    pub namespace: String,
    pub installation: String,
    pub head: DurableHead,
}
pub(crate) struct Anchor {
    path: PathBuf,
    lock: File,
}
impl Anchor {
    pub fn acquire(path: &Path) -> Result<Self, StorageError> {
        if !path.is_absolute() {
            return Err(StorageError::Binding);
        }
        let parent = path.parent().ok_or(StorageError::Binding)?;
        std::fs::create_dir_all(parent).map_err(io)?;
        let mut lock_name = path.as_os_str().to_owned();
        lock_name.push(".lock");
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(PathBuf::from(lock_name))
            .map_err(io)?;
        lock.try_lock_exclusive()
            .map_err(|_| StorageError::Fenced)?;
        Ok(Self {
            path: path.to_owned(),
            lock,
        })
    }
    pub fn release(&self) {
        let _ = FileExt::unlock(&self.lock);
    }
    pub fn read(&self) -> Result<Option<Record>, StorageError> {
        match std::fs::read(&self.path) {
            Ok(bytes) => {
                let record: Record =
                    serde_json::from_slice(&bytes).map_err(|_| StorageError::Corrupt)?;
                if record.schema_version != 1 {
                    return Err(StorageError::Corrupt);
                }
                Ok(Some(record))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(io(e)),
        }
    }
    pub fn write(
        &self,
        world_key: &str,
        namespace: &str,
        installation: &str,
        head: &DurableHead,
    ) -> Result<(), StorageError> {
        let record = Record {
            schema_version: 1,
            world_key: world_key.into(),
            namespace: namespace.into(),
            installation: installation.into(),
            head: head.clone(),
        };
        let bytes = serde_json::to_vec(&record).map_err(|_| StorageError::Corrupt)?;
        let mut name = self.path.as_os_str().to_owned();
        name.push(format!(
            ".tmp-{}-{}",
            std::process::id(),
            TEMPORARY.fetch_add(1, Ordering::Relaxed)
        ));
        let temporary = PathBuf::from(name);
        let result = (|| {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .map_err(io)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(std::fs::Permissions::from_mode(0o600))
                    .map_err(io)?;
            }
            file.write_all(&bytes).map_err(io)?;
            file.sync_all().map_err(io)?;
            std::fs::rename(&temporary, &self.path).map_err(io)?;
            File::open(self.path.parent().ok_or(StorageError::Binding)?)
                .map_err(io)?
                .sync_all()
                .map_err(io)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }
}
#[allow(clippy::needless_pass_by_value)] // map_err owns the concrete IO error.
fn io(error: std::io::Error) -> StorageError {
    StorageError::Anchor(error.to_string())
}
