pub mod gen_rust;
pub use gen_rust::generate_rust;

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
        Maskoid::ConstString(expected) => {
            match data.as_str() {
                Some(s) if s == expected => {},
                Some(s) => {
                    errors.push(ValidationError {
                        path: path.clone(),
                        message: format!("Expected string {:?}, got {:?}", expected, s),
                    });
                },
                None => {
                    errors.push(ValidationError {
                        path: path.clone(),
                        message: format!("Expected string {:?}, got {}", expected, type_name(data)),
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
        Maskoid::Tuple(maskoids) => {
            match data.as_array() {
                Some(arr) => {
                    if arr.len() != maskoids.len() {
                        errors.push(ValidationError {
                            path: path.clone(),
                            message: format!(
                                "Expected tuple of length {}, got length {}",
                                maskoids.len(),
                                arr.len()
                            ),
                        });
                    } else {
                        for (i, (elem, inner)) in arr.iter().zip(maskoids.iter()).enumerate() {
                            path.push(PathSegment::Index(i));
                            match_maskoid(schema, inner, elem, path, errors);
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
        Maskoid::TaggedUnion(variants) => {
            match data.as_object() {
                Some(obj) if obj.len() == 1 => {
                    let (tag, value) = obj.iter().next().unwrap();
                    match variants.get(tag) {
                        Some(inner) => {
                            path.push(PathSegment::Key(tag.clone()));
                            match_maskoid(schema, inner, value, path, errors);
                            path.pop();
                        },
                        None => {
                            let known: Vec<_> = variants.keys().cloned().collect();
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
        Maskoid::Record(fields) => {
            match data.as_object() {
                Some(obj) => {
                    for (field_name, field_maskoid) in fields {
                        match obj.get(field_name) {
                            Some(v) => {
                                path.push(PathSegment::Key(field_name.clone()));
                                match_maskoid(schema, field_maskoid, v, path, errors);
                                path.pop();
                            },
                            None => {
                                // Option fields may be absent from a record
                                if !matches!(field_maskoid, Maskoid::Option(_)) {
                                    errors.push(ValidationError {
                                        path: path.clone(),
                                        message: format!("Missing required field '{}'", field_name),
                                    });
                                }
                            },
                        }
                    }
                    for key in obj.keys() {
                        if !fields.contains_key(key) {
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
        pass(Maskoid::Null, json!(null));
        fail(Maskoid::Null, json!("hello"), &[]);
    }

    #[test]
    fn test_string() {
        pass(Maskoid::String, json!("hello"));
        fail(Maskoid::String, json!(42), &[]);
    }

    #[test]
    fn test_const_string() {
        pass(Maskoid::ConstString("hello".to_string()), json!("hello"));
        fail(Maskoid::ConstString("hello".to_string()), json!("world"), &[]);
    }

    #[test]
    fn test_bool() {
        pass(Maskoid::Bool, json!(true));
        fail(Maskoid::Bool, json!(42), &[]);
    }

    #[test]
    fn test_int() {
        pass(Maskoid::Int, json!(42));
        fail(Maskoid::Int, json!("hello"), &[]);
    }

    #[test]
    fn test_float() {
        pass(Maskoid::Float, json!(3.14));
        fail(Maskoid::Float, json!("hello"), &[]);
    }

    #[test]
    fn test_ref() {
        let mut bindings = HashMap::new();
        bindings.insert("main".to_string(), Maskoid::Ref("other".to_string()));
        bindings.insert("other".to_string(), Maskoid::String);
        let s = Schemask { bindings, default: Some("main".to_string()) };
        assert!(r#match(&s, None, &json!("hello")).is_ok());
        let err = r#match(&s, None, &json!(42)).expect_err("expected failure");
        assert_eq!(err.errors.len(), 1);
        assert_eq!(err.errors[0].path, &[] as &[PathSegment]);
        fail(Maskoid::Ref("nonexistent".to_string()), json!("hello"), &[]);
    }

    #[test]
    fn test_option() {
        pass(Maskoid::Option(Box::new(Maskoid::String)), json!(null));
        pass(Maskoid::Option(Box::new(Maskoid::String)), json!("hello"));
        fail(Maskoid::Option(Box::new(Maskoid::String)), json!(42), &[]);
    }

    #[test]
    fn test_option_nested() {
        let m = Maskoid::Option(Box::new(Maskoid::Option(Box::new(Maskoid::String))));
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
        fields.insert("name".to_string(), Maskoid::String);
        fields.insert("nickname".to_string(), Maskoid::Option(Box::new(Maskoid::String)));
        pass(Maskoid::Record(fields.clone()), json!({"name": "Alice"}));
        fail(Maskoid::Record(fields.clone()), json!({"name": "Alice", "nickname": 42}), &[PathSegment::Key("nickname".to_string())]);
    }

    #[test]
    fn test_set() {
        pass(Maskoid::Set(Box::new(Maskoid::Int)), json!([1, 2, 3]));
        fail(Maskoid::Set(Box::new(Maskoid::Int)), json!([1, "two", 3]), &[PathSegment::Index(1)]);
        fail(Maskoid::Set(Box::new(Maskoid::Int)), json!([1, 2, 1]), &[PathSegment::Index(2)]);
    }

    #[test]
    fn test_list() {
        pass(Maskoid::List(Box::new(Maskoid::String)), json!(["a", "b", "c"]));
        fail(Maskoid::List(Box::new(Maskoid::String)), json!(["a", 42, "c"]), &[PathSegment::Index(1)]);
    }

    #[test]
    fn test_string_map() {
        pass(Maskoid::StringMap(Box::new(Maskoid::Int)), json!({"a": 1, "b": 2}));
        fail(Maskoid::StringMap(Box::new(Maskoid::Int)), json!({"a": 1, "b": "two"}), &[PathSegment::Key("b".to_string())]);
    }

    #[test]
    fn test_tuple() {
        pass(Maskoid::Tuple(vec![Maskoid::String, Maskoid::Int]), json!(["hello", 42]));
        fail(Maskoid::Tuple(vec![Maskoid::String, Maskoid::Int]), json!(["hello", "world"]), &[PathSegment::Index(1)]);
        fail(Maskoid::Tuple(vec![Maskoid::String, Maskoid::Int]), json!(["hello", 42, "extra"]), &[]);
    }

    #[test]
    fn test_tagged_union() {
        let mut variants = HashMap::new();
        variants.insert("name".to_string(), Maskoid::String);
        variants.insert("age".to_string(), Maskoid::Int);
        pass(Maskoid::TaggedUnion(variants.clone()), json!({"name": "Alice"}));
        fail(Maskoid::TaggedUnion(variants.clone()), json!({"name": 42}), &[PathSegment::Key("name".to_string())]);
        fail(Maskoid::TaggedUnion(variants.clone()), json!({"name": "Alice", "age": 30}), &[]);
        fail(Maskoid::TaggedUnion(variants.clone()), json!({}), &[]);
        fail(Maskoid::TaggedUnion(variants.clone()), json!({"unknown": "Alice"}), &[]);
    }

    #[test]
    fn test_record() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), Maskoid::String);
        fields.insert("age".to_string(), Maskoid::Int);
        pass(Maskoid::Record(fields.clone()), json!({"name": "Alice", "age": 30}));
        fail(Maskoid::Record(fields.clone()), json!({"name": "Alice"}), &[]);
        fail(Maskoid::Record(fields.clone()), json!({"name": "Alice", "age": 30, "extra": "oops"}), &[]);
    }

    #[test]
    fn test_nested_path_index_then_key() {
        // Error inside a record nested under an array element: path = [1].x
        let mut fields = HashMap::new();
        fields.insert("x".to_string(), Maskoid::Int);
        let m = Maskoid::List(Box::new(Maskoid::Record(fields)));
        fail(
            m,
            json!([{"x": 1}, {"x": "bad"}]),
            &[PathSegment::Index(1), PathSegment::Key("x".to_string())],
        );
    }

    #[test]
    fn test_nested_path_key_then_index() {
        // Error inside an array nested under a record key: path = .items[2]
        let mut fields = HashMap::new();
        fields.insert("items".to_string(), Maskoid::List(Box::new(Maskoid::Int)));
        let m = Maskoid::Record(fields);
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
