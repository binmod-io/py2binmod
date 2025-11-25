use quote::quote;
use proc_macro2::TokenStream;

use crate::types::{Parameter, ParameterType};

pub trait CodeGenerator {
    fn generate(&self) -> TokenStream;
}

pub trait AsTokenStream {
    fn as_token_stream(&self) -> TokenStream;
}

impl AsTokenStream for Parameter {
    fn as_token_stream(&self) -> TokenStream {
        let name = syn::Ident::new(&self.name, proc_macro2::Span::call_site());
        let type_hint = self.type_hint.as_token_stream();

        quote! { #name: #type_hint }
    }
}

impl AsTokenStream for ParameterType {
    fn as_token_stream(&self) -> TokenStream {
        match self {
            ParameterType::String => quote! { String },
            ParameterType::Integer => quote! { i64 },
            ParameterType::Float => quote! { f64 },
            ParameterType::Boolean => quote! { bool },
            ParameterType::List(item_type) => {
                let item_type = item_type.as_token_stream();
                
                quote! { Vec<#item_type> }
            },
            ParameterType::Tuple(inner_types) => {
                let inner_types = inner_types
                    .iter()
                    .map(|t| t.as_token_stream());
                
                quote! { (#(#inner_types),*) }
            },
            ParameterType::Map { key_type, value_type } => {
                let key_type = key_type.as_token_stream();
                let value_type = value_type.as_token_stream();

                quote! { std::collections::HashMap<#key_type, #value_type> }
            },
            ParameterType::Optional(inner_type) => {
                let inner_type = inner_type.as_token_stream();
                
                quote! { Option<#inner_type> }
            },
            ParameterType::None => quote! { () },
            ParameterType::Any => quote! { serde_json::Value },
        }
    }
}