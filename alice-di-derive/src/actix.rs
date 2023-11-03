use proc_macro2::TokenStream;
use syn::{parse::Parse, parse2, punctuated::Punctuated, token::Comma, FnArg, ItemFn, Token, Type};

pub fn internal_actix_auto_inject(
    attr: proc_macro2::TokenStream,
    body: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let body: ItemFn = match parse2(body) {
        Ok(x) => x,
        Err(e) => return e.into_compile_error(),
    };
    let attr: ServiceProviderBuilder = match parse2(attr) {
        Ok(x) => x,
        Err(e) => return e.into_compile_error(),
    };
    let type_name = &attr.type_name;
    let sp_input: FnArg = match parse2(quote::quote! {sp: ::actix_web::web::Data<#type_name>}) {
        Ok(x) => x,
        Err(e) => return e.into_compile_error(),
    };
    let scoped_config_input: FnArg = match parse2(
        quote::quote! {scoped_config: ::alice_infrastructure::middleware::authorization::AliceScopedConfig},
    ) {
        Ok(x) => x,
        Err(e) => return e.into_compile_error(),
    };
    let (visibility, attrs, block) = (&body.vis, &body.attrs, &body.block);
    let mut sig = body.sig.clone();
    let inputs = sig.inputs;
    let mut new_inputs = Punctuated::<FnArg, Comma>::new();
    let mut injects = Vec::<TokenStream>::new();

    let header = if attr.is_async {
        attr.scoped_config.map(|x| {
            quote::quote! {
                let sp = sp.create_scoped(#x).await.unwrap();
            }
        })
    } else {
        attr.scoped_config.map(|x| {
            quote::quote! {
                let sp = sp.create_scoped(#x).unwrap();
            }
        })
    };
    for input in inputs.iter() {
        match input {
            FnArg::Typed(x) => {
                if x.attrs.iter().any(|x| x.path().get_ident().unwrap().to_string().eq("inject")) {
                    let (pat, ty) = (&x.pat, &x.ty);
                    injects.push(quote::quote! {
                        let #pat: #ty = sp.provide();
                    });
                } else {
                    new_inputs.push(FnArg::Typed(x.clone()))
                }
            }
            _ => new_inputs.push(input.clone()),
        }
    }
    new_inputs.push(sp_input);
    new_inputs.push(scoped_config_input);
    sig.inputs = new_inputs;
    quote::quote! {
        #(#attrs)*
        #visibility #sig {
            #header
            #(#injects)*
            #block
        }
    }
}

mod kw {
    syn::custom_keyword!(scoped);
    syn::custom_keyword!(async_scoped);
}

#[derive(Clone, Debug)]
struct ServiceProviderBuilder {
    pub type_name: Type,
    pub scoped_config: Option<TokenStream>,
    pub is_async: bool,
}

impl Parse for ServiceProviderBuilder {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let type_name = input.call(Type::parse)?;
        let mut is_async = false;
        let scoped_config = if input.peek(Token![,]) {
            let _ = input.parse::<Token![,]>()?;
            let scoped_config = quote::quote!(scoped_config.clone());
            if input.peek(kw::scoped) {
                let _ = input.parse::<kw::scoped>()?;
                Some(scoped_config)
            } else if input.peek(kw::async_scoped) {
                let _ = input.parse::<kw::async_scoped>()?;
                is_async = true;
                Some(scoped_config)
            } else {
                None
            }
        } else {
            None
        };
        Ok(Self {
            type_name,
            scoped_config,
            is_async,
        })
    }
}
