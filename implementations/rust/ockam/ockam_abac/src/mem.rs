use crate::policy::{Policy, PolicyList};
use crate::traits::PolicyStorage;
use crate::types::{Action, Resource};
use core::fmt;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::compat::vec::Vec;
use ockam_core::Result;

#[derive(Default)]
#[derive(Clone)]
pub struct Memory {
    pub(crate) inner: Arc<RwLock<Inner>>,
}

impl fmt::Debug for Memory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Memory")
    }
}

impl Memory {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner::new())),
        }
    }
}

#[derive(Default)]
#[derive(Clone)]
pub struct Inner {
    policies: BTreeMap<Resource, BTreeMap<Action, Policy>>,
}

impl Inner {
    fn new() -> Self {
        Inner::default()
    }

    fn del_policy(&mut self, r: &Resource, a: &Action) {
        if let Some(p) = self.policies.get_mut(r) {
            p.remove(a);
            if p.is_empty() {
                self.policies.remove(r);
            }
        }
    }

    fn get_policy(&self, r: &Resource, a: &Action) -> Option<Policy> {
        self.policies.get(r).and_then(|p| p.get(a).cloned())
    }

    fn set_policy(&mut self, r: &Resource, a: &Action, p: &Policy) {
        self.policies
            .entry(r.clone())
            .or_insert_with(BTreeMap::new)
            .insert(a.clone(), p.clone());
    }

    fn policies(&self, r: &Resource) -> Vec<(Action, Policy)> {
        if let Some(p) = self.policies.get(r) {
            p.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        }
    }
}

#[async_trait]
impl PolicyStorage for Memory {
    async fn del_policy(&self, r: &Resource, a: &Action) -> Result<()> {
        self.inner.write().unwrap().del_policy(r, a);
        Ok(())
    }

    async fn get_policy(&self, r: &Resource, a: &Action) -> Result<Option<Policy>> {
        Ok(self.inner.read().unwrap().get_policy(r, a))
    }

    async fn set_policy(&self, r: &Resource, a: &Action, p: &Policy) -> Result<()> {
        self.inner.write().unwrap().set_policy(r, a, p);
        Ok(())
    }

    async fn policies(&self, r: &Resource) -> Result<PolicyList> {
        Ok(PolicyList::new(self.inner.write().unwrap().policies(r)))
    }
}

#[cfg(test)]
mod tests {
    use crate::env::Env;
    use crate::expr::{int, seq, str};
    use crate::mem::Memory;
    use crate::parser::parse;
    use crate::policy::{Policy};
    use crate::types::{Action, Resource};

    #[test]
    fn example1() {
        let condition = r#"
            (and (= resource.version "1.0.0")
                 (= subject.name "John")
                 (member? "John" resource.admins))
        "#;

        let action = Action::new("r");
        let resource = Resource::new("/foo/bar/baz");
        let store = Memory::new();

        store.inner.write().unwrap().set_policy(
            &resource,
            &action,
            &(Policy::new(parse(condition).unwrap().unwrap())),
        );

        let mut e = Env::new();
        e.put("subject.age", int(25))
            .put("subject.name", str("John"))
            .put("resource.version", str("1.0.0"))
            .put("resource.admins", seq([str("root"), str("John")]));

        let policy = store
            .inner
            .write()
            .unwrap()
            .get_policy(&resource, &action)
            .unwrap();
        assert!(policy.evaluate_with_environment(&e).unwrap().unwrap())
    }
}
