use crate::policy::{Policy, PolicyList};
use crate::types::{Action, Resource};
use core::fmt::Debug;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use objekt_clonable::*;

#[clonable]
#[async_trait]
pub trait PolicyStorage: Debug + Send + Sync + Clone + 'static {
    async fn get_policy(&self, r: &Resource, a: &Action) -> Result<Option<Policy>>;
    async fn set_policy(&self, r: &Resource, a: &Action, c: &Policy) -> Result<()>;
    async fn del_policy(&self, r: &Resource, a: &Action) -> Result<()>;
    async fn policies(&self, r: &Resource) -> Result<PolicyList>;
}
