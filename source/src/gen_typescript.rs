use crate::{
    Maskoid,
    MaskoidRecord,
    Schemask,
};

/// Returns a TypeScript type expression for the given maskoid.
fn ts_type(maskoid: &Maskoid) -> String {
    match maskoid {
        Maskoid::Null => "null".to_string(),
        Maskoid::String => "string".to_string(),
        Maskoid::ConstString(m) => {
            format!("\"{}\"", m.value.replace('\\', "\\\\").replace('"', "\\\""))
        },
        Maskoid::Bool => "boolean".to_string(),
        Maskoid::Int | Maskoid::Float => "number".to_string(),
        Maskoid::Ref(m) => m.name.clone(),
        Maskoid::Option(m) => {
            if matches!(m.inner.as_ref(), Maskoid::Option(_)) {
                // Nested option: inner encoded as {"element": <value>}
                format!("{{ element: {} }} | null", ts_type(&m.inner))
            } else {
                format!("{} | null", ts_type(&m.inner))
            }
        },
        Maskoid::Set(m) => format!("Array<{}>", ts_type(&m.inner)),
        Maskoid::List(m) => format!("Array<{}>", ts_type(&m.inner)),
        Maskoid::StringMap(m) => {
            format!("Record<string, {}>", ts_type(&m.inner))
        },
        Maskoid::Tuple(m) => {
            let types: Vec<String> = m.elements.iter().map(|f| ts_type(&f.maskoid)).collect();
            format!("[{}]", types.join(", "))
        },
        Maskoid::TaggedUnion(m) => {
            let mut sorted: Vec<_> = m.variants.iter().collect();
            sorted.sort_by_key(|(k, _)| k.as_str());
            sorted
                .iter()
                .map(|(vname, variant)| {
                    let maybe_doc = jsdoc_inline(variant.description.as_deref());
                    format!("{}{{ {}: {} }}", maybe_doc, vname, ts_type(&variant.maskoid))
                })
                .collect::<Vec<_>>()
                .join("\n  | ")
        },
        Maskoid::Record(r) => ts_record_type(r),
    }
}

/// Renders a record type literal `{ field: T; ... }` with per-field JSDoc.
fn ts_record_type(r: &MaskoidRecord) -> String {
    let mut sorted: Vec<_> = r.fields.iter().collect();
    sorted.sort_by_key(|(k, _)| k.as_str());
    let field_strs: Vec<String> = sorted
        .iter()
        .map(|(fname, field)| {
            let doc = match field.description.as_deref() {
                None => String::new(),
                Some(d) => format!("  /** {} */\n", d),
            };
            match &field.maskoid {
                Maskoid::Option(m) => {
                    if matches!(m.inner.as_ref(), Maskoid::Option(_)) {
                        format!("{}  {}?: {{ element: {} }} | null", doc, fname, ts_type(&m.inner))
                    } else {
                        format!("{}  {}?: {} | null", doc, fname, ts_type(&m.inner))
                    }
                },
                other => format!("{}  {}: {}", doc, fname, ts_type(other)),
            }
        })
        .collect();
    format!("{{\n{};\n}}", field_strs.join(";\n"))
}

/// Returns a brief `/** desc */` prefix if a description is set, or empty string.
fn jsdoc_inline(desc: Option<&str>) -> String {
    match desc {
        None => String::new(),
        Some(d) => format!("/** {} */ ", d),
    }
}

/// Returns a `/** desc */\n` block if a description is set, or empty string.
fn jsdoc_block(desc: Option<&str>) -> String {
    match desc {
        None => String::new(),
        Some(d) => format!("/** {} */\n", d),
    }
}

pub fn generate_typescript(schema: &Schemask) -> String {
    let mut sorted: Vec<_> = schema.bindings.iter().collect();
    sorted.sort_by_key(|(k, _)| k.as_str());
    sorted
        .iter()
        .map(|(name, maskoid)| {
            let doc = jsdoc_block(maskoid.description());
            format!("{}type {} = {};\n", doc, name, ts_type(maskoid))
        })
        .collect::<Vec<_>>()
        .join("\n")
}
