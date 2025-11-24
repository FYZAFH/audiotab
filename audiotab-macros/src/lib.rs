use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

mod node_meta;
use node_meta::{parse_node_info, parse_fields, parse_ports};

#[proc_macro_derive(StreamNode, attributes(node_meta, param, input, output))]
pub fn derive_stream_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let node_info = match parse_node_info(&input) {
        Ok(info) => info,
        Err(e) => return e.write_errors().into(),
    };

    let fields = parse_fields(&input);
    let (inputs, outputs) = parse_ports(&input);

    let struct_name = &input.ident;
    let node_id = struct_name.to_string().to_lowercase();
    let node_name = &node_info.name;
    let category = &node_info.category;

    // Generate parameters
    let params = fields.iter().filter_map(|f| {
        let field_name = f.ident.as_ref()?.to_string();

        // Skip fields with #[serde(skip)]
        let has_skip = f.default.is_none();
        if has_skip { return None; }

        let default_val = f.default.as_ref()?.as_str();
        let type_name = extract_type_name(&f.ty);

        let param_code = if let (Some(min), Some(max)) = (f.min, f.max) {
            quote! {
                crate::registry::ParameterSchema {
                    name: #field_name.to_string(),
                    param_type: #type_name.to_string(),
                    default: serde_json::json!(#default_val),
                    min: Some(#min),
                    max: Some(#max),
                }
            }
        } else {
            quote! {
                crate::registry::ParameterSchema {
                    name: #field_name.to_string(),
                    param_type: #type_name.to_string(),
                    default: serde_json::json!(#default_val),
                    min: None,
                    max: None,
                }
            }
        };

        Some(param_code)
    });

    // Generate input port metadata
    let input_metas = inputs.iter().map(|port| {
        let port_id = port.ident.as_ref().unwrap().to_string();
        let port_name = port.name.as_ref().unwrap_or(&port_id);
        let data_type = port.data_type.as_ref().map(|s| s.as_str()).unwrap_or("any");

        quote! {
            crate::registry::PortMetadata {
                id: #port_id.to_string(),
                name: #port_name.to_string(),
                data_type: #data_type.to_string(),
            }
        }
    });

    // Generate output port metadata
    let output_metas = outputs.iter().map(|port| {
        let port_id = port.ident.as_ref().unwrap().to_string();
        let port_name = port.name.as_ref().unwrap_or(&port_id);
        let data_type = port.data_type.as_ref().map(|s| s.as_str()).unwrap_or("any");

        quote! {
            crate::registry::PortMetadata {
                id: #port_id.to_string(),
                name: #port_name.to_string(),
                data_type: #data_type.to_string(),
            }
        }
    });

    let mod_name = syn::Ident::new(
        &format!("__node_registration_{}", struct_name.to_string().to_lowercase()),
        struct_name.span(),
    );

    let factory_fn_name = syn::Ident::new(
        &format!("create_metadata_{}", struct_name.to_string().to_lowercase()),
        struct_name.span(),
    );

    let expanded = quote! {
        mod #mod_name {
            use super::*;

            fn #factory_fn_name() -> crate::registry::NodeMetadata {
                crate::registry::NodeMetadata {
                    id: #node_id.to_string(),
                    name: #node_name.to_string(),
                    category: #category.to_string(),
                    inputs: vec![#(#input_metas),*],
                    outputs: vec![#(#output_metas),*],
                    parameters: vec![#(#params),*],
                    factory: || Box::new(#struct_name::default()),
                }
            }

            ::inventory::submit! {
                crate::registry::NodeMetadataFactoryWrapper(#factory_fn_name)
            }
        }
    };

    TokenStream::from(expanded)
}

fn extract_type_name(ty: &syn::Type) -> &'static str {
    let type_str = quote!(#ty).to_string();

    if type_str.contains("f64") || type_str.contains("f32") {
        "number"
    } else if type_str.contains("u32") || type_str.contains("i32")
        || type_str.contains("u64") || type_str.contains("i64")
        || type_str.contains("usize") || type_str.contains("isize") {
        "number"
    } else if type_str.contains("String") || type_str.contains("str") {
        "string"
    } else if type_str.contains("bool") {
        "boolean"
    } else {
        "unknown"
    }
}
