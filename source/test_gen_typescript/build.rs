use {
    schemask::{
        Maskoid,
        MaskoidField,
        generate_typescript,
    },
    std::{
        collections::HashMap,
        fs,
        path::Path,
    },
};

fn main() {
    let schema = (|| -> schemask::latest::SchemaskV1 {
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
    })();
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out = Path::new(&out_dir);

    // Write the generated type definitions.
    fs::write(out.join("types.ts"), generate_typescript(&schema.to_versioned())).unwrap();

    // ── Valid cases ────────────────────────────────────────────────────────────
    fs::write(out.join("valid_primitives.ts"), "\
const valid_label: Label = \"hello\";
const valid_active: Active = true;
const valid_count: Count = 42;
const valid_ratio: Ratio = 3.14;
").unwrap();
    fs::write(out.join("valid_player_no_score.ts"), "\
const valid_player_no_score: Player = {
  name: \"Alice\",
  tags: [\"chess\"],
};
").unwrap();
    fs::write(out.join("valid_player_with_score.ts"), "\
const valid_player_with_score: Player = {
  name: \"Alice\",
  score: 99,
  tags: [\"chess\", \"go\"],
};
").unwrap();
    fs::write(out.join("valid_player_score_null.ts"), "\
const valid_player_score_null: Player = {
  name: \"Alice\",
  score: null,
  tags: [],
};
").unwrap();
    fs::write(out.join("valid_event_join.ts"), "\
const valid_event_join: Event = {
  Join: { name: \"Bob\", score: 10, tags: [] },
};
").unwrap();
    fs::write(out.join("valid_event_leave.ts"), "\
const valid_event_leave: Event = { Leave: \"Bob\" };
").unwrap();
    fs::write(out.join("valid_tags.ts"), "\
const valid_tags: Tags = [\"alpha\", \"beta\", \"gamma\"];
").unwrap();
    fs::write(out.join("valid_coords.ts"), "\
const valid_coords: Coords = [1.0, 2.5];
").unwrap();
    fs::write(out.join("valid_meta.ts"), "\
const valid_meta: Meta = { env: \"prod\", region: \"us-east-1\" };
").unwrap();
    fs::write(out.join("valid_name_ref.ts"), "\
const valid_name_ref: Name = \"Alice\";
").unwrap();

    // ── Invalid cases ──────────────────────────────────────────────────────────
    // name must be string, not number
    fs::write(out.join("invalid_player_name_type.ts"), "\
const invalid_player_name_type: Player = {
  name: 42,
  tags: [],
};
").unwrap();

    // tags is required; omitting it is an error
    fs::write(out.join("invalid_player_missing_tags.ts"), "\
const invalid_player_missing_tags: Player = {
  name: \"Alice\",
};
").unwrap();

    // score must be number | null, not a string
    fs::write(out.join("invalid_player_score_type.ts"), "\
const invalid_player_score_type: Player = {
  name: \"Alice\",
  score: \"high\",
  tags: [],
};
").unwrap();

    // Tags elements must be strings, not numbers
    fs::write(out.join("invalid_tags_element_type.ts"), "\
const invalid_tags_element_type: Tags = [1, 2, 3];
").unwrap();

    // Coords must have exactly two elements
    fs::write(out.join("invalid_coords_length.ts"), "\
const invalid_coords_length: Coords = [1.0];
").unwrap();

    // Event variant key must be Join or Leave
    fs::write(out.join("invalid_event_unknown_variant.ts"), "\
const invalid_event_unknown_variant: Event = { Unknown: \"data\" };
").unwrap();

    // Event Join payload must be a Player, not a plain string
    fs::write(out.join("invalid_event_join_payload.ts"), "\
const invalid_event_join_payload: Event = { Join: \"not-a-player\" };
").unwrap();

    // Meta values must be strings, not numbers
    fs::write(out.join("invalid_meta_value_type.ts"), "\
const invalid_meta_value_type: Meta = { count: 42 };
").unwrap();
}
