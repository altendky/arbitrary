use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use syn::*;

/// Used to filter out necessary field attribute and in panics.
static ARBITRARY_ATTRIBUTE_NAME: &str = "arbitrary";

/// Determines how a value for a field should be constructed.
#[cfg_attr(test, derive(Debug))]
pub enum FieldConstructor {
    /// Assume that Arbitrary is defined for the type of this field and use it (default)
    Arbitrary,

    /// Places `Default::default()` as a field value.
    Default,

    /// Use custom function or closure to generate a value for a field.
    With(TokenStream),

    /// Set a field always to the given value.
    Value(TokenStream),
}

pub fn determine_field_constructor(field: &Field) -> Result<FieldConstructor> {
    let opt_attr = fetch_attr_from_field(field)?;
    let ctor = match opt_attr {
        Some(attr) => parse_attribute(attr)?,
        None => FieldConstructor::Arbitrary,
    };
    Ok(ctor)
}

fn fetch_attr_from_field(field: &Field) -> Result<Option<&Attribute>> {
    let found_attributes: Vec<_> = field
        .attrs
        .iter()
        .filter(|a| {
            let path = &a.path;
            let name = quote!(#path).to_string();
            name == ARBITRARY_ATTRIBUTE_NAME
        })
        .collect();
    if found_attributes.len() > 1 {
        let name = field.ident.as_ref().unwrap();
        panic!(
            "Multiple conflicting #[{ARBITRARY_ATTRIBUTE_NAME}] attributes found on field `{name}`"
        );
    }
    Ok(found_attributes.into_iter().next())
}

fn parse_attribute(attr: &Attribute) -> Result<FieldConstructor> {
    let group = {
        let mut tokens_iter = attr.clone().tokens.into_iter();
        let token = tokens_iter
            .next()
            .unwrap_or_else(|| panic!("#[{ARBITRARY_ATTRIBUTE_NAME}] cannot be empty."));
        match token {
            TokenTree::Group(g) => g,
            t => panic!("#[{ARBITRARY_ATTRIBUTE_NAME}] must contain a group, got: {t})"),
        }
    };
    parse_attribute_internals(group.stream())
}

fn parse_attribute_internals(stream: TokenStream) -> Result<FieldConstructor> {
    let mut tokens_iter = stream.into_iter();
    let token = tokens_iter
        .next()
        .unwrap_or_else(|| panic!("#[{ARBITRARY_ATTRIBUTE_NAME}] cannot be empty."));
    match token.to_string().as_ref() {
        "default" => Ok(FieldConstructor::Default),
        "with" => {
            let func_path = parse_assigned_value("with", tokens_iter)?;
            Ok(FieldConstructor::With(func_path))
        }
        "value" => {
            let value = parse_assigned_value("value", tokens_iter)?;
            Ok(FieldConstructor::Value(value))
        }
        _ => {
            let msg = format!("Unknown option for #[{ARBITRARY_ATTRIBUTE_NAME}]: `{token}`");
            Err(syn::Error::new(token.span(), msg))
        }
    }
}

// Input:
//     = 2 + 2
// Output:
//     2 + 2
fn parse_assigned_value(
    opt_name: &str,
    mut tokens_iter: impl Iterator<Item = TokenTree>,
) -> Result<TokenStream> {
    let eq_sign = tokens_iter.next().unwrap_or_else(|| {
        panic!("Invalid syntax for #[{ARBITRARY_ATTRIBUTE_NAME}], `{opt_name}` is missing RHS.")
    });
    if eq_sign.to_string() != "=" {
        panic!("Invalid syntax for #[{ARBITRARY_ATTRIBUTE_NAME}], expected `=` after `{opt_name}`, got: `{eq_sign}`");
    }
    Ok(tokens_iter.collect())
}
