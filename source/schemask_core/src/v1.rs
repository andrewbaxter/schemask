#[cfg(test)]
mod tests {
    use {
        serde_json::json,
        super::*,
    };

    fn fail(maskoid: Maskoid, data: serde_json::Value, expected_path: &[PathSegment]) {
        let err = validate(&schema(maskoid), None, &data).expect_err("expected failure");
        assert_eq!(err.errors.len(), 1, "expected exactly one error, got: {:?}", err.errors);
        assert_eq!(err.errors[0].path, expected_path, "wrong error path");
    }

    fn pass(maskoid: Maskoid, data: serde_json::Value) {
        let s = schema(maskoid);
        assert!(validate(&s, None, &data).is_ok(), "expected pass");
    }

    fn schema(maskoid: Maskoid) -> SchemaskV1 {
        let mut bindings = BTreeMap::new();
        bindings.insert("root".to_string(), maskoid);
        return SchemaskV1 {
            bindings: bindings,
            default: Some("root".to_string()),
        };
    }

    #[test]
    fn test_bool() {
        pass(Maskoid::bool(), json!(true));
        fail(Maskoid::bool(), json!(42), &[]);
    }

    #[test]
    fn test_const_string() {
        pass(Maskoid::const_string("hello"), json!("hello"));
        fail(Maskoid::const_string("hello"), json!("world"), &[]);
    }

    #[test]
    fn test_float() {
        pass(Maskoid::float(), json!(3.14));
        fail(Maskoid::float(), json!("hello"), &[]);
    }

    #[test]
    fn test_int() {
        pass(Maskoid::int(), json!(42));
        fail(Maskoid::int(), json!("hello"), &[]);
    }

    #[test]
    fn test_list() {
        pass(Maskoid::list(Maskoid::string()), json!(["a", "b", "c"]));
        fail(Maskoid::list(Maskoid::string()), json!(["a", 42, "c"]), &[PathSegment::Index(1)]);
    }

    #[test]
    fn test_nested_path_index_then_key() {
        let mut fields = BTreeMap::new();
        fields.insert("x".to_string(), MaskoidField::new(Maskoid::int()));
        fail(Maskoid::list(Maskoid::record(fields)), json!([{
            "x": 1
        }, {
            "x": "bad"
        }]), &[PathSegment::Index(1), PathSegment::Key("x".to_string())]);
    }

    #[test]
    fn test_nested_path_key_then_index() {
        let mut fields = BTreeMap::new();
        fields.insert("items".to_string(), MaskoidField::new(Maskoid::list(Maskoid::int())));
        fail(Maskoid::record(fields), json!({
            "items":[1, 2, "bad"]
        }), &[PathSegment::Key("items".to_string()), PathSegment::Index(2)]);
    }

    #[test]
    fn test_no_root_no_default() {
        let s = SchemaskV1 {
            bindings: BTreeMap::new(),
            default: None,
        };
        let err = validate(&s, None, &json!(null)).expect_err("expected failure");
        assert_eq!(err.errors.len(), 1);
        assert_eq!(err.errors[0].path, &[] as &[PathSegment]);
    }

    #[test]
    fn test_null() {
        pass(Maskoid::null(), json!(null));
        fail(Maskoid::null(), json!("hello"), &[]);
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
        pass(m.clone(), json!({
            "element": null
        }));
        pass(m.clone(), json!({
            "element": "hello"
        }));
        fail(m.clone(), json!("hello"), &[]);
        fail(m.clone(), json!({
            "element": "hello",
            "extra": "world"
        }), &[]);
        fail(m.clone(), json!({
            "other": "hello"
        }), &[]);
        fail(m.clone(), json!({ }), &[]);
    }

    #[test]
    fn test_option_record_absent() {
        let mut fields = BTreeMap::new();
        fields.insert("name".to_string(), MaskoidField::new(Maskoid::string()));
        fields.insert("nickname".to_string(), MaskoidField::new(Maskoid::option(Maskoid::string())));
        pass(Maskoid::record(fields.clone()), json!({
            "name": "Alice"
        }));
        fail(Maskoid::record(fields.clone()), json!({
            "name": "Alice",
            "nickname": 42
        }), &[PathSegment::Key("nickname".to_string())]);
    }

    #[test]
    fn test_record() {
        let mut fields = BTreeMap::new();
        fields.insert("name".to_string(), MaskoidField::new(Maskoid::string()));
        fields.insert("age".to_string(), MaskoidField::new(Maskoid::int()));
        pass(Maskoid::record(fields.clone()), json!({
            "name": "Alice",
            "age": 30
        }));
        fail(Maskoid::record(fields.clone()), json!({
            "name": "Alice"
        }), &[]);
        fail(Maskoid::record(fields.clone()), json!({
            "name": "Alice",
            "age": 30,
            "extra": "oops"
        }), &[]);
    }

    #[test]
    fn test_ref() {
        let mut bindings = BTreeMap::new();
        bindings.insert("main".to_string(), Maskoid::ref_("other"));
        bindings.insert("other".to_string(), Maskoid::string());
        let s = SchemaskV1 {
            bindings: bindings,
            default: Some("main".to_string()),
        };
        assert!(validate(&s, None, &json!("hello")).is_ok());
        let err = validate(&s, None, &json!(42)).expect_err("expected failure");
        assert_eq!(err.errors.len(), 1);
        assert_eq!(err.errors[0].path, &[] as &[PathSegment]);
        fail(Maskoid::ref_("nonexistent"), json!("hello"), &[]);
    }

    #[test]
    fn test_set() {
        pass(Maskoid::set(Maskoid::int()), json!([1, 2, 3]));
        fail(Maskoid::set(Maskoid::int()), json!([1, "two", 3]), &[PathSegment::Index(1)]);
        fail(Maskoid::set(Maskoid::int()), json!([1, 2, 1]), &[PathSegment::Index(2)]);
    }

    #[test]
    fn test_string() {
        pass(Maskoid::string(), json!("hello"));
        fail(Maskoid::string(), json!(42), &[]);
    }

    #[test]
    fn test_string_map() {
        pass(Maskoid::string_map(Maskoid::int()), json!({
            "a": 1,
            "b": 2
        }));
        fail(Maskoid::string_map(Maskoid::int()), json!({
            "a": 1,
            "b": "two"
        }), &[PathSegment::Key("b".to_string())]);
    }

    #[test]
    fn test_tagged_union() {
        let mut variants = BTreeMap::new();
        variants.insert("name".to_string(), MaskoidField::new(Maskoid::string()));
        variants.insert("age".to_string(), MaskoidField::new(Maskoid::int()));
        pass(Maskoid::tagged_union(variants.clone()), json!({
            "name": "Alice"
        }));
        fail(Maskoid::tagged_union(variants.clone()), json!({
            "name": 42
        }), &[PathSegment::Key("name".to_string())]);
        fail(Maskoid::tagged_union(variants.clone()), json!({
            "name": "Alice",
            "age": 30
        }), &[]);
        fail(Maskoid::tagged_union(variants.clone()), json!({ }), &[]);
        fail(Maskoid::tagged_union(variants.clone()), json!({
            "unknown": "Alice"
        }), &[]);
    }

    #[test]
    fn test_tuple() {
        let t = || vec![MaskoidField::new(Maskoid::string()), MaskoidField::new(Maskoid::int())];
        pass(Maskoid::tuple(t()), json!(["hello", 42]));
        fail(Maskoid::tuple(t()), json!(["hello", "world"]), &[PathSegment::Index(1)]);
        fail(Maskoid::tuple(t()), json!(["hello", 42, "extra"]), &[]);
    }
}

use {
    schemask_derive::Maskoidy,
    serde::{
        Deserialize,
        Serialize,
    },
    std::{
        collections::{
            BTreeMap,
            HashSet,
        },
        fmt,
    },
};

pub trait Maskoidy {
    fn maskoid(seen: &mut HashSet<&'static str>, bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid;

    /// A unique identifier for this type, used for cycle detection during schema
    /// generation. Derived types return `concat!(file!(), ":", line!())` at the
    /// definition site. Non-derived implementations return `""`.
    fn schema_id() -> &'static str {
        return "";
    }

    /// The name under which this type is registered as a binding during schema
    /// generation. Non-derived implementations return `""`.
    fn schema_name() -> &'static str {
        return "";
    }

    /// Build a [`Schemask`] rooted at this type. Derived types are registered as named
    /// bindings; recursive references are broken by returning a [`Maskoid::ref_`] on
    /// the second encounter of a type.
    fn schemask() -> crate::Schemask {
        let mut seen = HashSet::new();
        let mut bindings = BTreeMap::new();
        let _ = Self::maskoid(&mut seen, &mut bindings);
        let default = if Self::schema_name().is_empty() {
            None
        } else {
            Some(Self::schema_name().to_string())
        };
        return (SchemaskV1 {
            bindings: bindings,
            default: default,
        }).to_versioned();
    }
}

impl Maskoidy for () {
    fn maskoid(_seen: &mut HashSet<&'static str>, _bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::null();
    }
}

impl Maskoidy for bool {
    fn maskoid(_seen: &mut HashSet<&'static str>, _bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::bool();
    }
}

impl<T: Maskoidy> Maskoidy for Box<T> {
    fn maskoid(seen: &mut HashSet<&'static str>, bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return T::maskoid(seen, bindings);
    }
}

impl<V: Maskoidy> Maskoidy for BTreeMap<String, V> {
    fn maskoid(seen: &mut HashSet<&'static str>, bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::string_map(V::maskoid(seen, bindings));
    }
}

impl Maskoidy for f32 {
    fn maskoid(_seen: &mut HashSet<&'static str>, _bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::float();
    }
}

impl Maskoidy for f64 {
    fn maskoid(_seen: &mut HashSet<&'static str>, _bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::float();
    }
}

impl Maskoidy for i16 {
    fn maskoid(_seen: &mut HashSet<&'static str>, _bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::int();
    }
}

impl Maskoidy for i32 {
    fn maskoid(_seen: &mut HashSet<&'static str>, _bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::int();
    }
}

impl Maskoidy for i64 {
    fn maskoid(_seen: &mut HashSet<&'static str>, _bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::int();
    }
}

impl Maskoidy for i8 {
    fn maskoid(_seen: &mut HashSet<&'static str>, _bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::int();
    }
}

impl<T: Maskoidy> Maskoidy for Option<T> {
    fn maskoid(seen: &mut HashSet<&'static str>, bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::option(T::maskoid(seen, bindings));
    }
}

impl Maskoidy for serde_json::Value {
    fn maskoid(_seen: &mut HashSet<&'static str>, _bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::any();
    }
}

impl<V: Maskoidy> Maskoidy for std::collections::HashMap<String, V> {
    fn maskoid(seen: &mut HashSet<&'static str>, bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::string_map(V::maskoid(seen, bindings));
    }
}

impl Maskoidy for String {
    fn maskoid(_seen: &mut HashSet<&'static str>, _bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::string();
    }
}

impl Maskoidy for u16 {
    fn maskoid(_seen: &mut HashSet<&'static str>, _bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::int();
    }
}

impl Maskoidy for u32 {
    fn maskoid(_seen: &mut HashSet<&'static str>, _bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::int();
    }
}

impl Maskoidy for u64 {
    fn maskoid(_seen: &mut HashSet<&'static str>, _bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::int();
    }
}

impl Maskoidy for u8 {
    fn maskoid(_seen: &mut HashSet<&'static str>, _bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::int();
    }
}

impl<T: Maskoidy> Maskoidy for Vec<T> {
    fn maskoid(seen: &mut HashSet<&'static str>, bindings: &mut BTreeMap<String, Maskoid>) -> Maskoid {
        return Maskoid::list(T::maskoid(seen, bindings));
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
        return Ok(());
    }
}

impl std::error::Error for Invalid { }

#[derive(Serialize, Deserialize, Clone, Debug, Maskoidy)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Maskoid {
    /// Match any value without validation.
    Any,
    /// Match any bool.
    Bool,
    /// Must be exactly this string.
    ConstString(String),
    /// Match any float.
    Float,
    /// Match any integer.
    Int,
    /// Homogenous list where every element matches this maskoid. Preserves order.
    List(Box<Maskoid>),
    /// Match null.
    Null,
    /// The value may match the maskoid or it may be null. In a record, the associated
    /// key may also be missing. If options are nested, the inner option is treated
    /// like a single element record with the key "element".
    Option(Box<Maskoid>),
    /// Each value is matched against the maskoid with the corresponding key. There's
    /// no match if there are extra elements or missing elements.
    Record(MaskoidRecord),
    /// Match the maskoid in the root `bindings` map with this name.
    Ref(String),
    /// Homogenous set where every element matches this maskoid. Unordered, no
    /// duplicates.
    Set(Box<Maskoid>),
    /// Match any string.
    String,
    /// Homogenous map where keys are strings and every value matches this maskoid.
    StringMap(Box<Maskoid>),
    /// Matches exactly one maskoid, based on a tag (the key). In json, this is encoded
    /// as a single key object `{KEY: ELEMENT}`.
    TaggedUnion(MaskoidTaggedUnion),
    /// Heterogenous list where the entries are matched pairwise with a same-length
    /// list of maskoids.
    Tuple(MaskoidTuple),
}

impl Maskoid {
    pub fn any() -> Self {
        return Maskoid::Any;
    }

    pub fn bool() -> Self {
        return Maskoid::Bool;
    }

    pub fn const_string(value: impl Into<String>) -> Self {
        return Maskoid::ConstString(value.into());
    }

    pub fn description(&self) -> Option<&str> {
        return match self {
            Maskoid::Tuple(m) => m.description.as_deref(),
            Maskoid::TaggedUnion(m) => m.description.as_deref(),
            Maskoid::Record(m) => m.description.as_deref(),
            _ => None,
        };
    }

    pub fn float() -> Self {
        return Maskoid::Float;
    }

    pub fn int() -> Self {
        return Maskoid::Int;
    }

    pub fn list(inner: Maskoid) -> Self {
        return Maskoid::List(Box::new(inner));
    }

    pub fn null() -> Self {
        return Maskoid::Null;
    }

    pub fn option(inner: Maskoid) -> Self {
        return Maskoid::Option(Box::new(inner));
    }

    pub fn record(fields: BTreeMap<String, MaskoidField>) -> Self {
        return Maskoid::Record(MaskoidRecord {
            description: None,
            fields: fields,
        });
    }

    pub fn ref_(name: impl Into<String>) -> Self {
        return Maskoid::Ref(name.into());
    }

    pub fn set(inner: Maskoid) -> Self {
        return Maskoid::Set(Box::new(inner));
    }

    pub fn string() -> Self {
        return Maskoid::String;
    }

    pub fn string_map(inner: Maskoid) -> Self {
        return Maskoid::StringMap(Box::new(inner));
    }

    pub fn tagged_union(variants: BTreeMap<String, MaskoidField>) -> Self {
        return Maskoid::TaggedUnion(MaskoidTaggedUnion {
            description: None,
            variants: variants,
        });
    }

    pub fn tuple(elements: Vec<MaskoidField>) -> Self {
        return Maskoid::Tuple(MaskoidTuple {
            description: None,
            elements: elements,
        });
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        let d = Some(desc.into());
        match &mut self {
            Maskoid::Tuple(m) => m.description = d,
            Maskoid::TaggedUnion(m) => m.description = d,
            Maskoid::Record(m) => m.description = d,
            _ => { },
        }
        return self;
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Maskoidy)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MaskoidField {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub maskoid: Maskoid,
}

impl MaskoidField {
    pub fn new(maskoid: Maskoid) -> Self {
        return MaskoidField {
            description: None,
            maskoid: maskoid,
        };
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        return self;
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Maskoidy)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MaskoidRecord {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub fields: BTreeMap<String, MaskoidField>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Maskoidy)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MaskoidTaggedUnion {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub variants: BTreeMap<String, MaskoidField>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Maskoidy)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MaskoidTuple {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub elements: Vec<MaskoidField>,
}

pub fn match_maskoid(
    schema: &SchemaskV1,
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
        Maskoid::ConstString(value) => {
            match data.as_str() {
                Some(s) if s == value => { },
                Some(s) => {
                    errors.push(ValidationError {
                        path: path.clone(),
                        message: format!("Expected string {:?}, got {:?}", value, s),
                    });
                },
                None => {
                    errors.push(ValidationError {
                        path: path.clone(),
                        message: format!("Expected string {:?}, got {}", value, type_name(data)),
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
        Maskoid::Any => { },
        Maskoid::Ref(name) => {
            match schema.bindings.get(name) {
                Some(inner) => {
                    match_maskoid(schema, inner, data, path, errors);
                },
                None => {
                    errors.push(ValidationError {
                        path: path.clone(),
                        message: format!("Ref to undefined binding '{}'", name),
                    });
                },
            }
        },
        Maskoid::Option(inner) => {
            if !data.is_null() {
                // Nested options are encoded as {"element": ...} per spec
                if matches!(inner.as_ref(), Maskoid::Option(_)) {
                    match data.as_object() {
                        Some(obj) if obj.len() == 1 && obj.contains_key("element") => {
                            path.push(PathSegment::Key("element".to_string()));
                            match_maskoid(schema, inner, obj.get("element").unwrap(), path, errors);
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
                    match_maskoid(schema, inner, data, path, errors);
                }
            }
        },
        Maskoid::Set(inner) => {
            match data.as_array() {
                Some(arr) => {
                    for (i, elem) in arr.iter().enumerate() {
                        path.push(PathSegment::Index(i));
                        match_maskoid(schema, inner, elem, path, errors);
                        path.pop();
                    }
                    for i in 0 .. arr.len() {
                        for j in (i + 1) .. arr.len() {
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
        Maskoid::List(inner) => {
            match data.as_array() {
                Some(arr) => {
                    for (i, elem) in arr.iter().enumerate() {
                        path.push(PathSegment::Index(i));
                        match_maskoid(schema, inner, elem, path, errors);
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
        Maskoid::StringMap(inner) => {
            match data.as_object() {
                Some(obj) => {
                    for (k, v) in obj {
                        path.push(PathSegment::Key(k.clone()));
                        match_maskoid(schema, inner, v, path, errors);
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
            if let Some(tag) = data.as_str() {
                match m.variants.get(tag) {
                    Some(variant) if matches!(variant.maskoid, Maskoid::Null) => { },
                    Some(_) => {
                        errors.push(ValidationError {
                            path: path.clone(),
                            message: format!("Union variant '{}' carries a value, expected a single-key object", tag),
                        });
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
                return;
            }
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
                            "Expected tagged union (single-key object, or variant name for a valueless variant), got {}",
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

#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    Index(usize),
    Key(String),
}

impl fmt::Display for PathSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return match self {
            PathSegment::Key(k) => write!(f, ".{}", k),
            PathSegment::Index(i) => write!(f, "[{}]", i),
        };
    }
}

/// This is the main type used for validation. It describes the structure of one or
/// more related data types.
#[derive(Serialize, Deserialize, Clone, Debug, Maskoidy)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchemaskV1 {
    /// Bindings are names associated with parts of the schema. The names are used to
    /// identify where to start validation, as well as allow types to refer to other
    /// types (or themselves, recursively) forming a graph.
    pub bindings: BTreeMap<String, Maskoid>,
    /// Which bound element to validate against if none are explicitly specified
    pub default: Option<String>,
}

impl SchemaskV1 {
    pub fn to_versioned(self) -> crate::Schemask {
        return crate::Schemask::V1(self);
    }
}

fn type_name(v: &serde_json::Value) -> &'static str {
    return match v {
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
    };
}

/// This is the main method for checking data against a schema. If the data is
/// valid it will return `Ok(())`, otherwise an error.
pub fn validate(schema: &SchemaskV1, root: Option<String>, data: &serde_json::Value) -> Result<(), Invalid> {
    let binding_name = match root.or_else(|| schema.default.clone()) {
        Some(name) => name,
        None => {
            return Err(Invalid { errors: vec![ValidationError {
                path: vec![],
                message: "No root specified and schema has no default binding".to_string(),
            }] });
        },
    };
    let maskoid = match schema.bindings.get(&binding_name) {
        Some(m) => m,
        None => {
            return Err(Invalid { errors: vec![ValidationError {
                path: vec![],
                message: format!("Binding '{}' not found in schema", binding_name),
            }] });
        },
    };
    let mut errors = vec![];
    let mut path = vec![];
    match_maskoid(schema, maskoid, data, &mut path, &mut errors);
    return if errors.is_empty() {
        Ok(())
    } else {
        Err(Invalid { errors: errors })
    };
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
    pub path: Vec<PathSegment>,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path_str: String = self.path.iter().map(|s| s.to_string()).collect();
        return write!(f, "{}: {}", if path_str.is_empty() {
            "(root)".to_string()
        } else {
            path_str
        }, self.message);
    }
}
