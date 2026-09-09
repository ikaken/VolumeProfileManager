use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use vpm_core::{
    VolumeProfile, match_profile, ordinal_ignore_case_contains, ordinal_ignore_case_eq,
};

fn probe(mode: &str, input: &str) -> String {
    static DLL: OnceLock<std::path::PathBuf> = OnceLock::new();
    let dll = DLL.get_or_init(|| {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/VolumeProfileManager.Compatibility");
        let result = Command::new("dotnet")
            .arg("build")
            .arg(folder.join("VolumeProfileManager.Compatibility.csproj"))
            .args(["--nologo", "--verbosity", "quiet"])
            .output()
            .expect(".NET 10 SDK is required for V1 compatibility tests");
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        folder.join("bin/Debug/net10.0/VolumeProfileManager.Compatibility.dll")
    });
    let mut child = Command::new("dotnet")
        .arg(dll)
        .arg(mode)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap()
}

#[test]
fn v1_rust_v1_roundtrip_preserves_all_six_fields_and_float_bits() {
    let json = probe("fixtures", "");
    let profiles: Vec<VolumeProfile> = serde_json::from_str(&json).unwrap();
    let output = serde_json::to_string(&profiles).unwrap();
    assert_eq!(probe("verify", &output), "ok");
    assert_eq!(
        profiles,
        serde_json::from_str::<Vec<VolumeProfile>>(&output).unwrap()
    );
}

#[test]
fn ordinal_case_matches_dotnet_for_every_unicode_scalar_mapping() {
    let mappings: Vec<[u32; 2]> = serde_json::from_str(&probe("mappings", "")).unwrap();
    for [lower, upper] in mappings {
        let left = char::from_u32(lower).unwrap().to_string();
        let right = char::from_u32(upper).unwrap().to_string();
        assert!(
            ordinal_ignore_case_eq(&left, &right),
            "U+{lower:04X} U+{upper:04X}"
        );
    }
    let mut pairs = Vec::new();
    for value in 0..=0x10ffff {
        if let Some(ch) = char::from_u32(value) {
            let uppercase = ch.to_uppercase().to_string();
            let original = ch.to_string();
            if uppercase != original {
                pairs.push([original, uppercase]);
            }
        }
    }
    assert_comparisons(&pairs);
}

fn assert_comparisons(pairs: &[[String; 2]]) {
    let expected: Vec<[bool; 2]> =
        serde_json::from_str(&probe("compare", &serde_json::to_string(pairs).unwrap())).unwrap();
    let mut differences = Vec::new();
    for (pair, expected) in pairs.iter().zip(expected) {
        let actual = [
            ordinal_ignore_case_eq(&pair[0], &pair[1]),
            ordinal_ignore_case_contains(&pair[0], &pair[1]),
        ];
        if actual != expected {
            differences.push(format!(
                "{pair:?} {:?}: Rust {actual:?}, .NET {expected:?}",
                pair[0]
                    .chars()
                    .map(|c| format!("U+{:04X}", c as u32))
                    .collect::<Vec<_>>()
            ));
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}

#[test]
fn ordinal_tricky_unicode_and_contains_match_dotnet() {
    let cases = [
        ("i", "İ"),
        ("i", "ı"),
        ("S", "ſ"),
        ("ß", "SS"),
        ("ß", "ẞ"),
        ("Σ", "ς"),
        ("Σ", "σ"),
        ("K", "k"),
        ("é", "e\u{301}"),
        ("𐐀", "𐐨"),
        ("合成 Ａ", "合成 ａ"),
        ("prefixΣtail", "ς"),
        ("", ""),
        ("x", ""),
        ("", "x"),
        ("🦀𐐀z", "𐐨"),
        ("ﬃ", "FFI"),
    ];
    assert_comparisons(&cases.map(|(a, b)| [a.to_owned(), b.to_owned()]));
}

#[test]
fn matcher_results_match_actual_v1_source() {
    let profiles = vec![
        VolumeProfile {
            device_id: "ID".into(),
            device_name: "  合成  Speaker  ".into(),
            ..Default::default()
        },
        VolumeProfile {
            device_id: "id".into(),
            device_name: "合成 Speaker".into(),
            last_applied: "2025-01-01T00:00:00Z".parse().unwrap(),
            ..Default::default()
        },
        VolumeProfile {
            device_id: "Σ".into(),
            device_name: "long 合成 Speaker tail".into(),
            ..Default::default()
        },
        VolumeProfile {
            device_id: "".into(),
            device_name: "\u{85}\u{a0}".into(),
            ..Default::default()
        },
        VolumeProfile {
            device_id: "x".into(),
            device_name: "a\tb".into(),
            ..Default::default()
        },
    ];
    for (id, name) in [
        ("id", Some("other")),
        ("new", Some("合成   Speaker")),
        ("new", Some("Speaker")),
        ("ς", None),
        ("", Some("\u{85}")),
        ("", Some("a b")),
        ("", Some("a\tb")),
        (" ", None),
        ("unknown", Some("none")),
    ] {
        let input = serde_json::json!({"profiles": profiles, "id": id, "name": name});
        let expected: isize = probe("match", &input.to_string()).parse().unwrap();
        let actual = match_profile(&profiles, id, name)
            .map(|p| {
                profiles
                    .iter()
                    .position(|candidate| std::ptr::eq(candidate, p))
                    .unwrap() as isize
            })
            .unwrap_or(-1);
        assert_eq!(actual, expected, "{id:?} {name:?}");
    }
}
