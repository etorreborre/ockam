use core::{fmt, str};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::format;
use ockam_core::compat::string::ToString;
use ockam_core::{async_trait, RelayMessage};
use ockam_core::{AccessControl, Result};
use ockam_identity::authenticated_storage::AuthenticatedStorage;
use ockam_identity::{credential::AttributesStorageUtils, IdentitySecureChannelLocalInfo};
use tracing as log;

use crate::expr::str;
use crate::traits::PolicyStorage;
use crate::types::{Action, Resource};
use crate::Env;

/// Evaluates a policy expression against an environment of attributes.
///
/// Attributes come from a pre-populated environment and are augmented
/// by subject attributes from credential data.
#[derive(Debug)]
pub struct PolicyAccessControl<S> {
    resource: Resource,
    action: Action,
    policies: Box<dyn PolicyStorage>,
    attributes: S,
    environment: Env,
    overwrite: bool,
}

impl<S> PolicyAccessControl<S> {
    /// Create a new `PolicyAccessControl`.
    ///
    /// The policy expression is evaluated by getting subject attributes from
    /// the given authenticated storage, adding them the given environment,
    /// which may already contain other resource, action or subject attributes.
    pub fn new(policies: Box<dyn PolicyStorage>, store: S, r: Resource, a: Action, env: Env) -> Self {
        Self {
            resource: r,
            action: a,
            policies,
            attributes: store,
            environment: env,
            overwrite: false,
        }
    }

    pub fn overwrite(&mut self) {
        self.overwrite = true
    }
}

#[async_trait]
impl<S> AccessControl for PolicyAccessControl<S>
where
    S: AuthenticatedStorage + fmt::Debug,
{
    async fn is_authorized(&self, msg: &RelayMessage) -> Result<bool> {
        // Load the policy expression for resource and action:
        let policy = if let Some(policy) = self
            .policies
            .get_policy(&self.resource, &self.action)
            .await?
        {
            if let Some(b) = policy.is_constant_policy() {
                // If the policy is a constant there is no need to populate
                // the environment or look for message metadata.
                return Ok(b);
            } else {
                policy
            }
        } else {
            // If no policy exists for this resource and action access is denied:
            log::debug! {
                resource = %self.resource,
                action   = %self.action,
                "no policy found; access denied"
            }
            return Ok(false);
        };

        // Get identity identifier from message metadata:
        let id = if let Ok(info) = IdentitySecureChannelLocalInfo::find_info(&msg.local_msg) {
            info.their_identity_id().clone()
        } else {
            log::debug! {
                resource = %self.resource,
                action   = %self.action,
                "identity identifier not found; access denied"
            }
            return Ok(false);
        };

        // Get identity attributes and populate the environment:
        let attrs =
            if let Some(a) = AttributesStorageUtils::get_attributes(&id, &self.attributes).await? {
                a
            } else {
                log::debug! {
                    resource = %self.resource,
                    action   = %self.action,
                    id       = %id,
                    "attributes not found; access denied"
                }
                return Ok(false);
            };

        let mut e = self.environment.clone();

        for (k, v) in &attrs {
            if k.find(|c: char| c.is_whitespace()).is_some() {
                log::warn! {
                    resource = %self.resource,
                    action   = %self.action,
                    id       = %id,
                    key      = %k,
                    "attribute key with whitespace ignored"
                }
            }
            match str::from_utf8(v) {
                Ok(s) => {
                    if !self.overwrite && e.contains(k) {
                        log::debug! {
                            resource = %self.resource,
                            action   = %self.action,
                            id       = %id,
                            key      = %k,
                            "attribute already present"
                        }
                        continue;
                    }
                    e.put(format!("subject.{k}"), str(s.to_string()));
                }
                Err(e) => {
                    log::warn! {
                        resource = %self.resource,
                        action   = %self.action,
                        id       = %id,
                        key      = %k,
                        err      = %e,
                        "failed to interpret attribute as string"
                    }
                }
            }
        }

        // Finally, evaluate the expression and return the result:
        match &policy.evaluate_with_environment(&e) {
            Ok(Some(b)) => {
                log::debug! {
                    resource      = %self.resource,
                    action        = %self.action,
                    id            = %id,
                    is_authorized = %b,
                    "policy evaluated"
                }
                Ok(*b)
            }
            Ok(None) => {
                log::warn! {
                    resource = %self.resource,
                    action   = %self.action,
                    id       = %id,
                    expr     = %{policy.expression()},
                    "evaluation did not yield a boolean result"
                }
                Ok(false)
            }
            Err(e) => {
                log::warn! {
                    resource = %self.resource,
                    action   = %self.action,
                    id       = %id,
                    err      = %e,
                    "policy evaluation failed"
                }
                Ok(false)
            }
        }
    }
}
