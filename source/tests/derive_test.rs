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

#[allow(dead_code)]
#[derive(Maskoidy)]
struct TreeNode {
    value: i64,
    children: Vec<TreeNode>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn schema_for<T: Maskoidy>() -> Schemask {
    return T::schemask().to_versioned();
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
    let s = Player::schemask();
    let m = &s.bindings["Player"];
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
    let s = Event::schemask();
    let m = &s.bindings["Event"];
    assert!(matches!(m, Maskoid::TaggedUnion(_)));
    let Maskoid::TaggedUnion(u) = m else { panic!() };
    assert!(u.variants.contains_key("Join"));
    assert!(u.variants.contains_key("Leave"));
    assert!(matches!(&u.variants["Join"].maskoid, Maskoid::Ref(name) if name == "Player"));
    assert_eq!(u.variants["Join"].description.as_deref(), Some("A new player joined."));
    assert!(matches!(u.variants["Leave"].maskoid, Maskoid::String));
    assert_eq!(u.variants["Leave"].description, None);
}

#[test]
fn label_maskoid_is_string() {
    // Newtype wrapping String → String maskoid
    assert!(matches!(Label::schemask().bindings["Label"], Maskoid::String));
}

#[test]
fn coords_maskoid_is_tuple() {
    let s = Coords::schemask();
    let m = &s.bindings["Coords"];
    assert!(matches!(m, Maskoid::Tuple(_)));
    let Maskoid::Tuple(t) = m else { panic!() };
    assert_eq!(t.elements.len(), 2);
    assert!(matches!(t.elements[0].maskoid, Maskoid::Float));
    assert!(matches!(t.elements[1].maskoid, Maskoid::Float));
}

#[test]
fn unit_maskoid_is_null() {
    assert!(matches!(Unit::schemask().bindings["Unit"], Maskoid::Null));
}

// ── Validation round-trip tests ───────────────────────────────────────────────

#[test]
fn player_valid_no_score() {
    let s = schema_for::<Player>();
    valid(&s, json!({ "name": "Alice", "tags": [] }));
}

#[test]
fn player_valid_with_score() {
    let s = schema_for::<Player>();
    valid(&s, json!({ "name": "Alice", "score": 42, "tags": ["chess"] }));
}

#[test]
fn player_valid_null_score() {
    let s = schema_for::<Player>();
    valid(&s, json!({ "name": "Alice", "score": null, "tags": [] }));
}

#[test]
fn player_invalid_wrong_name_type() {
    let s = schema_for::<Player>();
    invalid(&s, json!({ "name": 99, "tags": [] }));
}

#[test]
fn player_invalid_missing_tags() {
    let s = schema_for::<Player>();
    invalid(&s, json!({ "name": "Alice" }));
}

#[test]
fn event_valid_join() {
    let s = schema_for::<Event>();
    valid(&s, json!({ "Join": { "name": "Bob", "score": 10, "tags": [] } }));
}

#[test]
fn event_valid_leave() {
    let s = schema_for::<Event>();
    valid(&s, json!({ "Leave": "Bob" }));
}

#[test]
fn event_invalid_unknown_variant() {
    let s = schema_for::<Event>();
    invalid(&s, json!({ "Quit": "Bob" }));
}

#[test]
fn label_valid() {
    let s = schema_for::<Label>();
    valid(&s, json!("hello"));
}

#[test]
fn coords_valid() {
    let s = schema_for::<Coords>();
    valid(&s, json!([1.0, 2.5]));
}

#[test]
fn coords_invalid_wrong_length() {
    let s = schema_for::<Coords>();
    invalid(&s, json!([1.0]));
}

#[test]
fn tree_node_schemask_breaks_cycle() {
    let s = TreeNode::schemask();
    // The schema should contain a "TreeNode" binding
    assert!(s.bindings.contains_key("TreeNode"), "missing TreeNode binding");
    let Maskoid::Record(r) = &s.bindings["TreeNode"] else { panic!("expected Record") };
    assert!(r.fields.contains_key("value"));
    assert!(r.fields.contains_key("children"));
    // children should be a list of Ref("TreeNode") — the cycle is broken
    let Maskoid::List(inner) = &r.fields["children"].maskoid else { panic!("expected List") };
    assert!(matches!(inner.as_ref(), Maskoid::Ref(name) if name == "TreeNode"));
    assert_eq!(s.default, Some("TreeNode".to_string()));
}
