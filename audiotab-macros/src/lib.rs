use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

mod node_meta;
use node_meta::{parse_node_info, parse_fields};

#[proc_macro_derive(StreamNode, attributes(node_meta, param, input, output))]
pub fn derive_stream_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let node_info = match parse_node_info(&input) {
        Ok(info) => info,
        Err(e) => return e.write_errors().into(),
    };

    let fields = parse_fields(&input);

    let struct_name = &input.ident;
    let node_id = struct_name.to_string().to_lowercase();
    let node_name = &node_info.name;
    let category = &node_info.category;

    // Generate parameters
    let params = fields.iter().filter_map(|f| {
        let field_name = f.ident.as_ref()?.to_string();
        let default_val = f.default.as_ref().map(|s| s.as_str()).unwrap_or("null");
        let type_name = extract_type_name(&f.ty);

        let param_code = if let (Some(min), Some(max)) = (f.min, f.max) {
            quote! {
                audiotab::registry::ParameterSchema {
                    name: #field_name.to_string(),
                    param_type: #type_name.to_string(),
                    default: serde_json::json!(#default_val),
                    min: Some(#min),
                    max: Some(#max),
                }
            }
        } else {
            quote! {
                audiotab::registry::ParameterSchema {
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

    let expanded = quote! {
        inventory::submit! {
            audiotab::registry::NodeMetadata::new(
                #node_id,
                #node_name,
                #category,
            )
            .with_factory(|| Box::new(#struct_name::default()))
            #(.add_parameter(#params))*
        }
    };

    TokenStream::from(expanded)
}

fn extract_type_name(ty: &syn::Type) -> &'static str {
    // Simple type extraction for common types
    let type_str = quote!(#ty).to_string();

    if type_str.contains("f64") || type_str.contains("f32") {
        "number"
    } else if type_str.contains("String") || type_str.contains("str") {
        "string"
    } else if type_str.contains("bool") {
        "boolean"
    } else {
        "unknown"
    }
}
