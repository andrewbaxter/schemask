use {
    schemask::{
        Maskoid,
        Schemask,
        Schematize,
        r#match,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    serde_json::json,
    std::collections::HashMap,
};

// ── Types under test ──────────────────────────────────────────────────────────

/// A participant in a game session.
#[derive(Serialize, Deserialize, Schematize)]
struct Player {
    /// Display name shown in the lobby.
    name: String,
    /// Current score; absent until the player has scored.
    score: Option<i64>,
    tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Schematize)]
enum Event {
    /// A new player joined.
    Join(Player),
    Leave(String),
}

#[derive(Serialize, Deserialize, Schematize)]
struct Label(String);

#[derive(Serialize, Deserialize, Schematize)]
struct Coords(f64, f64);

#[derive(Serialize, Deserialize, Schematize)]
struct Unit;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn schema_for<T: Schematize>(name: &str) -> Schemask {
    let mut bindings = HashMap::new();
    bindings.insert(name.to_string(), T::maskoid());
    Schemask { bindings, default: Some(name.to_string()) }
}

fn valid(schema: &Schemask, data: serde_json::Value) {
    r#match(schema, None, &data).expect("expected valid");
}

fn invalid(schema: &Schemask, data: serde_json::Value) {
    r#match(schema, None, &data).expect_err("expected invalid");
}

// ── Maskoid shape tests ───────────────────────────────────────────────────────

#[test]
fn player_maskoid_is_record() {
    let m = Player::maskoid();
    assert!(matches!(m, Maskoid::Record(_)));
    let Maskoid::Record(r) = m else { panic!() };
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
    let m = Event::maskoid();
    assert!(matches!(m, Maskoid::TaggedUnion(_)));
    let Maskoid::TaggedUnion(u) = m else { panic!() };
    assert!(u.variants.contains_key("Join"));
    assert!(u.variants.contains_key("Leave"));
    assert!(matches!(u.variants["Join"].maskoid, Maskoid::Record(_)));
    assert_eq!(u.variants["Join"].description.as_deref(), Some("A new player joined."));
    assert!(matches!(u.variants["Leave"].maskoid, Maskoid::String));
    assert_eq!(u.variants["Leave"].description, None);
}

#[test]
fn label_maskoid_is_string() {
    // Newtype wrapping String → String maskoid
    assert!(matches!(Label::maskoid(), Maskoid::String));
}

#[test]
fn coords_maskoid_is_tuple() {
    assert!(matches!(Coords::maskoid(), Maskoid::Tuple(_)));
    let Maskoid::Tuple(t) = Coords::maskoid() else { panic!() };
    assert_eq!(t.elements.len(), 2);
    assert!(matches!(t.elements[0].maskoid, Maskoid::Float));
    assert!(matches!(t.elements[1].maskoid, Maskoid::Float));
}

#[test]
fn unit_maskoid_is_null() {
    assert!(matches!(Unit::maskoid(), Maskoid::Null));
}

// ── Validation round-trip tests ───────────────────────────────────────────────

#[test]
fn player_valid_no_score() {
    let s = schema_for::<Player>("Player");
    valid(&s, json!({ "name": "Alice", "tags": [] }));
}

#[test]
fn player_valid_with_score() {
    let s = schema_for::<Player>("Player");
    valid(&s, json!({ "name": "Alice", "score": 42, "tags": ["chess"] }));
}

#[test]
fn player_valid_null_score() {
    let s = schema_for::<Player>("Player");
    valid(&s, json!({ "name": "Alice", "score": null, "tags": [] }));
}

#[test]
fn player_invalid_wrong_name_type() {
    let s = schema_for::<Player>("Player");
    invalid(&s, json!({ "name": 99, "tags": [] }));
}

#[test]
fn player_invalid_missing_tags() {
    let s = schema_for::<Player>("Player");
    invalid(&s, json!({ "name": "Alice" }));
}

#[test]
fn event_valid_join() {
    let s = schema_for::<Event>("Event");
    valid(&s, json!({ "Join": { "name": "Bob", "score": 10, "tags": [] } }));
}

#[test]
fn event_valid_leave() {
    let s = schema_for::<Event>("Event");
    valid(&s, json!({ "Leave": "Bob" }));
}

#[test]
fn event_invalid_unknown_variant() {
    let s = schema_for::<Event>("Event");
    invalid(&s, json!({ "Quit": "Bob" }));
}

#[test]
fn label_valid() {
    let s = schema_for::<Label>("Label");
    valid(&s, json!("hello"));
}

#[test]
fn coords_valid() {
    let s = schema_for::<Coords>("Coords");
    valid(&s, json!([1.0, 2.5]));
}

#[test]
fn coords_invalid_wrong_length() {
    let s = schema_for::<Coords>("Coords");
    invalid(&s, json!([1.0]));
}
