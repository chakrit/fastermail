//! Typed accessors for reading values out of untyped `serde_json::Value` trees.
//!
//! These collapse the repetitive `v.get("k").and_then(|x| x.as_str())` shape into a
//! single call against a JSON Pointer path (`"/k"`, or `"/a/b/0"` for nested/indexed
//! access). The leading `/` is required; see RFC 6901.

use serde_json::Value;

/// Borrow the string at `ptr`, or `None` if absent or not a string.
pub fn str_at<'a>(v: &'a Value, ptr: &str) -> Option<&'a str> {
    v.pointer(ptr).and_then(Value::as_str)
}

/// Borrow the array at `ptr`, or `None` if absent or not an array.
pub fn array_at<'a>(v: &'a Value, ptr: &str) -> Option<&'a Vec<Value>> {
    v.pointer(ptr).and_then(Value::as_array)
}

/// Read the bool at `ptr`, or `None` if absent or not a bool.
pub fn bool_at(v: &Value, ptr: &str) -> Option<bool> {
    v.pointer(ptr).and_then(Value::as_bool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_nested_and_indexed_paths() {
        let v = json!({
            "name": "Inbox",
            "counts": { "unread": 3 },
            "ids": ["a", "b"],
            "active": true
        });

        assert_eq!(str_at(&v, "/name"), Some("Inbox"));
        assert_eq!(str_at(&v, "/ids/0"), Some("a"));
        assert_eq!(bool_at(&v, "/active"), Some(true));
        assert_eq!(array_at(&v, "/ids").map(Vec::len), Some(2));
    }

    #[test]
    fn missing_or_mistyped_paths_yield_none() {
        let v = json!({ "name": "Inbox" });

        assert_eq!(str_at(&v, "/missing"), None);
        assert_eq!(bool_at(&v, "/name"), None);
    }
}
