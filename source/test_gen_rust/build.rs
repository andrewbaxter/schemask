use {
    schemask::{
        Maskoid,
        Schemask,
        MaskoidField,
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
    bindings.insert("Label".to_string(), Maskoid::string());
    bindings.insert("Active".to_string(), Maskoid::bool());
    bindings.insert("Count".to_string(), Maskoid::int());
    bindings.insert("Ratio".to_string(), Maskoid::float());
    // Collection aliases
    bindings.insert("Tags".to_string(), Maskoid::list(Maskoid::string()));
    bindings.insert(
        "Coords".to_string(),
        Maskoid::tuple(vec![MaskoidField::new(Maskoid::float()), MaskoidField::new(Maskoid::float())]),
    );
    bindings.insert("Meta".to_string(), Maskoid::string_map(Maskoid::string()));
    // Ref alias
    bindings.insert("Name".to_string(), Maskoid::ref_("Label"));
    // Record with required and optional fields
    bindings.insert(
        "Player".to_string(),
        Maskoid::record({
            let mut f = HashMap::new();
            f.insert("name".to_string(), MaskoidField::new(Maskoid::ref_("Label")));
            f.insert("score".to_string(), MaskoidField::new(Maskoid::option(Maskoid::int())));
            f.insert("tags".to_string(), MaskoidField::new(Maskoid::ref_("Tags")));
            f
        }),
    );
    // Tagged union
    bindings.insert(
        "Event".to_string(),
        Maskoid::tagged_union({
            let mut v = HashMap::new();
            v.insert("Join".to_string(), MaskoidField::new(Maskoid::ref_("Player")));
            v.insert("Leave".to_string(), MaskoidField::new(Maskoid::ref_("Label")));
            v
        }),
    );
    Schemask {
        bindings,
        default: Some("Player".to_string()),
    }
}
