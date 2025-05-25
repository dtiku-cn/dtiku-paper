use darling::{ast::NestedMeta, FromMeta};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, punctuated::Punctuated, ItemFn, Token};

#[derive(Debug, FromMeta)]
struct CachedArgs {
    key: String,
    #[darling(default)]
    expire: Option<u64>,
}

pub fn cached(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    // ✅ 解析成 NestedMeta，而不是 Meta！
    let args_parsed =
        parse_macro_input!(attr with Punctuated::<NestedMeta, Token![,]>::parse_terminated);
    let args_vec = args_parsed.into_iter().collect::<Vec<_>>();

    let args = match CachedArgs::from_list(&args_vec) {
        Ok(v) => v,
        Err(e) => return e.write_errors().into(),
    };

    let cache_key_fmt = args.key;
    let redis_set_stmt = match args.expire {
        Some(expire_sec) => {
            quote! {
                let _: redis::Value = redis.set_ex(&cache_key, json, #expire_sec).await.context("cache error")?;
            }
        }
        None => {
            quote! {
                let _: redis::Value = redis.set(&cache_key, json).await.context("cache error")?;
            }
        }
    };

    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let ident = &sig.ident;
    let inputs = &sig.inputs;
    let output = &sig.output;
    let asyncness = &sig.asyncness;
    let generics = &sig.generics;
    let where_clause = &sig.generics.where_clause;
    let attrs = &input_fn.attrs;
    let user_block = &input_fn.block;

    let gen = quote! {
        #(#attrs)*
        #vis #asyncness fn #ident #generics(#inputs) #output #where_clause {
            use anyhow::Context as _;
            use spring_redis::redis::{self,AsyncCommands};
            use spring::{plugin::ComponentRegistry, tracing, App};

            let mut redis = App::global()
                .get_component::<spring_redis::Redis>()
                .expect("redis component not found");

            let cache_key = format!(#cache_key_fmt);

            let cached = match redis.get::<_, Option<String>>(&cache_key).await {
                Ok(val) => val,
                Err(err) => {
                    tracing::error!("cache error:{:?}", err);
                    None
                }
            };

            let value = match cached {
                Some(json) => {
                    Some(serde_json::from_str(&json).with_context(||format!("cache for '{cache_key}' json decode failed"))?)
                },
                None => {
                    let result = (|| async #user_block)().await?;
                    if let Some(ref val) = result {
                        let json = serde_json::to_string(val).with_context(||format!("cache for '{cache_key}' json encode failed"))?;
                        #redis_set_stmt
                    }
                    result
                }
            };

            Ok(value)
        }
    };

    gen.into()
}
