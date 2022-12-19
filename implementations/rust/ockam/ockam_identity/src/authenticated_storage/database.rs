use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use core::str;
use ockam_node::database::lmdb::*;
use crate::authenticated_storage::AuthenticatedStorage;
use ockam_node::tokio::task::{self};
use lmdb::Transaction;

#[async_trait]
impl AuthenticatedStorage for LmdbStorage {
    async fn get(&self, id: &str, key: &str) -> Result<Option<Vec<u8>>> {
        let d = self.clone();
        let k = format!("{id}:{key}");
        let t = move || {
            let r = d.env.begin_ro_txn().map_err(map_lmdb_err)?;
            match r.get(d.map, &k) {
                Ok(value) => Ok(Some(Vec::from(value))),
                Err(lmdb::Error::NotFound) => Ok(None),
                Err(e) => Err(map_lmdb_err(e)),
            }
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }

    async fn set(&self, id: &str, key: String, val: Vec<u8>) -> Result<()> {
        self.write(format!("{id}:{key}"), val).await
    }

    async fn del(&self, id: &str, key: &str) -> Result<()> {
        self.delete(format!("{id}:{key}")).await
    }
}
