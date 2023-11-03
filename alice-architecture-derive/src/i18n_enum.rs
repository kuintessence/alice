use heck::{ToKebabCase, ToLowerCamelCase};
use proc_macro2::Literal;
use syn::{parse2, spanned::Spanned, ItemEnum};
pub fn internal_i18n_enum(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let item_enum: ItemEnum = match parse2(input) {
        Ok(x) => x,
        Err(e) => return e.into_compile_error(),
    };

    let enum_name = &item_enum.ident;

    for v in item_enum.variants.iter() {
        if matches!(v.fields, syn::Fields::Unnamed(_)) {
            return syn::Error::new(v.span(), "Unnamed variant is not supported in #[i18n_enum]")
                .into_compile_error();
        }
    }

    let mut match_variants_get_code = vec![];
    for v in item_enum.variants.iter() {
        let v_ident = &v.ident;
        let mut v_status_attr = None;

        for attr in v.attrs.iter() {
            if attr.path().is_ident("status") {
                match v_status_attr {
                    Some(_) => {
                        return syn::Error::new(v.span(), "Err: More than one status tag.")
                            .into_compile_error()
                    }
                    None => v_status_attr = Some(attr),
                }
            }
        }
        let status_code = match v_status_attr {
            Some(attr) => {
                let lit: Literal = attr.parse_args().unwrap();
                lit.to_string()
                    .trim_matches(|c| c == '(' || c == ')')
                    .to_owned()
                    .parse::<u16>()
                    .unwrap()
            }
            None => return syn::Error::new(v.span(), "Require a status tag.").into_compile_error(),
        };

        match_variants_get_code.push(match &v.fields {
            syn::Fields::Named(_f) => quote::quote! {
                Self::#v_ident {..} => #status_code,
            },
            syn::Fields::Unit => quote::quote! {
                Self::#v_ident {..} => #status_code,
            },
            syn::Fields::Unnamed(_) => unreachable!(),
        });
    }

    // Variants
    let match_variants_get_key_values_and_content = item_enum.variants.iter().map(|v| {
        let v_ident = &v.ident;
        // Use variant ident kebab string as i18n_key.
        let i18n_key = v_ident.to_string().to_kebab_case();
        // Get i18n_args
        // If Unnamed Variant, it doesn't support
        // If Unit Variant, it doesn't has arg
        // If Named Variant, use it's each named_field ident's camelCase string as arg key, and value as arg value
        let i18n_getter = match &v.fields {
            syn::Fields::Named(fields) => {
                let fields = &fields.named;

                let fields_without_type = fields.iter().map(|f| {
                    let f_ident = f.ident.as_ref().unwrap();
                    quote::quote! {
                        #f_ident,
                    }
                });

                let fields_value_insert_to_hash_map = fields.iter().map(|f| {
                    let f_ident = f.ident.as_ref().unwrap();
                    let param_value = f_ident;
                    let param_key = f_ident.to_string().to_lower_camel_case();
                    quote::quote! {
                        map.insert(#param_key.to_string(), #param_value.to_string());
                    }
                });

                let mut content_fields = vec![];
                for f in fields.iter() {
                    if f.attrs.iter().any(|a| a.path().is_ident("content")) {
                        let ident = &f.ident;
                        let ident_str = ident.as_ref().unwrap().to_string().to_lower_camel_case();
                        content_fields.push(quote::quote! {
                            #ident_str: #ident,
                        });
                    }
                }
                let content = if content_fields.is_empty() {
                    None
                } else {
                    Some(quote::quote! {
                        serde_json::json!({
                            #(#content_fields)*
                        })
                    })
                };
                match content {
                    Some(c) => quote::quote! {
                        Self::#v_ident {#(#fields_without_type)*} => {
                            let mut map = std::collections::HashMap::new();
                            #(#fields_value_insert_to_hash_map)*
                            (#i18n_key, Some(map), Some(#c))
                        },
                    },
                    None => quote::quote! {
                        Self::#v_ident {#(#fields_without_type)*} => {
                            let mut map = std::collections::HashMap::new();
                            #(#fields_value_insert_to_hash_map)*
                            (#i18n_key, Some(map), None)
                        },
                    },
                }
            }
            syn::Fields::Unit => quote::quote! {
                Self::#v_ident => (#i18n_key, None, None),
            },
            syn::Fields::Unnamed(_) => {
                unreachable!();
            }
        };
        i18n_getter
    });

    quote::quote! {
        impl alice_architecture::response::I18NEnum for #enum_name {
            fn localize(
                &self,
                locale: &dyn alice_architecture::response::Locale,
            ) -> anyhow::Result<alice_architecture::response::LocalizedMsg> {
                let status = self.get_err_code();
                let (message, content) = self.get_i18n_message_and_content(locale)?;
                Ok(
                    alice_architecture::response::LocalizedMsg {
                        status,
                        message,
                        content
                    }
                )
            }

            fn status(&self) -> u16 {
                self.get_err_code()
            }
        }

        impl #enum_name {
            fn get_key_and_args_and_content(&self) -> (&str, Option<std::collections::HashMap<String, String>>, Option<serde_json::Value>) {
                match self {
                    #(#match_variants_get_key_values_and_content)*
                }
            }

            fn get_i18n_message_and_content(&self, locale: &dyn alice_architecture::response::Locale) -> anyhow::Result<(String, Option<serde_json::Value>)> {
                let (key, args, content) = self.get_key_and_args_and_content();
                let msg = match args {
                    Some(a) => locale.text_with_args(key, a)?,
                    None => locale.text(key)?,
                };
                Ok((msg, content))
            }

            fn get_err_code(&self) -> u16 {
                match self {
                    #(#match_variants_get_code)*
                }
            }
        }

    }
}
