use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse2, punctuated::Punctuated, AttributeArgs, ItemFn, Meta::NameValue, NestedMeta, ReturnType,
};

use crate::internals::attr::{parse_lit_into_int, parse_lit_into_path, Attr};
use crate::internals::{ast, ast::FnVariable, check, ctx::Context, symbol::*};

/// This macro will split the input function in two: the wrapper function that will be
/// called from `test`, and the test function that will contain the test code.
///
/// The following code:
/// ```ignore
/// #[ockam::test)]
/// async fn my_test(ctx: &mut ockam::Context) -> ockam::Result<()> {
///     ctx.stop().await
/// }
/// ```
///
/// Will be expanded to (ignoring part of the code generated by the compiler to run the test):
/// ```ignore
/// async fn _my_test(ctx: &mut ockam::Context) -> ockam::Result<()> {
///     ctx.stop().await
/// }
///
/// fn expand() {
///     use core::panic::AssertUnwindSafe;
///     use core::time::Duration;
///     use ockam_core::{Error, errcode::{Origin, Kind}};
///     use ockam::{NodeBuilder, compat::{tokio::time::timeout, futures::FutureExt}};
///     let (mut ctx, mut executor) = NodeBuilder::without_access_control().build();
///     executor
///         .execute(async move {
///             match AssertUnwindSafe(async {
///                 match timeout(Duration::from_millis(100u64), _my_test(&mut ctx)).await {
///                     Ok(r) => r,
///                     Err(_) => Err(Error::new(Origin::Node, Kind::Timeout, "Test timed out")),
///                 }
///             })
///             .catch_unwind()
///             .await
///             {
///                 Ok(r) => {
///                     if r.is_err() {
///                         let _ = AssertUnwindSafe(async { ctx.stop().await.unwrap(); })
///                             .catch_unwind()
///                             .await;
///                     }
///                     r
///                 }
///                 Err(_) => {
///                     let _ = AssertUnwindSafe(async { ctx.stop().await.unwrap(); })
///                         .catch_unwind()
///                         .await;
///                     ::core::panicking::panic_fmt(::core::fmt::Arguments::new_v1(
///                         &["Test panicked"],
///                         &[],
///                     ));
///                 }
///             }
///         })
///         .expect("Test panicked")
///         .expect("Test function returned error");
/// }
/// ```
pub(crate) fn expand(
    input_fn: ItemFn,
    attrs: AttributeArgs,
) -> Result<TokenStream, Vec<syn::Error>> {
    let mut test_fn = input_fn.clone();
    let ctx = Context::new();
    let cont = Container::from_ast(&ctx, &mut test_fn, input_fn, &attrs);
    ctx.check()?;
    Ok(output(cont))
}

fn output(mut cont: Container) -> TokenStream {
    let ctx_ident = match cont.data.ockam_ctx {
        None => quote! {ctx},
        Some(ctx) => {
            let ident = ctx.ident;
            quote! {#ident}
        }
    };
    let ctx_stop_stmt = quote! {
        let _ = AssertUnwindSafe(async { #ctx_ident.stop().await.unwrap(); })
            .catch_unwind()
            .await;
    };
    let test_fn = &cont.test_fn;
    let test_fn_ident = &cont.test_fn.sig.ident;
    let ockam_crate = cont.data.attrs.ockam_crate;
    let timeout_ms = cont.data.attrs.timeout_ms;
    cont.original_fn.block = parse2(quote! {
        {
            use core::panic::AssertUnwindSafe;
            use core::time::Duration;
            use ockam_core::{Error, errcode::{Origin, Kind}};
            use #ockam_crate::{NodeBuilder, compat::{tokio::time::timeout, futures::FutureExt}};

            let (mut #ctx_ident, mut executor) = NodeBuilder::without_access_control().build();
            executor
                .execute(async move {
                    // Wraps the test function call in a `catch_unwind` to catch possible panics.
                    match AssertUnwindSafe(async {
                        match timeout(Duration::from_millis(#timeout_ms), #test_fn_ident(&mut #ctx_ident)).await {
                            // Test went well. Return result as is.
                            Ok(r) => r,
                            // Test timed out. Return a custom error that we can handle.
                            Err(_) => Err(Error::new(Origin::Node, Kind::Timeout, "Test timed out"))
                        }
                    })
                    .catch_unwind()
                    .await
                    {
                        // Return test result.
                        Ok(r) => {
                            // It returned an error, so the context might not have been stopped.
                            // In that case, we try to stop the context manually.
                            if r.is_err() {
                                #ctx_stop_stmt
                            }
                            r
                        },
                        // Test panicked. Stop the context and bubble up the panic to make the test fail.
                        Err(_) => {
                            #ctx_stop_stmt
                            panic!("Test panicked");
                        }
                    }
                })
                .expect("Test panicked")
                .expect("Test function returned error");
        }
    }).expect("Parsing failure");
    let input_fn = &cont.original_fn;
    quote! {
        #test_fn
        #[::core::prelude::v1::test]
        #input_fn
    }
}

struct Container<'a> {
    // Macro data.
    data: Data<'a>,
    // Original function.
    original_fn: ItemFn,
    // Test function derived from the original.
    test_fn: &'a ItemFn,
}

impl<'a> Container<'a> {
    fn from_ast(
        ctx: &Context,
        test_fn: &'a mut ItemFn,
        input_fn: ItemFn,
        attrs: &AttributeArgs,
    ) -> Self {
        // The test function is renamed adding an `_` in front of the original name so that it
        // can be called from the original function.
        let fn_ident = &test_fn.sig.ident;
        test_fn.sig.ident = Ident::new(&format!("_{}", &fn_ident), fn_ident.span());

        let mut cont = Self {
            data: Data::from_ast(ctx, test_fn, attrs),
            original_fn: input_fn,
            test_fn,
        };
        cont.check(ctx);
        cont.cleanup();
        cont
    }

    // Compared to the `node` macro, this macro is more constrained to ensure that a test doesn't run indefinitely.
    // Most of the checks validate that the ockam context is defined properly in the input function so that it
    // can be stopped after the test is finished or after it times out.
    fn check(&self, ctx: &Context) {
        check::item_fn::is_async(ctx, self.test_fn);
        check::item_fn::returns_result(ctx, self.test_fn);
        check::item_fn::has_one_arg(ctx, self.test_fn);
        check::item_fn::has_ockam_ctx_arg(ctx, self.test_fn, &self.data.ockam_ctx);
        check::item_fn::ockam_ctx_is_mut_ref(ctx, &self.data.ockam_ctx);
    }

    fn cleanup(&mut self) {
        // Remove the arguments
        self.original_fn.sig.inputs = Punctuated::new();
        // Remove the output
        self.original_fn.sig.output = ReturnType::Default;
        // Remove async
        self.original_fn.sig.asyncness = None;
    }
}

struct Data<'a> {
    // Macro attributes.
    attrs: Attributes,
    // The `ctx` variable data extracted from the input function arguments.
    // (e.g. from `ctx: &mut ockam::Context` it extracts `ctx`, `&`, `mut` and `ockam::Context`).
    ockam_ctx: Option<FnVariable<'a>>,
}

impl<'a> Data<'a> {
    fn from_ast(ctx: &Context, input_fn: &'a ItemFn, attrs: &AttributeArgs) -> Self {
        Self {
            attrs: Attributes::from_ast(ctx, attrs),
            ockam_ctx: ast::ockam_context_variable_from_input_fn(ctx, input_fn),
        }
    }
}

struct Attributes {
    ockam_crate: TokenStream,
    timeout_ms: u64,
}

impl Attributes {
    fn from_ast(ctx: &Context, attrs: &AttributeArgs) -> Self {
        let mut ockam_crate = Attr::none(ctx, OCKAM_CRATE);
        let mut timeout_ms = Attr::none(ctx, TIMEOUT_MS);
        for attr in attrs {
            match attr {
                // Parse `#[ockam::test(crate = "ockam")]`
                NestedMeta::Meta(NameValue(nv)) if nv.path == OCKAM_CRATE => {
                    if let Ok(path) = parse_lit_into_path(ctx, OCKAM_CRATE, &nv.lit) {
                        ockam_crate.set(&nv.path, quote! { #path });
                    }
                }
                // Parse `#[ockam::test(timeout = 1000)]`
                NestedMeta::Meta(NameValue(nv)) if nv.path == TIMEOUT_MS => {
                    if let Ok(timeout) = parse_lit_into_int::<u64>(ctx, TIMEOUT_MS, &nv.lit) {
                        timeout_ms.set(&nv.path, timeout);
                    }
                }
                NestedMeta::Meta(m) => {
                    let path = m.path().into_token_stream().to_string().replace(' ', "");
                    ctx.error_spanned_by(m.path(), format!("unknown attribute `{}`", path));
                }
                NestedMeta::Lit(lit) => {
                    ctx.error_spanned_by(lit, "unexpected literal in attribute");
                }
            }
        }
        Self {
            ockam_crate: ockam_crate.get().unwrap_or(quote! { ockam_node }),
            timeout_ms: timeout_ms.get().unwrap_or(30_000),
        }
    }
}
