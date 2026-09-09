use {
    crate::{
        Maskoid,
        v1::SchemaskV1,
    },
    serde_json,
    std::fmt::Write,
};

pub fn generate_markdown(schema: &SchemaskV1) -> String {
    let mut out = String::new();
    let mut sorted: Vec<_> = schema.bindings.iter().collect();
    sorted.sort_by_key(|(k, _)| k.as_str());
    for (name, maskoid) in sorted {
        writeln!(out, "## {}\n", name).unwrap();
        if let Some(desc) = maskoid.description() {
            writeln!(out, "{}\n", desc).unwrap();
        }
        match maskoid {
            Maskoid::Record(r) => {
                writeln!(out, "| Field | Type | Description |").unwrap();
                writeln!(out, "|-------|------|-------------|").unwrap();
                let mut sorted: Vec<_> = r.fields.iter().collect();
                sorted.sort_by_key(|(k, _)| k.as_str());
                for (fname, field) in sorted {
                    let ty = md_escape(&md_type(&field.maskoid));
                    let desc = md_escape(field.description.as_deref().unwrap_or(""));
                    writeln!(out, "| {} | {} | {} |", md_escape(fname), ty, desc).unwrap();
                }
            },
            Maskoid::TaggedUnion(u) => {
                writeln!(out, "| Variant | Type | Description |").unwrap();
                writeln!(out, "|---------|------|-------------|").unwrap();
                let mut sorted: Vec<_> = u.variants.iter().collect();
                sorted.sort_by_key(|(k, _)| k.as_str());
                for (vname, variant) in sorted {
                    let ty = match variant.maskoid {
                        Maskoid::Null => md_escape(&serde_json::to_string(vname.as_str()).unwrap()),
                        _ => md_escape(&md_type(&variant.maskoid)),
                    };
                    let desc = md_escape(variant.description.as_deref().unwrap_or(""));
                    writeln!(out, "| {} | {} | {} |", md_escape(vname), ty, desc).unwrap();
                }
            },
            other => writeln!(out, "{}", md_type(other)).unwrap(),
        }
        writeln!(out).unwrap();
    }
    return out;
}

fn md_escape(s: &str) -> String {
    return s.replace('|', "\\|").replace('\n', " ");
}

fn md_type(maskoid: &Maskoid) -> String {
    return match maskoid {
        Maskoid::Null => "null".to_string(),
        Maskoid::Any => "any".to_string(),
        Maskoid::String => "string".to_string(),
        Maskoid::ConstString(v) => serde_json::to_string(v.as_str()).unwrap(),
        Maskoid::Bool => "boolean".to_string(),
        Maskoid::Int => "integer".to_string(),
        Maskoid::Float => "number".to_string(),
        Maskoid::Ref(name) => format!("[{name}](#{name})", name = name.to_lowercase()),
        Maskoid::Option(inner) => format!("{} | null", md_type(inner)),
        Maskoid::Set(inner) => format!("Set<{}>", md_type(inner)),
        Maskoid::List(inner) => format!("Array<{}>", md_type(inner)),
        Maskoid::StringMap(inner) => format!("Record<string, {}>", md_type(inner)),
        Maskoid::Tuple(t) => {
            let types: Vec<String> = t.elements.iter().map(|f| md_type(&f.maskoid)).collect();
            return format!("[{}]", types.join(", "));
        },
        Maskoid::Record(_) => "object".to_string(),
        Maskoid::TaggedUnion(_) => "union".to_string(),
    };
}
