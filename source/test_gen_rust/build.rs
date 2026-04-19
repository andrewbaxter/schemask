use {
    schemask::{
        Maskoid,
        Schemask,
        generate_rust,
    },
    std::{
        collections::HashMap,
        fs,
        path::Path,
    },
};

fn main() {
    let schema = test_schema();
    let code = generate_rust(&schema);
    let out_dir = std::env::var("OUT_DIR").unwrap();
    fs::write(Path::new(&out_dir).join("generated.rs"), code).unwrap();
}

pub fn test_schema() -> Schemask {
    let mut bindings = HashMap::new();
    // Primitive aliases
    bindings.insert("Label".to_string(), Maskoid::String);
    bindings.insert("Active".to_string(), Maskoid::Bool);
    bindings.insert("Count".to_string(), Maskoid::Int);
    bindings.insert("Ratio".to_string(), Maskoid::Float);
    // Collection aliases
    bindings.insert("Tags".to_string(), Maskoid::List(Box::new(Maskoid::String)));
    bindings.insert(
        "Coords".to_string(),
        Maskoid::Tuple(vec![Maskoid::Float, Maskoid::Float]),
    );
    bindings.insert(
        "Meta".to_string(),
        Maskoid::StringMap(Box::new(Maskoid::String)),
    );
    // Ref alias
    bindings.insert("Name".to_string(), Maskoid::Ref("Label".to_string()));
    // Record with required and optional fields
    bindings.insert(
        "Player".to_string(),
        Maskoid::Record({
            let mut f = HashMap::new();
            f.insert("name".to_string(), Maskoid::Ref("Label".to_string()));
            f.insert(
                "score".to_string(),
                Maskoid::Option(Box::new(Maskoid::Int)),
            );
            f.insert("tags".to_string(), Maskoid::Ref("Tags".to_string()));
            f
        }),
    );
    // Tagged union
    bindings.insert(
        "Event".to_string(),
        Maskoid::TaggedUnion({
            let mut v = HashMap::new();
            v.insert("Join".to_string(), Maskoid::Ref("Player".to_string()));
            v.insert("Leave".to_string(), Maskoid::Ref("Label".to_string()));
            v
        }),
    );
    Schemask {
        bindings,
        default: Some("Player".to_string()),
    }
}
