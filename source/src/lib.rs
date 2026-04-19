use {
    serde::{
        Deserialize,
        Serialize,
    },
    std::collections::HashMap,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SchemaskElement {
    String,
    ConstString,
    Bool,
    ConstBool,
    Int,
    ConstInt,
    Float,
    ConstFloat,
    /// Use the schemask element in the root `bindings` map with this name.
    Ref(String),
    Option(Box<SchemaskElement>),
    /// Homogenous set, unordered, no duplicates
    Set(Box<SchemaskElement>),
    List(Box<SchemaskElement>),
    StringMap(Box<SchemaskElement>),
    Tuple(Vec<SchemaskElement>),
    TaggedUnion(HashMap<String, SchemaskElement>),
    Record(HashMap<String, SchemaskElement>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Schemask {
    pub bindings: HashMap<String, SchemaskElement>,
    /// Which bound element to validate against if none are explicitly specified
    pub default: Option<String>,
}
