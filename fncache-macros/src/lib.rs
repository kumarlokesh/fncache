use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn fncache(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ttl_seconds = if attr.is_empty() {
        60u64
    } else {
        let attr_str = attr.to_string();
        if let Some(ttl_str) = attr_str.split('=').nth(1) {
            ttl_str.trim().parse::<u64>().unwrap_or(60u64)
        } else {
            60u64
        }
    };

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

                let key = format!("{}-{:?}", stringify!(#fn_name), (#(&(#arg_names1)),*));

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

                let key = format!("{}-{:?}", stringify!(#fn_name), (#(&(#arg_names1)),*));

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
