#[cfg(test)]
mod tests {
    use {
        std::{
            fs,
            path::{
                Path,
                PathBuf,
            },
            process::Command,
            sync::{
                OnceLock,
                atomic::{
                    AtomicUsize,
                    Ordering,
                },
            },
        },
    };

    fn classes() -> Option<&'static PathBuf> {
        static CLASSES: OnceLock<Option<PathBuf>> = OnceLock::new();
        return CLASSES.get_or_init(|| {
            if !Command::new("javac").arg("-version").status().map(|s| s.success()).unwrap_or(false) {
                eprintln!("javac not found, skipping");
                return None;
            }
            let out = Path::new(env!("OUT_DIR"));
            let classes = out.join("classes");
            let _ = fs::remove_dir_all(&classes);
            fs::create_dir_all(&classes).unwrap();
            let mut sources = vec![];
            for root in [
                Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime"),
                Path::new(env!("CARGO_MANIFEST_DIR")).join("java"),
                out.join("java"),
            ] {
                let mut stack = vec![root];
                while let Some(dir) = stack.pop() {
                    for entry in fs::read_dir(dir).unwrap() {
                        let path = entry.unwrap().path();
                        if path.is_dir() {
                            stack.push(path);
                        } else if path.extension().map(|e| e == "java").unwrap_or(false) {
                            sources.push(path);
                        }
                    }
                }
            }
            let status =
                Command::new("javac")
                    .args(["-Xlint:all,-serial", "-Werror", "-d"])
                    .arg(&classes)
                    .args(&sources)
                    .status()
                    .unwrap();
            assert!(status.success(), "generated java failed to compile");
            return Some(classes);
        }).as_ref();
    }

    fn round_trip(classes: &Path, binding: &str, json: &str) -> Result<serde_json::Value, String> {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let file =
            Path::new(
                env!("OUT_DIR"),
            ).join(format!("fixture_{}_{}.json", binding, COUNTER.fetch_add(1, Ordering::Relaxed)));
        fs::write(&file, json).unwrap();
        let output =
            Command::new("java").arg("-cp").arg(classes).arg("Main").arg(binding).arg(&file).output().unwrap();
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        return Ok(serde_json::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap());
    }

    fn schema() -> schemask::Schemask {
        return serde_json::from_str(
            &fs::read_to_string(Path::new(env!("OUT_DIR")).join("schema.json")).unwrap(),
        ).unwrap();
    }

    #[test]
    fn valid_round_trips() {
        let Some(classes) = classes() else {
            return;
        };
        let schema = schema();
        let account =
            r#"{"active":true,"extra":{"k":[1,2.5,null]},"home":{"city":"Oslo"},"name":"Alice","score":9,"tags":["a","b"],"type":"user"}"#;
        let join = format!(r#"{{"Join":{}}}"#, account);
        for (
            binding,
            input,
            expected,
        ) in [
            ("Label", r#""hello""#, r#""hello""#),
            ("Tags", r#"["a","b"]"#, r#"["a","b"]"#),
            ("Unique", r#"[3,1,2]"#, r#"[3,1,2]"#),
            ("Meta", r#"{"env":"prod","region":"eu"}"#, r#"{"env":"prod","region":"eu"}"#),
            ("Maybe", "null", "null"),
            ("Maybe", r#"{"element":null}"#, r#"{"element":null}"#),
            ("Maybe", r#"{"element":7}"#, r#"{"element":7}"#),
            ("Coords", "[1.5,2]", "[1.5,2.0]"),
            ("Account", account, account),
            (
                "Account",
                r#"{"active":false,"avatar-url":null,"extra":null,"home":{"city":"Oslo"},"name":"Bob","tags":[],"type":"user"}"#,
                r#"{"active":false,"extra":null,"home":{"city":"Oslo"},"name":"Bob","tags":[],"type":"user"}"#,
            ),
            ("Event", join.as_str(), join.as_str()),
            ("Shape", r#"{"circle":{"radius":2}}"#, r#"{"circle":{"radius":2.0}}"#),
            ("Shape", r#"{"named":"box"}"#, r#"{"named":"box"}"#),
        ] {
            schemask::validate(
                &schema,
                Some(binding.to_string()),
                &serde_json::from_str(input).unwrap(),
            ).unwrap_or_else(|e| panic!("fixture for {} is not valid against the schema: {}", binding, e));
            let got =
                round_trip(classes, binding, input).unwrap_or_else(|e| panic!("{} {}: {}", binding, input, e));
            assert_eq!(got, serde_json::from_str::<serde_json::Value>(expected).unwrap(), "{} {}", binding, input);
        }
        for (
            input,
            expected,
        ) in [
            (r#""Ping""#, r#""Ping""#),
            (r#"{"Ping":null}"#, r#""Ping""#),
            (r#"{"user_left":"Bob"}"#, r#"{"user_left":"Bob"}"#),
            (r#"{"Moved":[0,1]}"#, r#"{"Moved":[0.0,1.0]}"#),
        ] {
            schemask::validate(&schema, Some("Event".to_string()), &serde_json::from_str(input).unwrap()).unwrap();
            let got = round_trip(classes, "Event", input).unwrap_or_else(|e| panic!("Event {}: {}", input, e));
            assert_eq!(got, serde_json::from_str::<serde_json::Value>(expected).unwrap(), "Event {}", input);
        }
    }

    #[test]
    fn invalid_rejected() {
        let Some(classes) = classes() else {
            return;
        };
        let schema = schema();
        for (
            binding,
            input,
        ) in [
            ("Label", "42"),
            ("Tags", r#"["a",1]"#),
            ("Unique", "[1,1]"),
            ("Meta", r#"{"k":1}"#),
            ("Maybe", "7"),
            ("Maybe", r#"{"element":7,"x":1}"#),
            ("Coords", "[1.0]"),
            (
                "Account",
                r#"{"active":true,"extra":null,"home":{"city":"Oslo"},"name":"Alice","tags":[],"type":"admin"}"#,
            ),
            ("Account", r#"{"active":true,"extra":null,"home":{"city":"Oslo"},"tags":[],"type":"user"}"#),
            (
                "Account",
                r#"{"active":true,"extra":null,"home":{"city":"Oslo"},"name":"Alice","tags":[],"type":"user","nope":1}"#,
            ),
            (
                "Account",
                r#"{"active":true,"extra":null,"home":{"city":"Oslo"},"name":"Alice","score":"high","tags":[],"type":"user"}"#,
            ),
            ("Event", r#""Unknown""#),
            ("Event", r#"{"Ping":1}"#),
            ("Event", r#"{"Join":"x"}"#),
            ("Event", r#"{"Join":{},"Ping":null}"#),
            ("Event", r#""Ping" 1"#),
            ("Shape", r#""circle""#),
            ("Shape", r#"{"circle":{"radius":2,"x":1}}"#),
        ] {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(input) {
                assert!(
                    schemask::validate(&schema, Some(binding.to_string()), &value).is_err(),
                    "fixture for {} unexpectedly valid against the schema: {}",
                    binding,
                    input
                );
            }
            assert!(round_trip(classes, binding, input).is_err(), "{} {} should have been rejected", binding, input);
        }
    }
}
