use {
    schemask::{
        Maskoid,
        MaskoidElement,
        MaskoidRef,
        Maskoidy,
        Schemask,
        validate,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    serde_json::json,
};

#[derive(Serialize, Deserialize, Maskoidy)]
struct Coords(f64, f64);

#[test]
fn coords_invalid_wrong_length() {
    invalid(&schema_for::<Coords>(), json!([1.0]));
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
    assert!(matches!(t.elements[0].maskoid, Maskoid::Float(_)));
    assert!(matches!(t.elements[1].maskoid, Maskoid::Float(_)));
}

#[test]
fn coords_valid() {
    valid(&schema_for::<Coords>(), json!([1.0, 2.5]));
}

#[derive(Serialize, Deserialize, Maskoidy)]
enum Event {
    /// A new player joined.
    Join(Player),
    Leave(String),
}

#[test]
fn event_invalid_unknown_variant() {
    invalid(&schema_for::<Event>(), json!({
        "Quit": "Bob"
    }));
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
    assert!(matches!(&u.variants["Join"].maskoid, Maskoid:: Ref(MaskoidRef { name, .. }) if name == "Player"));
    assert_eq!(u.variants["Join"].description.as_deref(), Some("A new player joined."));
    assert!(matches!(u.variants["Leave"].maskoid, Maskoid::String(_)));
    assert_eq!(u.variants["Leave"].description, None);
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

fn invalid(schema: &Schemask, data: serde_json::Value) {
    validate(schema, None, &data).expect_err("expected invalid");
}

#[derive(Serialize, Deserialize, Maskoidy)]
struct Label(String);

#[test]
fn label_maskoid_is_string() {
    // Newtype wrapping String → String maskoid
    let Schemask::V1(s) = Label::schemask();
    assert!(matches!(s.bindings["Label"], Maskoid::String(_)));
}

#[test]
fn label_valid() {
    valid(&schema_for::<Label>(), json!("hello"));
}

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

#[test]
fn player_invalid_missing_tags() {
    invalid(&schema_for::<Player>(), json!({
        "name": "Alice"
    }));
}

#[test]
fn player_invalid_wrong_name_type() {
    invalid(&schema_for::<Player>(), json!({
        "name": 99,
        "tags":[]
    }));
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
    assert!(matches!(r.fields["name"].maskoid, Maskoid::String(_)));
    assert_eq!(r.fields["name"].description.as_deref(), Some("Display name shown in the lobby."));
    assert!(matches!(r.fields["score"].maskoid, Maskoid::Option(_)));
    assert_eq!(r.fields["score"].description.as_deref(), Some("Current score; absent until the player has scored."));
    assert!(matches!(r.fields["tags"].maskoid, Maskoid::List(_)));
    assert_eq!(r.fields["tags"].description, None);
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
fn player_valid_null_score() {
    valid(&schema_for::<Player>(), json!({
        "name": "Alice",
        "score": null,
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

#[derive(Serialize, Deserialize, Maskoidy)]
#[serde(rename_all = "camelCase")]
struct Renamed {
    #[serde(rename = "avatar-url")]
    avatar_url: String,
    display_name: String,
    r#type: String,
}

#[test]
fn renamed_struct_validates_against_its_own_schema() {
    valid(&schema_for::<Renamed>(), serde_json::to_value(&Renamed {
        display_name: "a".to_string(),
        avatar_url: "b".to_string(),
        r#type: "c".to_string(),
    }).unwrap());
}

#[derive(Serialize, Deserialize, Maskoidy)]
#[serde(rename_all = "snake_case")]
enum RenamedEvent {
    #[serde(rename = "JOIN")]
    UserJoined(String),
    UserLeft(String),
}

#[derive(Serialize, Deserialize, Maskoidy)]
#[serde(rename_all = "snake_case", rename_all_fields = "kebab-case")]
enum RenamedFields {
    SomeVariant {
        inner_field: String,
    },
}

// ── Helpers ───────────────────────────────────────────────────────────────────
fn schema_for<T: Maskoidy>() -> Schemask {
    return T::schemask();
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
    Failed(String),
    Pending,
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
    let Maskoid::List(MaskoidElement { element: inner, .. }) = &r.fields["children"].maskoid else {
        panic!("expected List")
    };
    assert!(matches!(inner.as_ref(), Maskoid:: Ref(MaskoidRef { name, .. }) if name == "TreeNode"));
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

#[derive(Serialize, Deserialize, Maskoidy)]
struct TreeNode {
    children: Vec<TreeNode>,
    value: i64,
}
#[derive(Serialize, Deserialize, Maskoidy)]
struct Unit;

#[test]
fn unit_maskoid_is_null() {
    let Schemask::V1(s) = Unit::schemask();
    assert!(matches!(s.bindings["Unit"], Maskoid::Null(_)));
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
fn unknown_bare_string_variant_rejected() {
    invalid(&schema_for::<Status>(), json!("nonexistent"));
}

fn valid(schema: &Schemask, data: serde_json::Value) {
    validate(schema, None, &data).expect("expected valid");
}

#[test]
fn valued_variant_rejects_bare_string() {
    invalid(&schema_for::<Status>(), json!("failed"));
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
fn variant_names_follow_serde() {
    let Schemask::V1(s) = RenamedEvent::schemask();
    let Maskoid::TaggedUnion(u) = &s.bindings["RenamedEvent"] else {
        panic!()
    };
    let mut keys: Vec<&str> = u.variants.keys().map(|k| k.as_str()).collect();
    keys.sort();
    assert_eq!(keys, ["JOIN", "user_left"]);
}

#[derive(Serialize, Deserialize, Maskoidy)]
struct Nested {
    v: Option<schemask::SOption<i64>>,
}

#[test]
fn soption_matches_nested_option_encoding() {
    use schemask::SOption;

    let s = Nested::schemask();
    let Schemask::V1(v1) = &s;
    let Maskoid::Record(r) = &v1.bindings["Nested"] else {
        panic!()
    };
    let Maskoid::Option(MaskoidElement { element: inner, .. }) = &r.fields["v"].maskoid else {
        panic!()
    };
    assert!(matches!(inner.as_ref(), Maskoid::Option(_)));
    for (value, expected) in [(Nested { v: None }, json!({
        "v": null
    })), (Nested { v: Some(SOption::None) }, json!({
        "v": {
            "element": null
        }
    })), (Nested { v: Some(SOption::Some(5)) }, json!({
        "v": {
            "element": 5
        }
    }))] {
        let encoded = serde_json::to_value(&value).unwrap();
        assert_eq!(encoded, expected);
        valid(&s, encoded.clone());
        let back: Nested = serde_json::from_value(encoded).unwrap();
        assert_eq!(back.v, value.v);
    }
    assert!(serde_json::from_value::<Nested>(json!({
        "v": {
            "element": 1,
            "x": 2
        }
    })).is_err());
}

#[test]
#[should_panic(expected = "Option<Option<T>> is ambiguous")]
fn option_option_panics_at_schema_time() {
    <Option<Option<i64>> as Maskoidy>::schemask();
}

#[derive(Serialize, Deserialize, Maskoidy)]
struct Skips {
    a: i64,
    #[serde(skip)]
    b: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    c: Option<i64>,
}

#[test]
fn skipped_fields_are_omitted_from_schema() {
    let Schemask::V1(s) = Skips::schemask();
    let Maskoid::Record(r) = &s.bindings["Skips"] else {
        panic!()
    };
    let keys: Vec<&str> = r.fields.keys().map(|k| k.as_str()).collect();
    assert_eq!(keys, ["a", "c"]);
    let value = Skips {
        a: 1,
        b: 2,
        c: None,
    };
    assert_eq!(value.b, 2);
    valid(&Skips::schemask(), serde_json::to_value(&value).unwrap());
}

#[derive(Serialize, Deserialize, Maskoidy)]
#[serde(transparent)]
struct Transparent {
    inner: Vec<String>,
}

#[test]
fn transparent_struct_uses_inner_type() {
    let Schemask::V1(s) = Transparent::schemask();
    assert!(matches!(s.bindings["Transparent"], Maskoid::List(_)));
    valid(&Transparent::schemask(), serde_json::to_value(&Transparent { inner: vec!["x".to_string()] }).unwrap());
}

#[derive(Serialize, Deserialize, Maskoidy)]
struct Wrapper<T> {
    inner: T,
}

#[test]
fn generic_gets_trait_bound_automatically() {
    valid(&Wrapper::<i64>::schemask(), json!({
        "inner": 4
    }));
}

#[derive(Serialize, Deserialize, Maskoidy)]
struct TwoInstantiations {
    a: Wrapper<i64>,
    b: Wrapper<String>,
}

#[test]
#[should_panic(expected = "two distinct types are both named 'Wrapper'")]
fn generic_instantiations_with_one_name_panic() {
    TwoInstantiations::schemask();
}

mod other {
    #[derive(serde::Serialize, serde::Deserialize, schemask::Maskoidy)]
    pub struct Player {
        pub y: String,
    }
}

#[derive(Serialize, Deserialize, Maskoidy)]
struct SameNames {
    a: Player,
    b: other::Player,
}

#[test]
#[should_panic(expected = "two distinct types are both named 'Player'")]
fn same_name_in_two_modules_panics() {
    SameNames::schemask();
}

#[derive(Serialize, Deserialize, Maskoidy)]
struct MoreImpls {
    set: std::collections::BTreeSet<String>,
    size: usize,
    pair: (i64, String),
    arr: [bool; 2],
    shared: Box<String>,
}

#[test]
fn additional_std_impls() {
    let Schemask::V1(s) = MoreImpls::schemask();
    let Maskoid::Record(r) = &s.bindings["MoreImpls"] else {
        panic!()
    };
    assert!(matches!(r.fields["set"].maskoid, Maskoid::Set(_)));
    assert!(matches!(r.fields["size"].maskoid, Maskoid::Int(_)));
    assert!(matches!(r.fields["pair"].maskoid, Maskoid::Tuple(_)));
    assert!(matches!(r.fields["arr"].maskoid, Maskoid::List(_)));
    assert!(matches!(r.fields["shared"].maskoid, Maskoid::String(_)));
    valid(&MoreImpls::schemask(), serde_json::to_value(&MoreImpls {
        set: ["a".to_string()].into_iter().collect(),
        size: 1,
        pair: (1, "x".to_string()),
        arr: [true, false],
        shared: Box::new("s".to_string()),
    }).unwrap());
}

#[test]
fn check_schema_reports_dangling_refs() {
    let Schemask::V1(mut s) = Player::schemask();
    s.bindings.insert("Broken".to_string(), Maskoid::list(Maskoid::ref_("Nope")));
    s.default = Some("Missing".to_string());
    let err = schemask::check_schema(&s.to_versioned()).expect_err("expected failure");
    let text = err.to_string();
    assert!(text.contains("Nope"), "{}", text);
    assert!(text.contains("Missing"), "{}", text);
    assert!(schemask::check_schema(&Player::schemask()).is_ok());
}
