pub mod gen_markdown;
pub mod gen_rust;
pub mod gen_typescript;
pub mod v1;

use {
    schemask_derive::Maskoidy,
    serde::{
        Deserialize,
        Serialize,
    },
};
pub use {
    v1::{
        Invalid,
        Maskoid,
        MaskoidField,
        MaskoidRecord,
        MaskoidTaggedUnion,
        MaskoidTuple,
        Maskoidy,
        PathSegment,
        ValidationError,
    },
    v1 as latest,
};

/// Generate markdown documentation for a schemask.
pub fn generate_markdown(schema: &Schemask) -> String {
    match schema {
        Schemask::V1(s) => gen_markdown::generate_markdown(s),
    }
}

/// Generate rust types that would serialize to json that matches a schema.
pub fn generate_rust(schema: &Schemask) -> String {
    match schema {
        Schemask::V1(s) => gen_rust::generate_rust(s),
    }
}

pub fn generate_rust_tokens(schema: &Schemask) -> proc_macro2::TokenStream {
    match schema {
        Schemask::V1(s) => gen_rust::generate_rust_tokens(s),
    }
}

/// Generate typescript types that, if used to produce data, would serialize to
/// json that matches a schema.
pub fn generate_typescript(schema: &Schemask) -> String {
    match schema {
        Schemask::V1(s) => gen_typescript::generate_typescript(s),
    }
}

/// Versioned schemask enum. Currently only has a V1 variant.
#[derive(Serialize, Deserialize, Clone, Debug, Maskoidy)]
#[serde(rename_all = "snake_case")]
pub enum Schemask {
    V1(v1::SchemaskV1),
}

/// This is the main method for checking data against a schema. If the data is
/// valid it will return `Ok(())`, otherwise an error.
pub fn validate(schema: &Schemask, root: Option<String>, data: &serde_json::Value) -> Result<(), v1::Invalid> {
    match schema {
        Schemask::V1(s) => v1::validate(s, root, data),
    }
}
