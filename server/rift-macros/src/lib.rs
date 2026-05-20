//! Compile-time attribute macros for service-layer authorization.
//!
//! `#[requires(Permission::X)]` and `#[requires_any(...)]` inject a scope
//! check at the top of a service method body, delegating to
//! `AuthContext::require()` / `require_any()`. `#[requires_public]` is a
//! no-op marker that opts a method out of the architecture test enforcing
//! presence of one of these attributes.
//!
//! The runtime side lives in `server/src/services/auth/permissions/`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream, Parser},
    parse_macro_input,
    punctuated::Punctuated,
    Expr, ItemFn, LitStr, Token,
};

/// Require the caller's `AuthContext` to carry the given `Permission` before
/// the function body runs. Expands to an explicit early-return guard
/// (`if let Err(e) = ctx.require(<expr>) { return Err(e.into()); }`)
/// prepended to the function block. The early-return form (vs.
/// `.map_err(From::from)?`) keeps target-type inference unambiguous when
/// the service's error enum has multiple `From` impls.
///
/// The function must take a parameter named `ctx`. Its type should be
/// `&AuthContext`; the macro itself only checks the name (mismatched types
/// surface as the normal "method not found on type" compiler error).
#[proc_macro_attribute]
pub fn requires(args: TokenStream, input: TokenStream) -> TokenStream {
    let perm: Expr = match syn::parse(args) {
        Ok(e) => e,
        Err(e) => return e.to_compile_error().into(),
    };
    let item_fn = parse_macro_input!(input as ItemFn);
    if let Err(ts) = check_ctx_param(&item_fn) {
        return ts;
    }
    // Use an explicit `return Err(e.into())` instead of `.map_err(From::from)?`
    // so that when a service's error enum implements `From` for multiple
    // upstream error types (e.g. `From<QuotaError>` + `From<AuthzError>`),
    // the target type is inferred from the function's return type rather than
    // ambiguous against `Into::into` at the `?` site.
    let check = quote! {
        if let ::core::result::Result::Err(__rift_authz_err) = ctx.require(#perm) {
            return ::core::result::Result::Err(::core::convert::Into::into(__rift_authz_err));
        }
    };
    inject(item_fn, check).into()
}

/// Require the caller to carry at least one of the listed permissions. Expands
/// to an explicit early-return guard prepended to the function block.
#[proc_macro_attribute]
pub fn requires_any(args: TokenStream, input: TokenStream) -> TokenStream {
    let perms = match Punctuated::<Expr, Token![,]>::parse_terminated.parse(args) {
        Ok(p) if !p.is_empty() => p,
        Ok(_) => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "`#[requires_any]` needs at least one permission",
            )
            .to_compile_error()
            .into();
        }
        Err(e) => return e.to_compile_error().into(),
    };
    let item_fn = parse_macro_input!(input as ItemFn);
    if let Err(ts) = check_ctx_param(&item_fn) {
        return ts;
    }
    let perms_iter = perms.iter();
    let check = quote! {
        if let ::core::result::Result::Err(__rift_authz_err) =
            ctx.require_any(&[ #(#perms_iter),* ])
        {
            return ::core::result::Result::Err(::core::convert::Into::into(__rift_authz_err));
        }
    };
    inject(item_fn, check).into()
}

/// Marker for service methods that intentionally bypass scope enforcement.
/// Required `reason = "..."` documents *why* in source. No-op expansion —
/// the attribute's sole purpose is satisfying the `architecture_tests` rule
/// that every `pub async fn` taking `&AuthContext` declare its intent.
#[proc_macro_attribute]
pub fn requires_public(args: TokenStream, input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(args as RequiresPublicArgs);
    if parsed.reason.value().trim().is_empty() {
        return syn::Error::new(parsed.reason.span(), "`reason = \"...\"` must be non-empty")
            .to_compile_error()
            .into();
    }
    input
}

// ── Helpers ──

struct RequiresPublicArgs {
    reason: LitStr,
}

impl Parse for RequiresPublicArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: syn::Ident = input.parse()?;
        if key != "reason" {
            return Err(syn::Error::new(key.span(), "expected `reason = \"...\"`"));
        }
        input.parse::<Token![=]>()?;
        let reason: LitStr = input.parse()?;
        Ok(Self { reason })
    }
}

fn check_ctx_param(item_fn: &ItemFn) -> Result<(), TokenStream> {
    let has_ctx = item_fn.sig.inputs.iter().any(|arg| {
        matches!(
            arg,
            syn::FnArg::Typed(pt)
                if matches!(&*pt.pat, syn::Pat::Ident(p) if p.ident == "ctx")
        )
    });
    if has_ctx {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(
            &item_fn.sig,
            "`#[requires]` / `#[requires_any]` needs a parameter named `ctx` (typically `ctx: &AuthContext`)",
        )
        .to_compile_error()
        .into())
    }
}

fn inject(mut item_fn: ItemFn, stmt: TokenStream2) -> TokenStream2 {
    let parsed: syn::Stmt = syn::parse2(stmt).expect("rift-macros: generated stmt failed to parse");
    item_fn.block.stmts.insert(0, parsed);
    quote!(#item_fn)
}
