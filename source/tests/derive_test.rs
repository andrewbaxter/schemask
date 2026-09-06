use {
    schemask::{
        Maskoid,
        Schemask,
        Maskoidy,
        validate,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    serde_json::json,
};

// ── Types under test ──────────────────────────────────────────────────────────
/// A participant in a game session.
#[derive(Serialize, Deserialize, Maskoidy)]
struct Player {
    /// Display name shown in the lobby.
    name: String,
    /// Current score; absent until the player has scored.
    score: Option<i64>,
    tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Maskoidy)]
enum Event {
    /// A new player joined.
    Join(Player),
    Leave(String),
}

#[derive(Serialize, Deserialize, Maskoidy)]
struct Label(String);

#[derive(Serialize, Deserialize, Maskoidy)]
struct Coords(f64, f64);
#[derive(Serialize, Deserialize, Maskoidy)]
struct Unit;

#[derive(Serialize, Deserialize, Maskoidy)]
struct TreeNode {
    value: i64,
    children: Vec<TreeNode>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────
fn schema_for<T: Maskoidy>() -> Schemask {
    return T::schemask();
}

fn valid(schema: &Schemask, data: serde_json::Value) {
    validate(schema, None, &data).expect("expected valid");
}

fn invalid(schema: &Schemask, data: serde_json::Value) {
    validate(schema, None, &data).expect_err("expected invalid");
}

// ── Maskoid shape tests ───────────────────────────────────────────────────────
#[test]
fn player_maskoid_is_record() {
    let Schemask::V1(s) = Player::schemask();
    let m = &s.bindings["Player"];
    assert!(matches!(m, Maskoid::Record(_)));
    let Maskoid::Record(r) = m else {
        panic!()
    };
    assert_eq!(r.description.as_deref(), Some("A participant in a game session."));
    assert!(r.fields.contains_key("name"));
    assert!(r.fields.contains_key("score"));
    assert!(r.fields.contains_key("tags"));
    assert!(matches!(r.fields["name"].maskoid, Maskoid::String));
    assert_eq!(r.fields["name"].description.as_deref(), Some("Display name shown in the lobby."));
    assert!(matches!(r.fields["score"].maskoid, Maskoid::Option(_)));
    assert_eq!(r.fields["score"].description.as_deref(), Some("Current score; absent until the player has scored."));
    assert!(matches!(r.fields["tags"].maskoid, Maskoid::List(_)));
    assert_eq!(r.fields["tags"].description, None);
}

#[test]
fn event_maskoid_is_tagged_union() {
    let Schemask::V1(s) = Event::schemask();
    let m = &s.bindings["Event"];
    assert!(matches!(m, Maskoid::TaggedUnion(_)));
    let Maskoid::TaggedUnion(u) = m else {
        panic!()
    };
    assert!(u.variants.contains_key("Join"));
    assert!(u.variants.contains_key("Leave"));
    assert!(matches!(&u.variants["Join"].maskoid, Maskoid:: Ref(name) if name == "Player"));
    assert_eq!(u.variants["Join"].description.as_deref(), Some("A new player joined."));
    assert!(matches!(u.variants["Leave"].maskoid, Maskoid::String));
    assert_eq!(u.variants["Leave"].description, None);
}

#[test]
fn label_maskoid_is_string() {
    // Newtype wrapping String → String maskoid
    let Schemask::V1(s) = Label::schemask();
    assert!(matches!(s.bindings["Label"], Maskoid::String));
}

#[test]
fn coords_maskoid_is_tuple() {
    let Schemask::V1(s) = Coords::schemask();
    let m = &s.bindings["Coords"];
    assert!(matches!(m, Maskoid::Tuple(_)));
    let Maskoid::Tuple(t) = m else {
        panic!()
    };
    assert_eq!(t.elements.len(), 2);
    assert!(matches!(t.elements[0].maskoid, Maskoid::Float));
    assert!(matches!(t.elements[1].maskoid, Maskoid::Float));
}

#[test]
fn unit_maskoid_is_null() {
    let Schemask::V1(s) = Unit::schemask();
    assert!(matches!(s.bindings["Unit"], Maskoid::Null));
}

#[test]
fn schemask_schema_validates_itself() {
    let s = Schemask::schemask();
    validate(
        &s,
        None,
        &serde_json::to_value(&s).unwrap(),
    ).expect("the schemask schema should validate its own encoding");
}

#[derive(Serialize, Deserialize, Maskoidy)]
#[serde(rename_all = "snake_case")]
enum Status {
    Pending,
    Failed(String),
}

#[test]
fn unit_variant_is_bare_string() {
    let s = schema_for::<Status>();
    valid(&s, serde_json::to_value(&Status::Pending).unwrap());
    valid(&s, serde_json::to_value(&Status::Failed("nope".to_string())).unwrap());
    assert_eq!(serde_json::to_value(&Status::Pending).unwrap(), json!("pending"));
}

#[test]
fn unit_variant_object_form_matches_serde() {
    let round_trip = serde_json::from_value::<Status>(json!({
        "pending": null
    }));
    assert!(round_trip.is_ok(), "serde accepts the object form: {:?}", round_trip.err());
    valid(&schema_for::<Status>(), json!({
        "pending": null
    }));
}

#[test]
fn valued_variant_rejects_bare_string() {
    invalid(&schema_for::<Status>(), json!("failed"));
}

#[test]
fn unknown_bare_string_variant_rejected() {
    invalid(&schema_for::<Status>(), json!("nonexistent"));
}

#[derive(Serialize, Deserialize, Maskoidy)]
#[serde(rename_all = "camelCase")]
struct Renamed {
    display_name: String,
    #[serde(rename = "avatar-url")]
    avatar_url: String,
    r#type: String,
}

#[derive(Serialize, Deserialize, Maskoidy)]
#[serde(rename_all = "snake_case")]
enum RenamedEvent {
    UserLeft(String),
    #[serde(rename = "JOIN")]
    UserJoined(String),
}

#[derive(Serialize, Deserialize, Maskoidy)]
#[serde(rename_all = "snake_case", rename_all_fields = "kebab-case")]
enum RenamedFields {
    SomeVariant {
        inner_field: String,
    },
}

#[test]
fn struct_field_names_follow_serde() {
    let Schemask::V1(s) = Renamed::schemask();
    let Maskoid::Record(r) = &s.bindings["Renamed"] else {
        panic!()
    };
    let mut keys: Vec<&str> = r.fields.keys().map(|k| k.as_str()).collect();
    keys.sort();
    assert_eq!(keys, ["avatar-url", "displayName", "type"]);
    let json = serde_json::to_value(&Renamed {
        display_name: "a".to_string(),
        avatar_url: "b".to_string(),
        r#type: "c".to_string(),
    }).unwrap();
    let mut wire: Vec<&str> = json.as_object().unwrap().keys().map(|k| k.as_str()).collect();
    wire.sort();
    assert_eq!(wire, keys);
}

#[test]
fn variant_names_follow_serde() {
    let Schemask::V1(s) = RenamedEvent::schemask();
    let Maskoid::TaggedUnion(u) = &s.bindings["RenamedEvent"] else {
        panic!()
    };
    let mut keys: Vec<&str> = u.variants.keys().map(|k| k.as_str()).collect();
    keys.sort();
    assert_eq!(keys, ["JOIN", "user_left"]);
}

#[test]
fn variant_field_names_follow_rename_all_fields() {
    let Schemask::V1(s) = RenamedFields::schemask();
    let Maskoid::TaggedUnion(u) = &s.bindings["RenamedFields"] else {
        panic!()
    };
    let Maskoid::Record(r) = &u.variants["some_variant"].maskoid else {
        panic!()
    };
    assert!(r.fields.contains_key("inner-field"));
}

#[test]
fn renamed_struct_validates_against_its_own_schema() {
    valid(&schema_for::<Renamed>(), serde_json::to_value(&Renamed {
        display_name: "a".to_string(),
        avatar_url: "b".to_string(),
        r#type: "c".to_string(),
    }).unwrap());
}

// ── Validation round-trip tests ───────────────────────────────────────────────
#[test]
fn player_valid_no_score() {
    valid(&schema_for::<Player>(), json!({
        "name": "Alice",
        "tags":[]
    }));
}

#[test]
fn player_valid_with_score() {
    valid(&schema_for::<Player>(), json!({
        "name": "Alice",
        "score": 42,
        "tags":["chess"]
    }));
}

#[test]
fn player_valid_null_score() {
    valid(&schema_for::<Player>(), json!({
        "name": "Alice",
        "score": null,
        "tags":[]
    }));
}

#[test]
fn player_invalid_wrong_name_type() {
    invalid(&schema_for::<Player>(), json!({
        "name": 99,
        "tags":[]
    }));
}

#[test]
fn player_invalid_missing_tags() {
    invalid(&schema_for::<Player>(), json!({
        "name": "Alice"
    }));
}

#[test]
fn event_valid_join() {
    valid(&schema_for::<Event>(), json!({
        "Join": {
            "name": "Bob",
            "score": 10,
            "tags":[]
        }
    }));
}

#[test]
fn event_valid_leave() {
    valid(&schema_for::<Event>(), json!({
        "Leave": "Bob"
    }));
}

#[test]
fn event_invalid_unknown_variant() {
    invalid(&schema_for::<Event>(), json!({
        "Quit": "Bob"
    }));
}

#[test]
fn label_valid() {
    valid(&schema_for::<Label>(), json!("hello"));
}

#[test]
fn coords_valid() {
    valid(&schema_for::<Coords>(), json!([1.0, 2.5]));
}

#[test]
fn coords_invalid_wrong_length() {
    invalid(&schema_for::<Coords>(), json!([1.0]));
}

#[test]
fn tree_node_schemask_breaks_cycle() {
    let Schemask::V1(s) = TreeNode::schemask();

    // The schema should contain a "TreeNode" binding
    assert!(s.bindings.contains_key("TreeNode"), "missing TreeNode binding");
    let Maskoid::Record(r) = &s.bindings["TreeNode"] else {
        panic!("expected Record")
    };
    assert!(r.fields.contains_key("value"));
    assert!(r.fields.contains_key("children"));

    // children should be a list of Ref("TreeNode") — the cycle is broken
    let Maskoid::List(inner) = &r.fields["children"].maskoid else {
        panic!("expected List")
    };
    assert!(matches!(inner.as_ref(), Maskoid:: Ref(name) if name == "TreeNode"));
    assert_eq!(s.default, Some("TreeNode".to_string()));
    let node = TreeNode {
        value: 1,
        children: vec![TreeNode {
            value: 2,
            children: vec![],
        }],
    };
    valid(&TreeNode::schemask(), serde_json::to_value(&node).unwrap());
}
