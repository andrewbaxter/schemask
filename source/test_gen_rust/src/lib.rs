include!(concat!(env!("OUT_DIR"), "/generated.rs"));
mod from_macro;

#[cfg(test)]
mod tests {
    use {
        super::*,
        schemask::{
            Maskoid,
            MaskoidField,
        },
        std::collections::HashMap,
    };

    fn check(root: &str, value: &impl serde::Serialize) {
        schemask::validate(&(|| -> schemask::latest::SchemaskV1 {
            // Mirror of the schema in build.rs.
            let mut bindings = HashMap::new();
            bindings.insert("Label".to_string(), Maskoid::string());
            bindings.insert("Active".to_string(), Maskoid::bool());
            bindings.insert("Count".to_string(), Maskoid::int());
            bindings.insert("Ratio".to_string(), Maskoid::float());
            bindings.insert("Tags".to_string(), Maskoid::list(Maskoid::string()));
            bindings.insert(
                "Coords".to_string(),
                Maskoid::tuple(vec![MaskoidField::new(Maskoid::float()), MaskoidField::new(Maskoid::float())]),
            );
            bindings.insert("Meta".to_string(), Maskoid::string_map(Maskoid::string()));
            bindings.insert("Name".to_string(), Maskoid::ref_("Label"));
            bindings.insert("Player".to_string(), Maskoid::record({
                let mut f = HashMap::new();
                f.insert("name".to_string(), MaskoidField::new(Maskoid::ref_("Label")));
                f.insert("score".to_string(), MaskoidField::new(Maskoid::option(Maskoid::int())));
                f.insert("tags".to_string(), MaskoidField::new(Maskoid::ref_("Tags")));
                f
            }));
            bindings.insert("Event".to_string(), Maskoid::tagged_union({
                let mut v = HashMap::new();
                v.insert("Join".to_string(), MaskoidField::new(Maskoid::ref_("Player")));
                v.insert("Leave".to_string(), MaskoidField::new(Maskoid::ref_("Label")));
                v
            }));
            return schemask::latest::SchemaskV1 {
                bindings: bindings,
                default: Some("Player".to_string()),
            };
        })().to_versioned(), Some(root.to_string()), &serde_json::to_value(value).unwrap()).unwrap();
    }

    #[test]
    fn test_label() {
        check("Label", &"hello".to_string());
    }

    #[test]
    fn test_active() {
        check("Active", &true);
    }

    #[test]
    fn test_count() {
        check("Count", &42i64);
    }

    #[test]
    fn test_ratio() {
        check("Ratio", &3.14f64);
    }

    #[test]
    fn test_name_ref() {
        check("Name", &"Alice".to_string());
    }

    #[test]
    fn test_tags() {
        check("Tags", &vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn test_coords() {
        check("Coords", &(1.0f64, 2.5f64));
    }

    #[test]
    fn test_meta() {
        let mut m = HashMap::new();
        m.insert("env".to_string(), "prod".to_string());
        check("Meta", &m);
    }

    #[test]
    fn test_player_no_score() {
        check("Player", &Player {
            name: "Alice".to_string(),
            score: None,
            tags: vec![],
        });
    }

    #[test]
    fn test_player_with_score() {
        check("Player", &Player {
            name: "Alice".to_string(),
            score: Some(99),
            tags: vec!["chess".to_string()],
        });
    }

    #[test]
    fn test_event_join() {
        check("Event", &Event::Join(Player {
            name: "Bob".to_string(),
            score: Some(10),
            tags: vec![],
        }));
    }

    #[test]
    fn test_event_leave() {
        check("Event", &Event::Leave("Bob".to_string()));
    }
}
