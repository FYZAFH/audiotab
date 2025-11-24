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
        .filter(|f| f.attrs.iter().any(|attr| attr.path().is_ident("param")))
        .filter_map(|f| ParamField::from_field(f).ok())
        .collect()
}

pub fn parse_ports(input: &DeriveInput) -> (Vec<PortField>, Vec<PortField>) {
    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => return (Vec::new(), Vec::new()),
        },
        _ => return (Vec::new(), Vec::new()),
    };

    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    for field in fields.iter() {
        // Check for #[input] attribute
        if field.attrs.iter().any(|attr| attr.path().is_ident("input")) {
            if let Ok(port) = PortField::from_field(field) {
                inputs.push(port);
            }
        }

        // Check for #[output] attribute
        if field.attrs.iter().any(|attr| attr.path().is_ident("output")) {
            if let Ok(port) = PortField::from_field(field) {
                outputs.push(port);
            }
        }
    }

    (inputs, outputs)
}
