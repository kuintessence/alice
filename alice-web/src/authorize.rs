use syn::{parse2, ItemFn};

pub fn internal_authorize(
    _attr: proc_macro2::TokenStream,
    body: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let body: ItemFn = match parse2(body) {
        Ok(x) => x,
        Err(e) => return e.into_compile_error(),
    };
    let (visibility, attrs, block, sig) = (&body.vis, &body.attrs, &body.block.stmts, &body.sig);
    quote::quote! {
        #(#attrs)*
        #visibility #sig {
            let user_info: Option<alice_architecture::authorization::UserInfo> = {
                let extensions = actix_web::HttpMessage::extensions(&raw_req);
                let user_info_ext: Option<&anyhow::Result<Option<alice_architecture::authorization::UserInfo>>> = extensions.get();
                let user_info_ext = match user_info_ext {
                    Some(u) => u,
                    None => {
                        return actix_web::web::Json(ResponseBase::err(400, "Unauthorized."));
                    },
                };
                match user_info_ext {
                    Ok(u) => u.to_owned(),
                    Err(e) => {
                        tracing::error!("{e}");
                        return actix_web::web::Json(ResponseBase::err(400, &e.to_string()));
                    }
                }
            };
            #(#block)*
        }
    }
}
