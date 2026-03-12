#![recursion_limit = "512"]

extern crate proc_macro;
mod async_channel;
mod sync_channel;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, format_ident, quote};
use syn::{
    AttrStyle, Attribute, FnArg, Ident, Pat, PatType, ReturnType, Token, Type, Visibility, braced,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
    spanned::Spanned,
    token::Comma,
};

fn combine_errors(mut base: syn::Result<()>, new_error: syn::Error) -> syn::Result<()> {
    match base {
        Ok(_) => Err(new_error),
        Err(ref mut errors) => {
            errors.combine(new_error);
            base
        }
    }
}

struct Service {
    attrs: Vec<Attribute>,
    vis: Visibility,
    ident: Ident,
    rpcs: Vec<RpcMethod>,
}

struct RpcMethod {
    is_async: bool,
    attrs: Vec<Attribute>,
    ident: Ident,
    args: Vec<PatType>,
    output: ReturnType,
}

impl Parse for Service {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        input.parse::<Token![trait]>()?;
        let ident: Ident = input.parse()?;
        let content;
        braced!(content in input);
        let mut rpcs = Vec::<RpcMethod>::new();
        while !content.is_empty() {
            rpcs.push(content.parse()?);
        }
        let mut ident_errors = Ok(());
        for rpc in &rpcs {
            if rpc.ident == "new" {
                ident_errors = combine_errors(
                    ident_errors,
                    syn::Error::new(
                        rpc.ident.span(),
                        format!(
                            "method name conflicts with generated fn `{}Client::new`",
                            ident.unraw()
                        ),
                    ),
                );
            }
            if rpc.ident == "serve" {
                ident_errors = combine_errors(
                    ident_errors,
                    syn::Error::new(
                        rpc.ident.span(),
                        format!("method name conflicts with generated fn `{ident}::serve`"),
                    ),
                );
            }
        }
        ident_errors?;

        Ok(Self {
            attrs,
            vis,
            ident,
            rpcs,
        })
    }
}

impl Parse for RpcMethod {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let is_async = input.parse::<Token![async]>().is_ok();
        input.parse::<Token![fn]>()?;
        let ident = input.parse()?;
        let content;
        parenthesized!(content in input);
        let mut args = Vec::new();
        let mut errors = Ok(());
        let mut has_self = false;

        for arg in content.parse_terminated(FnArg::parse, Comma)? {
            match arg {
                FnArg::Typed(captured) if matches!(&*captured.pat, Pat::Ident(_)) => {
                    args.push(captured);
                }
                FnArg::Typed(captured) => {
                    errors = combine_errors(
                        errors,
                        syn::Error::new(captured.pat.span(), "patterns aren't allowed in RPC args"),
                    );
                }
                FnArg::Receiver(ref r) => {
                    if has_self {
                        errors = combine_errors(
                            errors,
                            syn::Error::new(arg.span(), "duplicate self parameter"),
                        );
                    }
                    if r.lifetime().is_some() {
                        errors = combine_errors(
                            errors,
                            syn::Error::new(arg.span(), "self parameter cannot have a lifetime"),
                        );
                    }
                    if r.mutability.is_some() {
                        errors = combine_errors(
                            errors,
                            syn::Error::new(arg.span(), "self parameter cannot be mutable"),
                        );
                    }
                    has_self = true;
                }
            }
        }
        errors?;
        let output = input.parse()?;
        input.parse::<Token![;]>()?;

        Ok(Self {
            is_async,
            attrs,
            ident,
            args,
            output,
        })
    }
}

fn collect_cfg_attrs(rpcs: &[RpcMethod]) -> Vec<Vec<&Attribute>> {
    rpcs.iter()
        .map(|rpc| {
            rpc.attrs
                .iter()
                .filter(|att| {
                    att.style == AttrStyle::Outer
                        && match &att.meta {
                            syn::Meta::List(syn::MetaList { path, .. }) => {
                                path.get_ident() == Some(&Ident::new("cfg", rpc.ident.span()))
                            }
                            _ => false,
                        }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

#[proc_macro_attribute]
pub fn service(attr: TokenStream, input: TokenStream) -> TokenStream {
    let _ = attr; // 忽略属性参数
    let original_trait = TokenStream2::from(input.clone());
    let unit_type: &Type = &parse_quote!(());
    let Service {
        ref attrs,
        ref vis,
        ref ident,
        ref rpcs,
    } = parse_macro_input!(input as Service);

    let camel_case_fn_names: &Vec<_> = &rpcs
        .iter()
        .map(|rpc| snake_to_camel(&rpc.ident.unraw().to_string()))
        .collect();
    let args: &[&[PatType]] = &rpcs.iter().map(|rpc| &*rpc.args).collect::<Vec<_>>();

    let derives = quote! {
        #[derive(
            ::anycall::serde::Serialize,
            ::anycall::serde::Deserialize
        )]
        #[serde(crate = "::anycall::serde")]
    };

    let methods = rpcs.iter().map(|rpc| &rpc.ident).collect::<Vec<_>>();

    ServiceGenerator {
        original_trait: &original_trait,
        async_service_trait_ident: &format_ident!("{}AsyncService", ident),
        async_provider_ident: &format_ident!("{}AsyncProvider", ident),
        sync_service_trait_ident: &format_ident!("{}SyncService", ident),
        sync_provider_ident: &format_ident!("{}SyncProvider", ident),
        client_ident: &format_ident!("{}Client", ident),
        async_client_trait_ident: &format_ident!("{}AsyncClient", ident),
        sync_client_trait_ident: &format_ident!("{}SyncClient", ident),
        request_ident: &format_ident!("{}Request", ident),
        response_ident: &format_ident!("{}Response", ident),
        vis,
        args,
        method_attrs: &rpcs.iter().map(|rpc| &*rpc.attrs).collect::<Vec<_>>(),
        method_cfgs: &collect_cfg_attrs(rpcs),
        method_idents: &methods,
        attrs,
        return_types: &rpcs
            .iter()
            .map(|rpc| match rpc.output {
                ReturnType::Type(_, ref ty) => ty.as_ref(),
                ReturnType::Default => unit_type,
            })
            .collect::<Vec<_>>(),
        arg_pats: &args
            .iter()
            .map(|args| args.iter().map(|arg| &*arg.pat).collect())
            .collect::<Vec<_>>(),
        camel_case_idents: &rpcs
            .iter()
            .zip(camel_case_fn_names.iter())
            .map(|(rpc, name)| Ident::new(name, rpc.ident.span()))
            .collect::<Vec<_>>(),
        derives: &derives,
        all_sync_methods: rpcs.iter().all(|rpc| !rpc.is_async),
    }
    .into_token_stream()
    .into()
}

struct ServiceGenerator<'a> {
    original_trait: &'a TokenStream2,
    async_service_trait_ident: &'a Ident,
    async_provider_ident: &'a Ident,
    sync_service_trait_ident: &'a Ident,
    sync_provider_ident: &'a Ident,
    client_ident: &'a Ident,
    async_client_trait_ident: &'a Ident,
    sync_client_trait_ident: &'a Ident,
    request_ident: &'a Ident,
    response_ident: &'a Ident,
    vis: &'a Visibility,
    attrs: &'a [Attribute],
    camel_case_idents: &'a [Ident],
    method_idents: &'a [&'a Ident],
    method_attrs: &'a [&'a [Attribute]],
    method_cfgs: &'a [Vec<&'a Attribute>],
    args: &'a [&'a [PatType]],
    return_types: &'a [&'a Type],
    arg_pats: &'a [Vec<&'a Pat>],
    derives: &'a TokenStream2,
    all_sync_methods: bool,
}

impl ServiceGenerator<'_> {
    fn original_trait(&self) -> TokenStream2 {
        self.original_trait.clone()
    }

    fn enum_request(&self) -> TokenStream2 {
        let &Self {
            derives,
            vis,
            request_ident,
            camel_case_idents,
            args,
            method_cfgs,
            ..
        } = self;

        quote! {
            #[allow(missing_docs)]
            #[derive(Debug)]
            #derives
            #vis enum #request_ident {
                #(
                    #(#method_cfgs)*
                    #camel_case_idents { #(#args),* }
                ),*
            }
        }
    }

    fn enum_response(&self) -> TokenStream2 {
        let &Self {
            derives,
            vis,
            response_ident,
            camel_case_idents,
            return_types,
            ..
        } = self;

        quote! {
            #[allow(missing_docs)]
            #[derive(Debug)]
            #derives
            #vis enum #response_ident {
                #(#camel_case_idents(#return_types)),*
            }
        }
    }

    fn struct_client(&self) -> TokenStream2 {
        let &Self {
            vis, client_ident, ..
        } = self;

        quote! {
            #[allow(unused)]
            #vis struct #client_ident<A>(A);
        }
    }

    fn impl_client_new(&self) -> TokenStream2 {
        let &Self {
            client_ident, vis, ..
        } = self;

        quote! {
            impl<A> #client_ident<A> {
                #vis fn new(client: A) -> #client_ident<A> {
                    #client_ident(client)
                }
            }
        }
    }
}

impl ToTokens for ServiceGenerator<'_> {
    fn to_tokens(&self, output: &mut TokenStream2) {
        let async_gen = async_channel::AsyncChannelGenerator {
            async_service_trait_ident: self.async_service_trait_ident,
            async_provider_ident: self.async_provider_ident,
            async_client_trait_ident: self.async_client_trait_ident,
            request_ident: self.request_ident,
            response_ident: self.response_ident,
            client_ident: self.client_ident,
            vis: self.vis,
            attrs: self.attrs,
            camel_case_idents: self.camel_case_idents,
            method_idents: self.method_idents,
            method_attrs: self.method_attrs,
            method_cfgs: self.method_cfgs,
            args: self.args,
            return_types: self.return_types,
            arg_pats: self.arg_pats,
        };

        let mut generated_tokens = vec![
            self.original_trait(),
            async_gen.struct_async_provider(),
            async_gen.impl_serve_for_server(),
            self.enum_request(),
            self.enum_response(),
            async_gen.trait_async_client(),
            self.struct_client(),
            self.impl_client_new(),
            async_gen.impl_client_rpc_methods(),
            async_gen.impl_server_rpc_methods(),
        ];

        if self.all_sync_methods {
            let sync_gen = sync_channel::SyncChannelGenerator {
                sync_service_trait_ident: self.sync_service_trait_ident,
                sync_provider_ident: self.sync_provider_ident,
                sync_client_trait_ident: self.sync_client_trait_ident,
                request_ident: self.request_ident,
                response_ident: self.response_ident,
                client_ident: self.client_ident,
                vis: self.vis,
                attrs: self.attrs,
                camel_case_idents: self.camel_case_idents,
                method_idents: self.method_idents,
                method_attrs: self.method_attrs,
                method_cfgs: self.method_cfgs,
                args: self.args,
                return_types: self.return_types,
                arg_pats: self.arg_pats,
            };

            generated_tokens.extend(vec![
                sync_gen.struct_sync_provider(),
                sync_gen.impl_serve_for_sync_server(),
                sync_gen.trait_sync_client(),
                sync_gen.impl_sync_client_rpc_methods(),
                sync_gen.impl_sync_server_rpc_methods(),
            ]);
        }

        output.extend(generated_tokens);
    }
}

fn snake_to_camel(ident_str: &str) -> String {
    let mut camel_ty = String::with_capacity(ident_str.len());

    let mut last_char_was_underscore = true;
    for c in ident_str.chars() {
        match c {
            '_' => last_char_was_underscore = true,
            c if last_char_was_underscore => {
                camel_ty.extend(c.to_uppercase());
                last_char_was_underscore = false;
            }
            c => camel_ty.extend(c.to_lowercase()),
        }
    }

    camel_ty.shrink_to_fit();
    camel_ty
}
