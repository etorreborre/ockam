use crate::policy::{Policy, PolicyList};
use crate::types::{Action, Resource};
use core::fmt::Debug;
use core::str;
use lmdb::Cursor;
use lmdb::Transaction;
use minicbor::{Decode, Encode};
use objekt_clonable::*;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use ockam_node::database::lmdb::*;
use ockam_node::tokio::task::{self};
use std::borrow::Cow;
use tracing as log;

#[async_trait]
impl PolicyStorage for LmdbStorage {
    async fn get_policy(&self, r: &Resource, a: &Action) -> Result<Option<Policy>> {
        let d = self.clone();
        let k = format!("{r}:{a}");
        let t = move || {
            let r = d.env.begin_ro_txn().map_err(map_lmdb_err)?;
            match r.get(d.map, &k) {
                Ok(value) => {
                    let pe: PolicyEntry = minicbor::decode(value)?;
                    Ok(Some(pe.policy.into_owned()))
                }
                Err(lmdb::Error::NotFound) => Ok(None),
                Err(e) => Err(map_lmdb_err(e)),
            }
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }

    async fn set_policy(&self, r: &Resource, a: &Action, c: &Policy) -> Result<()> {
        let v = minicbor::to_vec(PolicyEntry {
            policy: Cow::Borrowed(c),
        })?;
        self.write(format!("{r}:{a}"), v).await
    }

    async fn del_policy(&self, r: &Resource, a: &Action) -> Result<()> {
        self.delete(format!("{r}:{a}")).await
    }

    async fn policies(&self, r: &Resource) -> Result<PolicyList> {
        let d = self.clone();
        let r = r.clone();
        let t = move || {
            let tx = d.env.begin_ro_txn().map_err(map_lmdb_err)?;
            let mut c = tx.open_ro_cursor(d.map).map_err(map_lmdb_err)?;
            let mut xs = Vec::new();
            for entry in c.iter_from(r.as_str()) {
                let (k, v) = entry.map_err(map_lmdb_err)?;
                let ks = str::from_utf8(k).map_err(from_utf8_err)?;
                if let Some((prefix, a)) = ks.split_once(':') {
                    if prefix != r.as_str() {
                        break;
                    }
                    let x: PolicyEntry = minicbor::decode(v)?;
                    xs.push((Action::new(a), x.policy.into_owned()))
                } else {
                    log::warn!(key = %ks, "malformed key in policy database")
                }
            }
            Ok(xs)
        };
        task::spawn_blocking(t)
            .await
            .map_err(map_join_err)
            .map(|r| r.map(PolicyList::new))?
    }
}

/// Policy storage entry.
///
/// Used instead of storing plain `Policy` values to allow for additional
/// metadata, versioning, etc.
#[derive(Debug, Encode, Decode)]
#[rustfmt::skip]
struct PolicyEntry<'a> {
    #[b(0)] policy: Cow<'a, Policy>
}
