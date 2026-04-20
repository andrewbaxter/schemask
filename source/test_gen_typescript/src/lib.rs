use {
    std::{
        path::Path,
        process::Command,
    },
};

fn tsc(file: &str) -> bool {
    let out = Path::new(env!("OUT_DIR"));
    Command::new("tsc")
        .args(["--noEmit", "--strict", "--target", "ES2020", "--lib", "ES2020"])
        .arg(out.join("types.ts"))
        .arg(out.join(file))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn tsc_available() -> bool {
    Command::new("tsc").arg("--version").status().map(|s| s.success()).unwrap_or(false)
}

macro_rules! valid{
    ($name: ident, $file: literal) => {
        #[test] fn $name() {
            if !tsc_available() {
                eprintln!("tsc not found, skipping");
                return;
            }
            assert!(tsc($file), "expected {} to type-check successfully", $file);
        }
    };
}

macro_rules! invalid{
    ($name: ident, $file: literal) => {
        #[test] fn $name() {
            if !tsc_available() {
                eprintln!("tsc not found, skipping");
                return;
            }
            assert!(! tsc($file), "expected {} to fail type-checking", $file);
        }
    };
}

valid!(test_valid_primitives, "valid_primitives.ts");

valid!(test_valid_player_no_score, "valid_player_no_score.ts");

valid!(test_valid_player_with_score, "valid_player_with_score.ts");

valid!(test_valid_player_score_null, "valid_player_score_null.ts");

valid!(test_valid_event_join, "valid_event_join.ts");

valid!(test_valid_event_leave, "valid_event_leave.ts");

valid!(test_valid_tags, "valid_tags.ts");

valid!(test_valid_coords, "valid_coords.ts");

valid!(test_valid_meta, "valid_meta.ts");

valid!(test_valid_name_ref, "valid_name_ref.ts");

invalid!(test_invalid_player_name_type, "invalid_player_name_type.ts");

invalid!(test_invalid_player_missing_tags, "invalid_player_missing_tags.ts");

invalid!(test_invalid_player_score_type, "invalid_player_score_type.ts");

invalid!(test_invalid_tags_element_type, "invalid_tags_element_type.ts");

invalid!(test_invalid_coords_length, "invalid_coords_length.ts");

invalid!(test_invalid_event_unknown_variant, "invalid_event_unknown_variant.ts");

invalid!(test_invalid_event_join_payload, "invalid_event_join_payload.ts");

invalid!(test_invalid_meta_value_type, "invalid_meta_value_type.ts");
