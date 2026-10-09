use {
    schemask::{
        Maskoid,
        MaskoidField,
        gen_java::JavaConfig,
        generate_java,
    },
    std::{
        collections::BTreeMap,
        fs,
        path::Path,
    },
};

fn main() {
    let schema = (|| -> schemask::latest::SchemaskV1 {
        let mut bindings = BTreeMap::new();
        bindings.insert("Label".to_string(), Maskoid::string());
        bindings.insert("Tags".to_string(), Maskoid::list(Maskoid::string()));
        bindings.insert("Unique".to_string(), Maskoid::set(Maskoid::int()));
        bindings.insert("Meta".to_string(), Maskoid::string_map(Maskoid::string()));
        bindings.insert("Maybe".to_string(), Maskoid::option(Maskoid::option(Maskoid::int())));
        bindings.insert(
            "Coords".to_string(),
            Maskoid::tuple(vec![MaskoidField::new(Maskoid::float()), MaskoidField::new(Maskoid::float())]),
        );
        bindings.insert("Account".to_string(), Maskoid::record({
            let mut f = BTreeMap::new();
            f.insert("name".to_string(), MaskoidField::new(Maskoid::ref_("Label")));
            f.insert("type".to_string(), MaskoidField::new(Maskoid::const_string("user")));
            f.insert("avatar-url".to_string(), MaskoidField::new(Maskoid::option(Maskoid::string())));
            f.insert("score".to_string(), MaskoidField::new(Maskoid::option(Maskoid::int())));
            f.insert("active".to_string(), MaskoidField::new(Maskoid::bool()));
            f.insert("tags".to_string(), MaskoidField::new(Maskoid::ref_("Tags")));
            f.insert("extra".to_string(), MaskoidField::new(Maskoid::any()));
            f.insert("home".to_string(), MaskoidField::new(Maskoid::record({
                let mut h = BTreeMap::new();
                h.insert("city".to_string(), MaskoidField::new(Maskoid::string()));
                h
            })));
            f
        }));
        bindings.insert("Event".to_string(), Maskoid::tagged_union({
            let mut v = BTreeMap::new();
            v.insert("Join".to_string(), MaskoidField::new(Maskoid::ref_("Account")));
            v.insert("user_left".to_string(), MaskoidField::new(Maskoid::ref_("Label")));
            v.insert("Ping".to_string(), MaskoidField::new(Maskoid::null()));
            v.insert("Moved".to_string(), MaskoidField::new(Maskoid::ref_("Coords")));
            v
        }));
        bindings.insert("Shape".to_string(), Maskoid::tagged_union({
            let mut v = BTreeMap::new();
            v.insert("circle".to_string(), MaskoidField::new(Maskoid::record({
                let mut f = BTreeMap::new();
                f.insert("radius".to_string(), MaskoidField::new(Maskoid::float()));
                f
            })));
            v.insert("named".to_string(), MaskoidField::new(Maskoid::ref_("Label")));
            v
        }));
        return schemask::latest::SchemaskV1 {
            bindings: bindings,
            default: Some("Event".to_string()),
        };
    })();
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out = Path::new(&out_dir);
    let versioned = schema.to_versioned();
    fs::write(out.join("schema.json"), serde_json::to_string_pretty(&versioned).unwrap()).unwrap();
    let java = out.join("java");
    if java.exists() {
        fs::remove_dir_all(&java).unwrap();
    }
    for file in generate_java(&versioned, &JavaConfig {
        package: "gen".to_string(),
        runtime_package: "rt.json".to_string(),
        prelude_package: "rt.util".to_string(),
    }) {
        let path = java.join(&file.path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, file.source).unwrap();
    }
    println!("cargo:rerun-if-changed=build.rs");
}
