use proc_macro::TokenStream as ProcTokenStream;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, ItemEnum
};

/// implies `derive(Message, Serialize, Deserialize)`
#[proc_macro_attribute]
pub fn message(_args: ProcTokenStream, input: ProcTokenStream) -> ProcTokenStream {
    let input: TokenStream = input.into();
    quote!(
        #[derive(lightyear::prelude::Message, serde::Serialize, serde::Deserialize)]
        #input
    ).into()
}

/// implies `derive(Component, Message, Serialize, Deserialize)`
/// implies sync(full) if no sync attribute is found
#[proc_macro_attribute]
pub fn component(_args: ProcTokenStream, input: ProcTokenStream) -> ProcTokenStream {
    let input: TokenStream = input.into();
    quote!(
        #[derive(bevy::prelude::Component, lightyear::prelude::Message, serde::Serialize, serde::Deserialize)]
        #input
    ).into()
}

/// implies `derive(Serialize, Deserialize, Clone)`
#[proc_macro_attribute]
pub fn input(_args: ProcTokenStream, input: ProcTokenStream) -> ProcTokenStream {
    let input: TokenStream = input.into();
    quote!(
        #[derive(serde::Serialize, serde::Deserialize, Clone)]
        #input
    ).into()
}

/// Marks the Input map similar to `component_protocol` and `message_protocol`
/// implies `UserAction` + `derive(Serialize, Deserialize, Clone, PartialEq, Debug)`
#[proc_macro_attribute]
pub fn inputs(_args: ProcTokenStream, input: ProcTokenStream) -> ProcTokenStream {
    let output: TokenStream = input.clone().into();
    let item = parse_macro_input!(input as ItemEnum);
    let name = &item.ident;

    quote!(
        #[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Debug)]
        #output

        impl lightyear::prelude::UserAction for #name {}
    ).into()
}
