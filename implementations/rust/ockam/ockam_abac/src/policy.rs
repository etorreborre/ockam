use crate::expr::Expr;
use crate::types::Action;
use minicbor::{Decode, Encode};
use crate::eval::eval;
use crate::env::{Env};
use crate::error::EvalError;
use std::fmt;
use std::fmt::{Formatter, Display};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Decode, Encode, Clone)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Policy {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2000111>,
    #[n(1)] expression: Expr,
}

impl Display for Policy {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.expression, f)
    }
}

impl Policy {
    pub fn new(e: Expr) -> Self {
        Policy {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            expression: e,
        }
    }

    pub fn expression(&self) -> &Expr {
        &self.expression
    }

    // If the policy doesn't depend on any environment or resource
    // return the boolean it evaluates to
    // This allows to avoid getting data to evaluate the policy expression
    pub fn is_constant_policy(&self) -> Option<bool> {
        match self.expression {
            Expr::Bool(b) => Some(b),
            _ => None,
        }
    }

    // Evaluate the policy expression given an environment
    // if the expression can be reduced to a boolean return the value
    // return None if the expression cannot be reduced to a boolean
    // return an error if the evaluation produces an error. This can
    // happen if an expression is not typechecking, for example if it contains a comparison
    // operator with only one operand. Most of theses issues could be mitigated by providing
    // a proper parser + typechecker for the expression language
    pub fn evaluate_with_environment(&self, env: &Env) -> Result<Option<bool>, EvalError> {
        match eval(self.expression(), env) {
          Ok(Expr::Bool(b)) => Ok(Some(b)),
          Ok(_) => Ok(None),
          Err(e) => Err(e),
        }
    }
}

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PolicyList {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3521457>,
    #[n(1)] policies: Vec<(Action, Policy)>,
}

impl PolicyList {
    pub fn new(ps: Vec<(Action, Policy)>) -> Self {
        PolicyList {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            policies: ps,
        }
    }

    pub fn policies(&self) -> &[(Action, Policy)] {
        &self.policies
    }
}
