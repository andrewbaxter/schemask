use {
    serde::{
        Deserialize,
        Serialize,
    },
    std::collections::HashMap,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Maskoid {
    /// Match null.
    Null,
    /// Match any string.
    String,
    /// Must be exactly this string.
    ConstString(String),
    /// Match any bool.
    Bool,
    /// Match any int.
    Int,
    /// Match any float.
    Float,
    /// Match the maskoid in the root `bindings` map with this name.
    Ref(String),
    /// The value may match the maskoid or it may be null. In a record, the associated
    /// key may also be missing. If options are nested, the inner option is treated
    /// like a single element record with the key "element".
    Option(Box<Maskoid>),
    /// Homogenous set where every element matches this maskoid. Unordered, no
    /// duplicates.
    Set(Box<Maskoid>),
    /// Homogenous list where every element matches this maskoid. Preserves order.
    List(Box<Maskoid>),
    /// Homogenous map where keys are strings and every value matches this maskoid.
    /// Preserves order.
    StringMap(Box<Maskoid>),
    /// Heterogenous list where the entries are matched pairwise with a same-length
    /// list of maskoids.
    Tuple(Vec<Maskoid>),
    /// Matches exactly one maskoid, based on a tag (the key). In json, this is encoded
    /// as a single key object `{KEY: ELEMENT}`.
    TaggedUnion(HashMap<String, Maskoid>),
    /// Each value is matched against the maskoid with the corresponding key. There's
    /// no match if there are extra elements or missing elements.
    Record(HashMap<String, Maskoid>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Schemask {
    pub bindings: HashMap<String, Maskoid>,
    /// Which bound element to validate against if none are explicitly specified
    pub default: Option<String>,
}
