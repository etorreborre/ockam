use ockam_core::async_trait;
use ockam_core::compat::{boxed::Box, string::String, vec::Vec};
use ockam_core::{AsyncTryClone, Result};
use ambassador::delegatable_trait;
use ambassador::Delegate;

/// Storage for Authenticated data
#[async_trait]
#[delegatable_trait]
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

#[derive(Delegate)]
#[delegate(AuthenticatedStorage)]
#[derive(Clone, Debug)]
pub enum AuthenticatedStorageImpl {
  DatabaseImpl(LmdbStorage),
  InMemoryImpl(InMemoryStorage)
}
