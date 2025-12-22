use greentic_integration::fixtures::{Fixture, normalize_json};
use serde_json::json;

#[test]
fn normalize_strips_unstable_fields_and_uuid_like_values() {
    let raw = Fixture::load_json("expected/normalize_input.json").expect("load fixture");
    let normalized = normalize_json(raw);

    let expected = json!({
        "payload": {
            "nested": {
                "keep": "ok"
            },
            "list": [
                { "value": 1 }
            ]
        }
    });

    assert_eq!(normalized, expected);
}
