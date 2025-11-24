use darling::{FromAttributes, FromField};
use syn::{DeriveInput, Fields};

/// Parsed attributes from #[node_meta(...)]
#[derive(Debug, FromAttributes)]
#[darling(attributes(node_meta))]
pub struct NodeMetaArgs {
    pub name: String,
    pub category: String,
}

/// Parsed attributes from #[param(...)]
#[derive(Debug, FromField)]
#[darling(attributes(param))]
pub struct ParamField {
    pub ident: Option<syn::Ident>,
    pub ty: syn::Type,

    #[darling(default)]
    pub default: Option<String>,

    #[darling(default)]
    pub min: Option<f64>,

    #[darling(default)]
    pub max: Option<f64>,
}

/// Parse inputs/outputs from #[port(...)]
#[derive(Debug, FromField)]
#[darling(attributes(input, output))]
pub struct PortField {
    pub ident: Option<syn::Ident>,

    #[darling(default)]
    pub name: Option<String>,

    #[darling(default)]
    pub data_type: Option<String>,
}

pub fn parse_node_info(input: &DeriveInput) -> darling::Result<NodeMetaArgs> {
    NodeMetaArgs::from_attributes(&input.attrs)
}

pub fn parse_fields(input: &DeriveInput) -> Vec<ParamField> {
    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => return Vec::new(),
        },
        _ => return Vec::new(),
    };

    fields
        .iter()
        .filter_map(|f| ParamField::from_field(f).ok())
        .collect()
}
