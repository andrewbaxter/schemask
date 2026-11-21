use {
    crate::{
        Maskoid,
        v1::{
            MaskoidConstString,
            MaskoidElement,
            MaskoidRef,
            SchemaskV1,
        },
    },
    serde_json,
};

/// Escapes `*/` so it cannot prematurely close a block comment.
fn escape_jsdoc(s: &str) -> String {
    return s.replace("*/", "* /");
}

fn ts_key(key: &str) -> String {
    let mut chars = key.chars();
    let ident = match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
        },
        _ => false,
    };
    if ident {
        return key.to_string();
    }
    return serde_json::to_string(key).unwrap();
}

/// Generate typescript types that, if used to produce data, would serialize to
/// json that matches a schema.
pub fn generate_typescript(schema: &SchemaskV1) -> String {
    let mut sorted: Vec<_> = schema.bindings.iter().collect();
    sorted.sort_by_key(|(k, _)| k.as_str());
    return sorted.iter().map(|(name, maskoid)| {
        let doc = match maskoid.description() {
            None => String::new(),
            Some(d) => format!("/** {} */\n", escape_jsdoc(d)),
        };
        format!("{}type {} = {};\n", doc, name, ts_type(maskoid))
    }).collect::<Vec<_>>().join("\n");
}

/// Returns a TypeScript type expression for the given maskoid.
fn ts_type(maskoid: &Maskoid) -> String {
    return match maskoid {
        Maskoid::Null(_) => "null".to_string(),
        Maskoid::String(_) => "string".to_string(),
        Maskoid::ConstString(MaskoidConstString { value, .. }) => {
            serde_json::to_string(value.as_str()).unwrap()
        },
        Maskoid::Bool(_) => "boolean".to_string(),
        Maskoid::Int(_) | Maskoid::Float(_) => "number".to_string(),
        Maskoid::Any(_) => "unknown".to_string(),
        Maskoid::Ref(MaskoidRef { name, .. }) => name.clone(),
        Maskoid::Option(MaskoidElement { element: inner, .. }) => {
            if matches!(inner.as_ref(), Maskoid::Option(_)) {
                // Nested option: inner encoded as {"element": `<value>`}
                format!("{{ element: {} }} | null", ts_type(inner))
            } else {
                format!("{} | null", ts_type(inner))
            }
        },
        Maskoid::Set(MaskoidElement { element: inner, .. }) => format!("Array<{}>", ts_type(inner)),
        Maskoid::List(MaskoidElement { element: inner, .. }) => format!("Array<{}>", ts_type(inner)),
        Maskoid::StringMap(MaskoidElement { element: inner, .. }) => {
            format!("Record<string, {}>", ts_type(inner))
        },
        Maskoid::Tuple(m) => {
            let types: Vec<String> = m.elements.iter().map(|f| ts_type(&f.maskoid)).collect();
            format!("[{}]", types.join(", "))
        },
        Maskoid::TaggedUnion(m) => {
            let mut sorted: Vec<_> = m.variants.iter().collect();
            sorted.sort_by_key(|(k, _)| k.as_str());
            sorted.iter().map(|(vname, variant)| {
                let maybe_doc = match variant.description.as_deref() {
                    None => String::new(),
                    Some(d) => format!("/** {} */ ", escape_jsdoc(d)),
                };
                if matches!(variant.maskoid, Maskoid::Null(_)) {
                    return format!("{}{}", maybe_doc, serde_json::to_string(vname.as_str()).unwrap());
                }
                format!("{}{{ {}: {} }}", maybe_doc, ts_key(vname), ts_type(&variant.maskoid))
            }).collect::<Vec<_>>().join("\n  | ")
        },
        Maskoid::Record(r) => {
            let mut sorted: Vec<_> = r.fields.iter().collect();
            sorted.sort_by_key(|(k, _)| k.as_str());
            let field_strs: Vec<String> = sorted.iter().map(|(fname, field)| {
                let doc = match field.description.as_deref() {
                    None => String::new(),
                    Some(d) => format!("  /** {} */\n", escape_jsdoc(d)),
                };
                let key = ts_key(fname);
                return match &field.maskoid {
                    Maskoid::Option(MaskoidElement { element: inner, .. }) => {
                        if matches!(inner.as_ref(), Maskoid::Option(_)) {
                            format!("{}  {}?: {{ element: {} }} | null", doc, key, ts_type(inner))
                        } else {
                            format!("{}  {}?: {} | null", doc, key, ts_type(inner))
                        }
                    },
                    other => format!("{}  {}: {}", doc, key, ts_type(other)),
                };
            }).collect();
            format!("{{\n{};\n}}", field_strs.join(";\n"))
        },
    };
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::MaskoidField,
        std::collections::BTreeMap,
    };

    #[test]
    fn non_identifier_keys_are_quoted() {
        let mut fields = BTreeMap::new();
        fields.insert("avatar-url".to_string(), MaskoidField::new(Maskoid::option(Maskoid::string())));
        fields.insert("a b".to_string(), MaskoidField::new(Maskoid::int()));
        fields.insert("ok_1".to_string(), MaskoidField::new(Maskoid::int()));
        let mut variants = BTreeMap::new();
        variants.insert("my-tag".to_string(), MaskoidField::new(Maskoid::int()));
        let mut bindings = BTreeMap::new();
        bindings.insert("R".to_string(), Maskoid::record(fields));
        bindings.insert("U".to_string(), Maskoid::tagged_union(variants));
        let out = generate_typescript(&SchemaskV1 {
            bindings: bindings,
            default: None,
        });
        assert!(out.contains("\"avatar-url\"?: string | null"), "{}", out);
        assert!(out.contains("\"a b\": number"), "{}", out);
        assert!(out.contains("  ok_1: number"), "{}", out);
        assert!(out.contains("{ \"my-tag\": number }"), "{}", out);
    }
}
