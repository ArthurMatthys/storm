extern crate proc_macro;

#[macro_use]
mod macros;

mod ctx;
mod derive_input_ext;
mod field_ext;

#[cfg(feature = "postgres")]
mod postgres;

use derive_input_ext::DeriveInputExt;
use field_ext::FieldExt;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Ctx)]
pub fn ctx(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    ctx::generate(&input).into()
}

#[cfg(feature = "postgres")]
#[proc_macro_derive(FromSql)]
pub fn from_sql(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    postgres::from_sql(&input).into()
}

#[cfg(feature = "postgres")]
#[proc_macro_derive(ToSql)]
pub fn to_sql(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    postgres::to_sql(&input).into()
}
