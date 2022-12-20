use ockam_core::async_trait;
use ockam_core::compat::{boxed::Box, string::String, vec::Vec};
use ockam_core::{AsyncTryClone, Result};

/// Storage for Authenticated data
#[async_trait]
pub trait AuthenticatedStorage: AsyncTryClone + Send + Sync + 'static {
    /// Get entry
    async fn get(&self, id: &str, key: &str) -> Result<Option<Vec<u8>>>;

    /// Set entry
    async fn set(&self, id: &str, key: String, val: Vec<u8>) -> Result<()>;

    /// Delete entry
    async fn del(&self, id: &str, key: &str) -> Result<()>;
}

/// In-memory impl
pub mod mem;

/// Database impl
pub mod database;

use mem::*;
use ockam_node::database::lmdb::*;

#[derive(Clone, Debug)]
pub enum AuthenticatedStorageImpl {
  DatabaseImpl(LmdbStorage),
  InMemoryImpl(InMemoryStorage)
}
use AuthenticatedStorageImpl::*;

#[async_trait]
impl AuthenticatedStorage for AuthenticatedStorageImpl {
    /// Get entry
    async fn get(&self, id: &str, key: &str) -> Result<Option<Vec<u8>>> {
        match self {
            DatabaseImpl(s) => (*s).get(id, key).await,
            InMemoryImpl(s) => (*s).get(id, key).await
        }
    }

    /// Set entry
    async fn set(&self, id: &str, key: String, val: Vec<u8>) -> Result<()> {
        match self {
            DatabaseImpl(s) => s.set(id, key, val).await,
            InMemoryImpl(s) => s.set(id, key, val).await
        }
    }

    /// Delete entry
    async fn del(&self, id: &str, key: &str) -> Result<()> {
        match self {
            DatabaseImpl(s) => s.del(id, key).await,
            InMemoryImpl(s) => s.del(id, key).await
        }
    }
}
