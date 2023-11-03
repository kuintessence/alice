mod aggregate_root;
mod i18n_enum;

use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::DeriveInput;

use i18n_enum::internal_i18n_enum;

#[proc_macro_derive(AggregateRoot)]
pub fn aggregate_root(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    aggregate_root::impl_arrgegate_root(ast).into()
}

#[proc_macro_derive(I18NEnum, attributes(status, content))]
pub fn i18n_enum(body: TokenStream) -> TokenStream {
    internal_i18n_enum(proc_macro2::TokenStream::from(body)).into() }
