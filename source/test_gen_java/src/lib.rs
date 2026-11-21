#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::Path,
        process::Command,
        sync::{
            OnceLock,
            atomic::{
                AtomicUsize,
                Ordering,
            },
        },
    };

    fn classpath() -> Option<&'static String> {
        static CLASSPATH: OnceLock<Option<String>> = OnceLock::new();
        return CLASSPATH.get_or_init(|| {
            if !Command::new("mvn").arg("-v").output().map(|o| o.status.success()).unwrap_or(false) {
                eprintln!("mvn not found, skipping");
                return None;
            }
            let project = Path::new(env!("OUT_DIR")).join("maven");
            let cp_file = project.join("target/classpath.txt");
            let status =
                Command::new("mvn")
                    .args(["-q", "-B", "compile", "dependency:build-classpath"])
                    .arg(format!("-Dmdep.outputFile={}", cp_file.display()))
                    .current_dir(&project)
                    .status()
                    .unwrap();
            assert!(status.success(), "generated java failed to compile (see maven output above)");
            let sep = if cfg!(windows) {
                ";"
            } else {
                ":"
            };
            return Some(
                format!("{}{}{}", project.join("target/classes").display(), sep, fs::read_to_string(cp_file).unwrap()),
            );
        }).as_ref();
    }

    #[test]
    fn invalid_rejected() {
        let Some(classpath) = classpath() else {
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
            ("Maybe", r#"{"value":7,"x":1}"#),
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
            ("Foo", r#"{"bar_baz":{"y":2}}"#),
            ("FooBar", r#"{"baz":{"x":1}}"#),
        ] {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(input) {
                assert!(
                    schemask::validate(&schema, Some(binding.to_string()), &value).is_err(),
                    "fixture for {} unexpectedly valid against the schema: {}",
                    binding,
                    input
                );
            }
            assert!(round_trip(classpath, binding, input).is_err(), "{} {} should have been rejected", binding, input);
        }
    }

    fn round_trip(classpath: &str, binding: &str, json: &str) -> Result<serde_json::Value, String> {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let file =
            Path::new(
                env!("OUT_DIR"),
            ).join(format!("fixture_{}_{}.json", binding, COUNTER.fetch_add(1, Ordering::Relaxed)));
        fs::write(&file, json).unwrap();
        let output =
            Command::new("java")
                .arg("-cp")
                .arg(classpath)
                .arg("Main")
                .arg(binding)
                .arg(&file)
                .output()
                .unwrap();
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
        let Some(classpath) = classpath() else {
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
            ("Maybe", r#"{"value":null}"#, r#"{"value":null}"#),
            ("Maybe", r#"{"value":7}"#, r#"{"value":7}"#),
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
            ("Foo", r#"{"bar_baz":{"x":1}}"#, r#"{"bar_baz":{"x":1}}"#),
            ("FooBar", r#"{"baz":{"y":2}}"#, r#"{"baz":{"y":2}}"#),
        ] {
            schemask::validate(
                &schema,
                Some(binding.to_string()),
                &serde_json::from_str(input).unwrap(),
            ).unwrap_or_else(|e| panic!("fixture for {} is not valid against the schema: {}", binding, e));
            let got =
                round_trip(classpath, binding, input).unwrap_or_else(|e| panic!("{} {}: {}", binding, input, e));
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
            let got = round_trip(classpath, "Event", input).unwrap_or_else(|e| panic!("Event {}: {}", input, e));
            assert_eq!(got, serde_json::from_str::<serde_json::Value>(expected).unwrap(), "Event {}", input);
        }
    }
}
