use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{DeriveInput, ItemStruct, parse_quote};

pub fn impl_arrgegate_root(ast: DeriveInput) -> TokenStream {
    let ident = &ast.ident;
    let data = &ast.data;

    let mut db_ident_str = "Db".to_string();
    db_ident_str.push_str(&ident.to_string());
    let db_ident = Ident::new(&db_ident_str, Span::call_site());

    let db_struct: ItemStruct = match data {
        syn::Data::Struct(ref s) => match s.fields {
            syn::Fields::Named(ref fields) => {
                let mut db_fields = fields.clone();
                db_fields.named.iter_mut().for_each(|f| {
                    let ty = &f.ty;
                    let db_ty: syn::Type =
                        parse_quote!(alice_architecture::repository::DbField<#ty>);
                    f.ty = db_ty;
                });
                parse_quote! {
                    pub struct #db_ident
                    #db_fields
                }
            }
            _ => {
                return syn::Error::new(ident.span(), "AggregateRoot only support named fields.")
                    .into_compile_error();
            }
        },
        _ => {
            return syn::Error::new(ident.span(), "AggregateRoot only support struct.")
                .into_compile_error();
        }
    };

    let from_items = if let syn::Fields::Named(ref fields) = db_struct.fields {
        fields
            .named
            .iter()
            .map(|f| {
                let f_ident = &f.ident;
                quote!(#f_ident: alice_architecture::repository::DbField::Set(value.#f_ident),)
            })
            .collect::<Vec<_>>()
    } else {
        unreachable!()
    };

    quote! {
        impl alice_architecture::model::AggregateRoot for #ident {
            type UpdateEntity = #db_ident;
        }

        impl alice_architecture::repository::DbEntity for #db_ident { }

        #db_struct

        impl From<#ident> for #db_ident {
            fn from(value: #ident) -> Self {
                Self {
                    #(#from_items)*
                }
            }
        }
    }
}
