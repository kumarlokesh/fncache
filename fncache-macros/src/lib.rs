use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{parse::Parse, parse::ParseStream, parse_macro_input, ItemFn, Lit, Token};
use syn::{Error, Result};

/// Enum to represent different key derivation strategies
enum KeyDerivation {
    Runtime,
    CompileTime,
}

/// Parse the attributes passed to the fncache macro
struct FncacheArgs {
    ttl: Option<u64>,
    key_derivation: KeyDerivation,
}

impl Parse for FncacheArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let vars = Punctuated::<syn::MetaNameValue, Token![,]>::parse_terminated(input)?;

        let mut ttl = None;
        let mut key_derivation = KeyDerivation::Runtime;

        for var in vars {
            let ident = var
                .path
                .get_ident()
                .ok_or_else(|| Error::new_spanned(&var.path, "Expected identifier"))?;

            if ident == "ttl" {
                match &var.lit {
                    Lit::Int(lit) => {
                        ttl = Some(lit.base10_parse()?);
                    }
                    _ => return Err(Error::new_spanned(&var.lit, "ttl must be an integer")),
                }
            } else if ident == "key_derivation" {
                match &var.lit {
                    Lit::Str(lit_str) => {
                        let value = lit_str.value();
                        if value == "runtime" {
                            key_derivation = KeyDerivation::Runtime;
                        } else if value == "compile_time" {
                            key_derivation = KeyDerivation::CompileTime;
                        } else {
                            return Err(Error::new_spanned(
                                &var.lit,
                                "key_derivation must be either 'runtime' or 'compile_time'",
                            ));
                        }
                    }
                    _ => {
                        return Err(Error::new_spanned(
                            &var.lit,
                            "key_derivation must be a string literal",
                        ))
                    }
                }
            }
        }

        Ok(FncacheArgs {
            ttl,
            key_derivation,
        })
    }
}

#[proc_macro_attribute]
pub fn fncache(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input::parse::<FncacheArgs>(attr.clone()).unwrap_or_else(|_| {
        FncacheArgs {
            ttl: None,
            key_derivation: KeyDerivation::Runtime,
        }
    });

    let use_compile_time_keys = match args.key_derivation {
        KeyDerivation::CompileTime => true,
        KeyDerivation::Runtime => false,
    };

    let ttl_seconds = args.ttl.unwrap_or(60);

    let input_fn = parse_macro_input!(item as ItemFn);

    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let block = &input_fn.block;
    let attrs = &input_fn.attrs;

    let fn_name = &sig.ident;
    let asyncness = &sig.asyncness;
    let _generics = &sig.generics;
    let inputs = &sig.inputs;
    let _output = &sig.output;

    let is_async = asyncness.is_some();

    let arg_names = inputs.iter().map(|arg| match arg {
        syn::FnArg::Receiver(_) => quote! { self },
        syn::FnArg::Typed(pat_type) => {
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                let ident = &pat_ident.ident;
                quote! { #ident }
            } else {
                quote! { _ }
            }
        }
    });

    let arg_names1: Vec<_> = arg_names.clone().collect();
    let _arg_names2: Vec<_> = arg_names.collect();

    let expanded = if is_async {
        quote! {
            #(#attrs)*
            #vis #sig {
                use fncache::backends::CacheBackend;
                use std::time::Duration;
                use futures::TryFutureExt;

                let key = if #use_compile_time_keys {
                    format!("{}-ct-{}", module_path!(), stringify!(#fn_name))
                } else {
                    format!("{}-{:?}", stringify!(#fn_name), (#(&(#arg_names1)),*))
                };

                if let Ok(cache_guard) = fncache::global_cache().lock() {
                    if let Ok(Some(cached)) = cache_guard.get(&key).await {
                        if let Ok(deserialized) = bincode::deserialize::<_>(&cached) {
                            return deserialized;
                        }
                    }
                }

                let result = #block;

                if let Ok(serialized) = bincode::serialize(&result) {
                    if let Ok(mut cache_guard) = fncache::global_cache().lock() {
                        let _ = cache_guard.set(
                            key,
                            serialized,
                            Some(Duration::from_secs(#ttl_seconds))
                        ).await;
                    }
                }

                result
            }
        }
    } else {
        quote! {
            #(#attrs)*
            #vis #sig {
                use fncache::backends::CacheBackend;
                use std::time::Duration;
                use futures::executor;

                let key = if #use_compile_time_keys {
                    format!("{}-ct-{}", module_path!(), stringify!(#fn_name))
                } else {
                    format!("{}-{:?}", stringify!(#fn_name), (#(&(#arg_names1)),*))
                };

                if let Ok(cache_guard) = fncache::global_cache().lock() {
                    if let Ok(Some(cached)) = executor::block_on(cache_guard.get(&key)) {
                        if let Ok(deserialized) = bincode::deserialize::<_>(&cached) {
                            return deserialized;
                        }
                    }
                }

                let result = #block;

                if let Ok(serialized) = bincode::serialize(&result) {
                    if let Ok(mut cache_guard) = fncache::global_cache().lock() {
                        let _ = executor::block_on(cache_guard.set(
                            key,
                            serialized,
                            Some(Duration::from_secs(#ttl_seconds))
                        ));
                    }
                }

                result
            }
        }
    };

    expanded.into()
}
