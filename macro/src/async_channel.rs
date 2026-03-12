use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Attribute, Ident, Pat, PatType, Type, Visibility};

pub(crate) struct AsyncChannelGenerator<'a> {
    pub async_service_trait_ident: &'a Ident,
    pub async_provider_ident: &'a Ident,
    pub async_client_trait_ident: &'a Ident,
    pub request_ident: &'a Ident,
    pub response_ident: &'a Ident,
    pub client_ident: &'a Ident,
    pub vis: &'a Visibility,
    pub attrs: &'a [Attribute],
    pub camel_case_idents: &'a [Ident],
    pub method_idents: &'a [&'a Ident],
    pub method_attrs: &'a [&'a [Attribute]],
    pub method_cfgs: &'a [Vec<&'a Attribute>],
    pub args: &'a [&'a [PatType]],
    pub return_types: &'a [&'a Type],
    pub arg_pats: &'a [Vec<&'a Pat>],
}

impl AsyncChannelGenerator<'_> {
    pub fn struct_async_provider(&self) -> TokenStream2 {
        let Self {
            vis,
            async_provider_ident,
            ..
        } = self;

        quote! {
            #vis struct #async_provider_ident<S> {
                service: S,
            }
        }
    }

    pub fn impl_serve_for_server(&self) -> TokenStream2 {
        let Self {
            request_ident,
            async_provider_ident,
            response_ident,
            camel_case_idents,
            arg_pats,
            method_idents,
            method_cfgs,
            async_service_trait_ident,
            ..
        } = self;

        let match_case_blocks = method_cfgs
            .iter()
            .zip(camel_case_idents.iter())
            .zip(arg_pats.iter())
            .zip(method_idents.iter())
            .map(
                |(((method_cfgs, camel_case_ident), arg_pats), method_ident)| {
                    quote! {
                        #(#method_cfgs)*
                        #request_ident::#camel_case_ident { #(#arg_pats),* } => {
                            let fut = self.service.#method_ident(context, #(#arg_pats),*);
                            ::std::boxed::Box::pin(async move {
                                let response = fut.await;
                                #response_ident::#camel_case_ident(response)
                            })
                        }
                    }
                },
            );

        quote! {
            impl<S, Ctx> ::anycall::async_channel::AsyncServiceProvider for #async_provider_ident<S>
            where
                S: #async_service_trait_ident<Ctx = Ctx>
            {
                type Req = #request_ident;
                type Resp = #response_ident;
                type Ctx = Ctx;

                fn serve(
                    &self,
                    context: <Self as ::anycall::async_channel::AsyncServiceProvider>::Ctx,
                    request_body: <Self as ::anycall::async_channel::AsyncServiceProvider>::Req,
                ) -> ::std::pin::Pin<
                    ::std::boxed::Box<
                        dyn ::std::future::Future<
                            Output = <Self as ::anycall::async_channel::AsyncServiceProvider>::Resp
                        > + Send + '_
                    >
                > {
                    match request_body {
                        #(#match_case_blocks)*
                    }
                }
            }
        }
    }

    pub fn trait_async_client(&self) -> TokenStream2 {
        let Self {
            async_client_trait_ident,
            method_attrs,
            method_idents,
            args,
            return_types,
            vis,
            ..
        } = self;

        quote! {
            #vis trait #async_client_trait_ident {
                type Err;
                #(
                    #[allow(unused)]
                    #(#method_attrs)*
                    async fn #method_idents(&self, #(#args),*) -> ::core::result::Result<#return_types, Self::Err>;
                )*
            }
        }
    }

    pub fn impl_client_rpc_methods(&self) -> TokenStream2 {
        let Self {
            client_ident,
            request_ident,
            response_ident,
            method_attrs,
            method_idents,
            args,
            return_types,
            arg_pats,
            camel_case_idents,
            async_client_trait_ident,
            ..
        } = self;

        quote! {
            impl<A> #async_client_trait_ident for #client_ident<A>
            where
                A: ::anycall::async_channel::AsyncClientAgent
            {
                type Err = A::Err;

                #(
                    #[allow(unused)]
                    #(#method_attrs)*
                    fn #method_idents(
                        &self,
                        #(#args),*
                    ) -> impl ::core::future::Future<Output = ::core::result::Result<#return_types, A::Err>> + '_ {
                        let request = #request_ident::#camel_case_idents { #(#arg_pats),* };
                        let resp = self.0.call(request);
                        async move {
                            match resp.await? {
                                #response_ident::#camel_case_idents(msg) => ::core::result::Result::Ok(msg),
                                _ => ::core::unreachable!(),
                            }
                        }
                    }
                )*
            }
        }
    }

    pub fn impl_server_rpc_methods(&self) -> TokenStream2 {
        let Self {
            attrs,
            vis,
            return_types,
            async_service_trait_ident,
            async_provider_ident,
            method_attrs,
            method_idents,
            args,
            ..
        } = self;

        let rpc_fns = method_attrs
            .iter()
            .zip(method_idents.iter())
            .zip(args.iter())
            .zip(return_types.iter())
            .map(|(((attrs, ident), args), output)| {
                quote! {
                    #(#attrs)*
                    fn #ident(
                        &self,
                        ctx: Self::Ctx,
                        #(#args),*
                    ) -> impl ::std::future::Future<Output = #output> + Send + '_;
                }
            });

        quote! {
            #(#attrs)*
            #vis trait #async_service_trait_ident: ::core::marker::Sized {
                type Ctx;

                #(#rpc_fns)*

                fn into_provider(self) -> #async_provider_ident<Self> {
                    #async_provider_ident { service: self }
                }
            }
        }
    }
}
