schemask::from_schemask!("../schema.json");

#[cfg(test)]
mod tests {
    use super::*;

    fn check(root: &str, value: &impl serde::Serialize) {
        let schema: schemask::Schemask = serde_json::from_str(include_str!("../schema.json")).unwrap();
        schemask::validate(&schema, Some(root.to_string()), &serde_json::to_value(value).unwrap()).expect("should validate");
    }

    fn sample_account() -> Account {
        Account {
            avatar_url: None,
            display_name: Some("Ann".to_string()),
            name: "ann".to_string(),
            score: Some(5),
            tags: vec!["chess".to_string()],
            r#type: "admin".to_string(),
        }
    }

    #[test]
    fn account_validates_against_schema() {
        check("Account", &sample_account());
    }

    #[test]
    fn account_renames_to_exact_keys() {
        let json = serde_json::to_value(sample_account()).unwrap();
        assert!(json.get("displayName").is_some());
        assert!(json.get("type").is_some());
        assert!(json.get("avatar-url").is_none());
        let back: Account = serde_json::from_value(json).unwrap();
        assert_eq!(back.name, "ann");
        assert_eq!(back.r#type, "admin");
    }

    #[test]
    fn account_denies_unknown_fields() {
        let extra = serde_json::json!({
            "name": "a",
            "type": "t",
            "tags": [],
            "bogus": 1
        });
        assert!(serde_json::from_value::<Account>(extra).is_err(), "unknown field should be rejected");
    }

    #[test]
    fn event_variant_renames() {
        check("Event", &Event::UserLeft("bob".to_string()));
        let json = serde_json::to_value(Event::UserLeft("bob".to_string())).unwrap();
        assert!(json.get("user_left").is_some(), "variant should serialize to its exact tag");
        check("Event", &Event::Join(sample_account()));
    }

    #[test]
    fn aliases_generated() {
        let _label: Label = "x".to_string();
        let _count: Count = 3;
        let _tags: Tags = vec!["a".to_string()];
    }
}
