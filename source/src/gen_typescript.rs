use crate::{
    Maskoid,
    Schemask,
};

/// Returns a TypeScript type expression for the given maskoid.
fn ts_type(maskoid: &Maskoid) -> String {
    match maskoid {
        Maskoid::Null => "null".to_string(),
        Maskoid::String => "string".to_string(),
        Maskoid::ConstString(s) => {
            format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
        },
        Maskoid::Bool => "boolean".to_string(),
        Maskoid::Int | Maskoid::Float => "number".to_string(),
        Maskoid::Ref(r) => r.clone(),
        Maskoid::Option(inner) => {
            if matches!(inner.as_ref(), Maskoid::Option(_)) {
                // Nested option: inner encoded as {"element": <value>}
                format!("{{ element: {} }} | null", ts_type(inner))
            } else {
                format!("{} | null", ts_type(inner))
            }
        },
        Maskoid::Set(inner) | Maskoid::List(inner) => {
            format!("Array<{}>", ts_type(inner))
        },
        Maskoid::StringMap(inner) => {
            format!("Record<string, {}>", ts_type(inner))
        },
        Maskoid::Tuple(maskoids) => {
            let types: Vec<String> = maskoids.iter().map(|m| ts_type(m)).collect();
            format!("[{}]", types.join(", "))
        },
        Maskoid::TaggedUnion(variants) => {
            let mut sorted: Vec<_> = variants.iter().collect();
            sorted.sort_by_key(|(k, _)| k.as_str());
            sorted
                .iter()
                .map(|(vname, vmaskoid)| format!("{{ {}: {} }}", vname, ts_type(vmaskoid)))
                .collect::<Vec<_>>()
                .join("\n  | ")
        },
        Maskoid::Record(fields) => {
            let mut sorted: Vec<_> = fields.iter().collect();
            sorted.sort_by_key(|(k, _)| k.as_str());
            let field_strs: Vec<String> = sorted
                .iter()
                .map(|(fname, fmaskoid)| match fmaskoid {
                    Maskoid::Option(inner) => {
                        if matches!(inner.as_ref(), Maskoid::Option(_)) {
                            // Nested option: optional field, value is wrapper | null
                            format!("  {}?: {{ element: {} }} | null", fname, ts_type(inner))
                        } else {
                            format!("  {}?: {} | null", fname, ts_type(inner))
                        }
                    },
                    other => format!("  {}: {}", fname, ts_type(other)),
                })
                .collect();
            format!("{{\n{};\n}}", field_strs.join(";\n"))
        },
    }
}

pub fn generate_typescript(schema: &Schemask) -> String {
    let mut sorted: Vec<_> = schema.bindings.iter().collect();
    sorted.sort_by_key(|(k, _)| k.as_str());
    sorted
        .iter()
        .map(|(name, maskoid)| format!("type {} = {};\n", name, ts_type(maskoid)))
        .collect::<Vec<_>>()
        .join("\n")
}
