pub mod gen_rust;
pub use gen_rust::generate_rust;
pub mod gen_typescript;
pub use gen_typescript::generate_typescript;
pub use schemask_derive::Schematize;

use {
    serde::{
        Deserialize,
        Serialize,
    },
    std::{
        collections::HashMap,
        fmt,
    },
};

// ── Maskoid variant structs ───────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaskoidConstString {
    pub value: String,
}

/// References a named binding in the schema.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaskoidRef {
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaskoidOption {
    pub inner: Box<Maskoid>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaskoidSet {
    pub inner: Box<Maskoid>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaskoidList {
    pub inner: Box<Maskoid>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaskoidStringMap {
    pub inner: Box<Maskoid>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaskoidTuple {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub elements: Vec<MaskoidField>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaskoidField {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub maskoid: Maskoid,
}

impl MaskoidField {
    pub fn new(maskoid: Maskoid) -> Self {
        MaskoidField { description: None, maskoid }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaskoidTaggedUnion {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub variants: HashMap<String, MaskoidField>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaskoidRecord {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub fields: HashMap<String, MaskoidField>,
}

// ── Maskoid enum ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Maskoid {
    /// Match null.
    Null,
    /// Match any string.
    String,
    /// Must be exactly this string.
    ConstString(MaskoidConstString),
    /// Match any bool.
    Bool,
    /// Match any integer.
    Int,
    /// Match any float.
    Float,
    /// Match the maskoid in the root `bindings` map with this name.
    Ref(MaskoidRef),
    /// The value may match the maskoid or it may be null. In a record, the associated
    /// key may also be missing. If options are nested, the inner option is treated
    /// like a single element record with the key "element".
    Option(MaskoidOption),
    /// Homogenous set where every element matches this maskoid. Unordered, no duplicates.
    Set(MaskoidSet),
    /// Homogenous list where every element matches this maskoid. Preserves order.
    List(MaskoidList),
    /// Homogenous map where keys are strings and every value matches this maskoid.
    StringMap(MaskoidStringMap),
    /// Heterogenous list where the entries are matched pairwise with a same-length list of maskoids.
    Tuple(MaskoidTuple),
    /// Matches exactly one maskoid, based on a tag (the key). In json, this is encoded
    /// as a single key object `{KEY: ELEMENT}`.
    TaggedUnion(MaskoidTaggedUnion),
    /// Each value is matched against the maskoid with the corresponding key. There's
    /// no match if there are extra elements or missing elements.
    Record(MaskoidRecord),
}

impl Maskoid {
    // ── Constructors ─────────────────────────────────────────────────────────

    pub fn null() -> Self {
        Maskoid::Null
    }

    pub fn string() -> Self {
        Maskoid::String
    }

    pub fn const_string(value: impl Into<String>) -> Self {
        Maskoid::ConstString(MaskoidConstString { value: value.into() })
    }

    pub fn bool() -> Self {
        Maskoid::Bool
    }

    pub fn int() -> Self {
        Maskoid::Int
    }

    pub fn float() -> Self {
        Maskoid::Float
    }

    pub fn ref_(name: impl Into<String>) -> Self {
        Maskoid::Ref(MaskoidRef { name: name.into() })
    }

    pub fn option(inner: Maskoid) -> Self {
        Maskoid::Option(MaskoidOption { inner: Box::new(inner) })
    }

    pub fn set(inner: Maskoid) -> Self {
        Maskoid::Set(MaskoidSet { inner: Box::new(inner) })
    }

    pub fn list(inner: Maskoid) -> Self {
        Maskoid::List(MaskoidList { inner: Box::new(inner) })
    }

    pub fn string_map(inner: Maskoid) -> Self {
        Maskoid::StringMap(MaskoidStringMap { inner: Box::new(inner) })
    }

    pub fn tuple(elements: Vec<MaskoidField>) -> Self {
        Maskoid::Tuple(MaskoidTuple { description: None, elements: elements })
    }

    pub fn tagged_union(variants: HashMap<String, MaskoidField>) -> Self {
        Maskoid::TaggedUnion(MaskoidTaggedUnion { description: None, variants: variants })
    }

    pub fn record(fields: HashMap<String, MaskoidField>) -> Self {
        Maskoid::Record(MaskoidRecord { description: None, fields: fields })
    }

    // ── Description accessor and builder ─────────────────────────────────────

    pub fn description(&self) -> Option<&str> {
        match self {
            Maskoid::Tuple(m) => m.description.as_deref(),
            Maskoid::TaggedUnion(m) => m.description.as_deref(),
            Maskoid::Record(m) => m.description.as_deref(),
            _ => None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        let d = Some(desc.into());
        match &mut self {
            Maskoid::Tuple(m) => m.description = d,
            Maskoid::TaggedUnion(m) => m.description = d,
            Maskoid::Record(m) => m.description = d,
            _ => {},
        }
        return self;
    }
}

// ── Schema ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Schemask {
    pub bindings: HashMap<String, Maskoid>,
    /// Which bound element to validate against if none are explicitly specified
    pub default: Option<String>,
}

// ── Error types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    Key(String),
    Index(usize),
}

impl fmt::Display for PathSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathSegment::Key(k) => write!(f, ".{}", k),
            PathSegment::Index(i) => write!(f, "[{}]", i),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub path: Vec<PathSegment>,
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path_str: String = self.path.iter().map(|s| s.to_string()).collect();
        write!(
            f,
            "{}: {}",
            if path_str.is_empty() { "(root)".to_string() } else { path_str },
            self.message
        )
    }
}

#[derive(Debug, Clone)]
pub struct Invalid {
    pub errors: Vec<ValidationError>,
}

impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, e) in self.errors.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", e)?;
        }
        Ok(())
    }
}

impl std::error::Error for Invalid {}

// ── Validation ────────────────────────────────────────────────────────────────

pub fn r#match(schema: &Schemask, root: Option<String>, data: &serde_json::Value) -> Result<(), Invalid> {
    let binding_name = match root.or_else(|| schema.default.clone()) {
        Some(name) => name,
        None => {
            return Err(Invalid {
                errors: vec![ValidationError {
                    path: vec![],
                    message: "No root specified and schema has no default binding".to_string(),
                }],
            });
        },
    };
    let maskoid = match schema.bindings.get(&binding_name) {
        Some(m) => m,
        None => {
            return Err(Invalid {
                errors: vec![ValidationError {
                    path: vec![],
                    message: format!("Binding '{}' not found in schema", binding_name),
                }],
            });
        },
    };
    let mut errors = vec![];
    let mut path = vec![];
    match_maskoid(schema, maskoid, data, &mut path, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Invalid { errors })
    }
}

fn type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "int"
            } else {
                "float"
            }
        },
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn match_maskoid(
    schema: &Schemask,
    maskoid: &Maskoid,
    data: &serde_json::Value,
    path: &mut Vec<PathSegment>,
    errors: &mut Vec<ValidationError>,
) {
    match maskoid {
        Maskoid::Null => {
            if !data.is_null() {
                errors.push(ValidationError {
                    path: path.clone(),
                    message: format!("Expected null, got {}", type_name(data)),
                });
            }
        },
        Maskoid::String => {
            if !data.is_string() {
                errors.push(ValidationError {
                    path: path.clone(),
                    message: format!("Expected string, got {}", type_name(data)),
                });
            }
        },
        Maskoid::ConstString(m) => {
            match data.as_str() {
                Some(s) if s == m.value => {},
                Some(s) => {
                    errors.push(ValidationError {
                        path: path.clone(),
                        message: format!("Expected string {:?}, got {:?}", m.value, s),
                    });
                },
                None => {
                    errors.push(ValidationError {
                        path: path.clone(),
                        message: format!("Expected string {:?}, got {}", m.value, type_name(data)),
                    });
                },
            }
        },
        Maskoid::Bool => {
            if !data.is_boolean() {
                errors.push(ValidationError {
                    path: path.clone(),
                    message: format!("Expected bool, got {}", type_name(data)),
                });
            }
        },
        Maskoid::Int => {
            if !data.is_i64() && !data.is_u64() {
                errors.push(ValidationError {
                    path: path.clone(),
                    message: format!("Expected int, got {}", type_name(data)),
                });
            }
        },
        Maskoid::Float => {
            if !data.is_number() {
                errors.push(ValidationError {
                    path: path.clone(),
                    message: format!("Expected float, got {}", type_name(data)),
                });
            }
        },
        Maskoid::Ref(m) => {
            match schema.bindings.get(&m.name) {
                Some(inner) => {
                    match_maskoid(schema, inner, data, path, errors);
                },
                None => {
                    errors.push(ValidationError {
                        path: path.clone(),
                        message: format!("Ref to undefined binding '{}'", m.name),
                    });
                },
            }
        },
        Maskoid::Option(m) => {
            if !data.is_null() {
                // Nested options are encoded as {"element": ...} per spec
                if matches!(m.inner.as_ref(), Maskoid::Option(_)) {
                    match data.as_object() {
                        Some(obj) if obj.len() == 1 && obj.contains_key("element") => {
                            path.push(PathSegment::Key("element".to_string()));
                            match_maskoid(schema, &m.inner, obj.get("element").unwrap(), path, errors);
                            path.pop();
                        },
                        Some(obj) => {
                            errors.push(ValidationError {
                                path: path.clone(),
                                message: format!(
                                    "Expected nested option as {{\"element\": ...}}, got object with {} keys",
                                    obj.len()
                                ),
                            });
                        },
                        None => {
                            errors.push(ValidationError {
                                path: path.clone(),
                                message: format!(
                                    "Expected nested option as {{\"element\": ...}}, got {}",
                                    type_name(data)
                                ),
                            });
                        },
                    }
                } else {
                    match_maskoid(schema, &m.inner, data, path, errors);
                }
            }
        },
        Maskoid::Set(m) => {
            match data.as_array() {
                Some(arr) => {
                    for (i, elem) in arr.iter().enumerate() {
                        path.push(PathSegment::Index(i));
                        match_maskoid(schema, &m.inner, elem, path, errors);
                        path.pop();
                    }
                    for i in 0..arr.len() {
                        for j in (i + 1)..arr.len() {
                            if arr[i] == arr[j] {
                                path.push(PathSegment::Index(j));
                                errors.push(ValidationError {
                                    path: path.clone(),
                                    message: format!("Duplicate element (also at index {})", i),
                                });
                                path.pop();
                            }
                        }
                    }
                },
                None => {
                    errors.push(ValidationError {
                        path: path.clone(),
                        message: format!("Expected set (array), got {}", type_name(data)),
                    });
                },
            }
        },
        Maskoid::List(m) => {
            match data.as_array() {
                Some(arr) => {
                    for (i, elem) in arr.iter().enumerate() {
                        path.push(PathSegment::Index(i));
                        match_maskoid(schema, &m.inner, elem, path, errors);
                        path.pop();
                    }
                },
                None => {
                    errors.push(ValidationError {
                        path: path.clone(),
                        message: format!("Expected list (array), got {}", type_name(data)),
                    });
                },
            }
        },
        Maskoid::StringMap(m) => {
            match data.as_object() {
                Some(obj) => {
                    for (k, v) in obj {
                        path.push(PathSegment::Key(k.clone()));
                        match_maskoid(schema, &m.inner, v, path, errors);
                        path.pop();
                    }
                },
                None => {
                    errors.push(ValidationError {
                        path: path.clone(),
                        message: format!("Expected string map (object), got {}", type_name(data)),
                    });
                },
            }
        },
        Maskoid::Tuple(m) => {
            match data.as_array() {
                Some(arr) => {
                    if arr.len() != m.elements.len() {
                        errors.push(ValidationError {
                            path: path.clone(),
                            message: format!(
                                "Expected tuple of length {}, got length {}",
                                m.elements.len(),
                                arr.len()
                            ),
                        });
                    } else {
                        for (i, (elem, field)) in arr.iter().zip(m.elements.iter()).enumerate() {
                            path.push(PathSegment::Index(i));
                            match_maskoid(schema, &field.maskoid, elem, path, errors);
                            path.pop();
                        }
                    }
                },
                None => {
                    errors.push(ValidationError {
                        path: path.clone(),
                        message: format!("Expected tuple (array), got {}", type_name(data)),
                    });
                },
            }
        },
        Maskoid::TaggedUnion(m) => {
            match data.as_object() {
                Some(obj) if obj.len() == 1 => {
                    let (tag, value) = obj.iter().next().unwrap();
                    match m.variants.get(tag) {
                        Some(variant) => {
                            path.push(PathSegment::Key(tag.clone()));
                            match_maskoid(schema, &variant.maskoid, value, path, errors);
                            path.pop();
                        },
                        None => {
                            let known: Vec<_> = m.variants.keys().cloned().collect();
                            errors.push(ValidationError {
                                path: path.clone(),
                                message: format!(
                                    "Unknown union variant '{}', expected one of: {}",
                                    tag,
                                    known.join(", ")
                                ),
                            });
                        },
                    }
                },
                Some(obj) => {
                    errors.push(ValidationError {
                        path: path.clone(),
                        message: format!(
                            "Expected tagged union (single-key object), got object with {} keys",
                            obj.len()
                        ),
                    });
                },
                None => {
                    errors.push(ValidationError {
                        path: path.clone(),
                        message: format!(
                            "Expected tagged union (single-key object), got {}",
                            type_name(data)
                        ),
                    });
                },
            }
        },
        Maskoid::Record(m) => {
            match data.as_object() {
                Some(obj) => {
                    for (field_name, field) in &m.fields {
                        match obj.get(field_name) {
                            Some(v) => {
                                path.push(PathSegment::Key(field_name.clone()));
                                match_maskoid(schema, &field.maskoid, v, path, errors);
                                path.pop();
                            },
                            None => {
                                // Option fields may be absent from a record
                                if !matches!(field.maskoid, Maskoid::Option(_)) {
                                    errors.push(ValidationError {
                                        path: path.clone(),
                                        message: format!("Missing required field '{}'", field_name),
                                    });
                                }
                            },
                        }
                    }
                    for key in obj.keys() {
                        if !m.fields.contains_key(key) {
                            errors.push(ValidationError {
                                path: path.clone(),
                                message: format!("Unexpected field '{}'", key),
                            });
                        }
                    }
                },
                None => {
                    errors.push(ValidationError {
                        path: path.clone(),
                        message: format!("Expected record (object), got {}", type_name(data)),
                    });
                },
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        serde_json::json,
    };

    fn schema(maskoid: Maskoid) -> Schemask {
        let mut bindings = HashMap::new();
        bindings.insert("root".to_string(), maskoid);
        Schemask { bindings, default: Some("root".to_string()) }
    }

    fn pass(maskoid: Maskoid, data: serde_json::Value) {
        let s = schema(maskoid);
        assert!(r#match(&s, None, &data).is_ok(), "expected pass");
    }

    fn fail(maskoid: Maskoid, data: serde_json::Value, expected_path: &[PathSegment]) {
        let s = schema(maskoid);
        let err = r#match(&s, None, &data).expect_err("expected failure");
        assert_eq!(err.errors.len(), 1, "expected exactly one error, got: {:?}", err.errors);
        assert_eq!(err.errors[0].path, expected_path, "wrong error path");
    }

    #[test]
    fn test_null() {
        pass(Maskoid::null(), json!(null));
        fail(Maskoid::null(), json!("hello"), &[]);
    }

    #[test]
    fn test_string() {
        pass(Maskoid::string(), json!("hello"));
        fail(Maskoid::string(), json!(42), &[]);
    }

    #[test]
    fn test_const_string() {
        pass(Maskoid::const_string("hello"), json!("hello"));
        fail(Maskoid::const_string("hello"), json!("world"), &[]);
    }

    #[test]
    fn test_bool() {
        pass(Maskoid::bool(), json!(true));
        fail(Maskoid::bool(), json!(42), &[]);
    }

    #[test]
    fn test_int() {
        pass(Maskoid::int(), json!(42));
        fail(Maskoid::int(), json!("hello"), &[]);
    }

    #[test]
    fn test_float() {
        pass(Maskoid::float(), json!(3.14));
        fail(Maskoid::float(), json!("hello"), &[]);
    }

    #[test]
    fn test_ref() {
        let mut bindings = HashMap::new();
        bindings.insert("main".to_string(), Maskoid::ref_("other"));
        bindings.insert("other".to_string(), Maskoid::string());
        let s = Schemask { bindings, default: Some("main".to_string()) };
        assert!(r#match(&s, None, &json!("hello")).is_ok());
        let err = r#match(&s, None, &json!(42)).expect_err("expected failure");
        assert_eq!(err.errors.len(), 1);
        assert_eq!(err.errors[0].path, &[] as &[PathSegment]);
        fail(Maskoid::ref_("nonexistent"), json!("hello"), &[]);
    }

    #[test]
    fn test_option() {
        pass(Maskoid::option(Maskoid::string()), json!(null));
        pass(Maskoid::option(Maskoid::string()), json!("hello"));
        fail(Maskoid::option(Maskoid::string()), json!(42), &[]);
    }

    #[test]
    fn test_option_nested() {
        let m = Maskoid::option(Maskoid::option(Maskoid::string()));
        pass(m.clone(), json!(null));
        pass(m.clone(), json!({"element": null}));
        pass(m.clone(), json!({"element": "hello"}));
        fail(m.clone(), json!("hello"), &[]);
        fail(m.clone(), json!({"element": "hello", "extra": "world"}), &[]);
        fail(m.clone(), json!({"other": "hello"}), &[]);
        fail(m.clone(), json!({}), &[]);
    }

    #[test]
    fn test_option_record_absent() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), MaskoidField::new(Maskoid::string()));
        fields.insert("nickname".to_string(), MaskoidField::new(Maskoid::option(Maskoid::string())));
        pass(Maskoid::record(fields.clone()), json!({"name": "Alice"}));
        fail(
            Maskoid::record(fields.clone()),
            json!({"name": "Alice", "nickname": 42}),
            &[PathSegment::Key("nickname".to_string())],
        );
    }

    #[test]
    fn test_set() {
        pass(Maskoid::set(Maskoid::int()), json!([1, 2, 3]));
        fail(Maskoid::set(Maskoid::int()), json!([1, "two", 3]), &[PathSegment::Index(1)]);
        fail(Maskoid::set(Maskoid::int()), json!([1, 2, 1]), &[PathSegment::Index(2)]);
    }

    #[test]
    fn test_list() {
        pass(Maskoid::list(Maskoid::string()), json!(["a", "b", "c"]));
        fail(Maskoid::list(Maskoid::string()), json!(["a", 42, "c"]), &[PathSegment::Index(1)]);
    }

    #[test]
    fn test_string_map() {
        pass(Maskoid::string_map(Maskoid::int()), json!({"a": 1, "b": 2}));
        fail(
            Maskoid::string_map(Maskoid::int()),
            json!({"a": 1, "b": "two"}),
            &[PathSegment::Key("b".to_string())],
        );
    }

    #[test]
    fn test_tuple() {
        let t = || vec![MaskoidField::new(Maskoid::string()), MaskoidField::new(Maskoid::int())];
        pass(Maskoid::tuple(t()), json!(["hello", 42]));
        fail(Maskoid::tuple(t()), json!(["hello", "world"]), &[PathSegment::Index(1)]);
        fail(Maskoid::tuple(t()), json!(["hello", 42, "extra"]), &[]);
    }

    #[test]
    fn test_tagged_union() {
        let mut variants = HashMap::new();
        variants.insert("name".to_string(), MaskoidField::new(Maskoid::string()));
        variants.insert("age".to_string(), MaskoidField::new(Maskoid::int()));
        pass(Maskoid::tagged_union(variants.clone()), json!({"name": "Alice"}));
        fail(
            Maskoid::tagged_union(variants.clone()),
            json!({"name": 42}),
            &[PathSegment::Key("name".to_string())],
        );
        fail(Maskoid::tagged_union(variants.clone()), json!({"name": "Alice", "age": 30}), &[]);
        fail(Maskoid::tagged_union(variants.clone()), json!({}), &[]);
        fail(Maskoid::tagged_union(variants.clone()), json!({"unknown": "Alice"}), &[]);
    }

    #[test]
    fn test_record() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), MaskoidField::new(Maskoid::string()));
        fields.insert("age".to_string(), MaskoidField::new(Maskoid::int()));
        pass(Maskoid::record(fields.clone()), json!({"name": "Alice", "age": 30}));
        fail(Maskoid::record(fields.clone()), json!({"name": "Alice"}), &[]);
        fail(
            Maskoid::record(fields.clone()),
            json!({"name": "Alice", "age": 30, "extra": "oops"}),
            &[],
        );
    }

    #[test]
    fn test_nested_path_index_then_key() {
        let mut fields = HashMap::new();
        fields.insert("x".to_string(), MaskoidField::new(Maskoid::int()));
        let m = Maskoid::list(Maskoid::record(fields));
        fail(
            m,
            json!([{"x": 1}, {"x": "bad"}]),
            &[PathSegment::Index(1), PathSegment::Key("x".to_string())],
        );
    }

    #[test]
    fn test_nested_path_key_then_index() {
        let mut fields = HashMap::new();
        fields.insert("items".to_string(), MaskoidField::new(Maskoid::list(Maskoid::int())));
        let m = Maskoid::record(fields);
        fail(
            m,
            json!({"items": [1, 2, "bad"]}),
            &[PathSegment::Key("items".to_string()), PathSegment::Index(2)],
        );
    }

    #[test]
    fn test_no_root_no_default() {
        let s = Schemask { bindings: HashMap::new(), default: None };
        let err = r#match(&s, None, &json!(null)).expect_err("expected failure");
        assert_eq!(err.errors.len(), 1);
        assert_eq!(err.errors[0].path, &[] as &[PathSegment]);
    }
}

// ── Schematize trait ──────────────────────────────────────────────────────────

/// A type that can describe itself as a [`Maskoid`].
pub trait Schematize {
    fn maskoid() -> Maskoid;
}

impl Schematize for () {
    fn maskoid() -> Maskoid { Maskoid::null() }
}
impl Schematize for bool {
    fn maskoid() -> Maskoid { Maskoid::bool() }
}
impl Schematize for String {
    fn maskoid() -> Maskoid { Maskoid::string() }
}
impl Schematize for i8 {
    fn maskoid() -> Maskoid { Maskoid::int() }
}
impl Schematize for i16 {
    fn maskoid() -> Maskoid { Maskoid::int() }
}
impl Schematize for i32 {
    fn maskoid() -> Maskoid { Maskoid::int() }
}
impl Schematize for i64 {
    fn maskoid() -> Maskoid { Maskoid::int() }
}
impl Schematize for u8 {
    fn maskoid() -> Maskoid { Maskoid::int() }
}
impl Schematize for u16 {
    fn maskoid() -> Maskoid { Maskoid::int() }
}
impl Schematize for u32 {
    fn maskoid() -> Maskoid { Maskoid::int() }
}
impl Schematize for u64 {
    fn maskoid() -> Maskoid { Maskoid::int() }
}
impl Schematize for f32 {
    fn maskoid() -> Maskoid { Maskoid::float() }
}
impl Schematize for f64 {
    fn maskoid() -> Maskoid { Maskoid::float() }
}
impl<T: Schematize> Schematize for Option<T> {
    fn maskoid() -> Maskoid { Maskoid::option(T::maskoid()) }
}
impl<T: Schematize> Schematize for Vec<T> {
    fn maskoid() -> Maskoid { Maskoid::list(T::maskoid()) }
}
impl<T: Schematize> Schematize for Box<T> {
    fn maskoid() -> Maskoid { T::maskoid() }
}
impl<V: Schematize> Schematize for HashMap<String, V> {
    fn maskoid() -> Maskoid { Maskoid::string_map(V::maskoid()) }
}
