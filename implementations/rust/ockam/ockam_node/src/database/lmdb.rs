use core::str;
use lmdb::{Database, Environment, Transaction};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use crate::tokio::task::{self, JoinError};
use std::fmt;
use std::path::Path;
use std::sync::Arc;

/// Lmdb AuthenticatedStorage implementation
#[derive(Clone)]
pub struct LmdbStorage {
    /// current database environment
    pub env: Arc<Environment>,
    /// the database itself as a key value map
    pub map: Database,
}

impl fmt::Debug for LmdbStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Store")
    }
}

impl LmdbStorage {
    /// Constructor
    pub async fn new<P: AsRef<Path>>(p: P) -> Result<Self> {
        let p = p.as_ref().to_path_buf();
        let t = move || {
            let env = Environment::new()
                .set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR | lmdb::EnvironmentFlags::NO_TLS)
                .set_max_dbs(1)
                .open(p.as_ref())
                .map_err(map_lmdb_err)?;
            let map = env
                .create_db(Some("map"), lmdb::DatabaseFlags::empty())
                .map_err(map_lmdb_err)?;
            Ok(LmdbStorage {
                env: Arc::new(env),
                map,
            })
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }

    /// Write a key-value pair in the database
    pub async fn write(&self, k: String, v: Vec<u8>) -> Result<()> {
        let d = self.clone();
        let t = move || {
            let mut w = d.env.begin_rw_txn().map_err(map_lmdb_err)?;
            w.put(d.map, &k, &v, lmdb::WriteFlags::empty())
                .map_err(map_lmdb_err)?;
            w.commit().map_err(map_lmdb_err)?;
            Ok(())
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }

    /// Delete a key-value pair from the database
    pub async fn delete(&self, k: String) -> Result<()> {
        let d = self.clone();
        let t = move || {
            let mut w = d.env.begin_rw_txn().map_err(map_lmdb_err)?;
            match w.del(d.map, &k, None) {
                Ok(()) | Err(lmdb::Error::NotFound) => {}
                Err(e) => return Err(map_lmdb_err(e)),
            }
            w.commit().map_err(map_lmdb_err)?;
            Ok(())
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }
}

/// Make an ockam error from a join error
pub fn map_join_err(err: JoinError) -> Error {
    Error::new(Origin::Application, Kind::Io, err)
}

/// Make an ockam error from a lmdb error
pub fn map_lmdb_err(err: lmdb::Error) -> Error {
    Error::new(Origin::Application, Kind::Io, err)
}

/// Make an ockam error from a UTF-8 decoding error
pub fn from_utf8_err(err: str::Utf8Error) -> Error {
    Error::new(Origin::Other, Kind::Invalid, err)
}
