//! Behavioral tests driving the public library API.

use configdiff::{
    ArrayStrategy, ChangeKind, DiffOptions, Format, Value, diff, diff_str, parse, render,
};

fn json(s: &str) -> Value {
    parse(s, Format::Json).expect("valid json")
}

fn diff_json(old: &str, new: &str) -> configdiff::Diff {
    diff(&json(old), &json(new), &DiffOptions::default())
}

#[test]
fn identical_documents_have_no_changes() {
    let d = diff_json(r#"{"a":1,"b":[1,2,3]}"#, r#"{"a":1,"b":[1,2,3]}"#);
    assert!(d.is_empty());
    assert_eq!(d.len(), 0);
}

#[test]
fn key_order_is_ignored() {
    // Same content, different key order: semantically equal.
    let d = diff_json(r#"{"a":1,"b":2}"#, r#"{"b":2,"a":1}"#);
    assert!(d.is_empty());
}

#[test]
fn scalar_value_change_is_detected() {
    let d = diff_json(r#"{"port":8080}"#, r#"{"port":9090}"#);
    assert_eq!(d.len(), 1);
    let c = &d.changes()[0];
    assert_eq!(c.path.to_string(), "port");
    assert!(matches!(c.kind, ChangeKind::Changed { .. }));
}

#[test]
fn added_and_removed_keys() {
    let d = diff_json(r#"{"a":1}"#, r#"{"b":2}"#);
    let s = d.summary();
    assert_eq!(s.added, 1);
    assert_eq!(s.removed, 1);
    assert_eq!(s.changed, 0);
}

#[test]
fn type_change_is_distinct_from_value_change() {
    // 8080 (integer) vs "8080" (string): a type change, not a value change.
    let d = diff_json(r#"{"port":8080}"#, r#"{"port":"8080"}"#);
    assert_eq!(d.len(), 1);
    assert!(matches!(
        d.changes()[0].kind,
        ChangeKind::TypeChanged { .. }
    ));
    assert_eq!(d.summary().type_changed, 1);
}

#[test]
fn integer_vs_float_is_a_type_change_by_default() {
    let d = diff_json(r#"{"x":1}"#, r#"{"x":1.0}"#);
    assert_eq!(d.summary().type_changed, 1);
}

#[test]
fn loose_numbers_treats_int_and_float_as_equal() {
    let opts = DiffOptions::default().numbers_loose(true);
    let d = diff(&json(r#"{"x":1}"#), &json(r#"{"x":1.0}"#), &opts);
    assert!(d.is_empty());
}

#[test]
fn loose_numbers_still_reports_real_value_changes() {
    let opts = DiffOptions::default().numbers_loose(true);
    let d = diff(&json(r#"{"x":1}"#), &json(r#"{"x":2.0}"#), &opts);
    assert_eq!(d.len(), 1);
    assert!(matches!(d.changes()[0].kind, ChangeKind::Changed { .. }));
}

#[test]
fn float_tolerance_absorbs_tiny_differences() {
    let opts = DiffOptions::default().float_tolerance(Some(0.01));
    let d = diff(&json(r#"{"x":1.001}"#), &json(r#"{"x":1.002}"#), &opts);
    assert!(d.is_empty());

    let d = diff(&json(r#"{"x":1.0}"#), &json(r#"{"x":1.5}"#), &opts);
    assert_eq!(d.len(), 1);
}

#[test]
fn nested_paths_are_rendered_dotted_and_bracketed() {
    // Positional strategy so an in-place edit shows as a single change, letting
    // us assert the dotted + bracketed + dotted path rendering.
    let opts = DiffOptions::default().array_strategy(ArrayStrategy::Positional);
    let d = diff(
        &json(r#"{"server":{"hosts":[{"name":"a"},{"name":"b"}]}}"#),
        &json(r#"{"server":{"hosts":[{"name":"a"},{"name":"c"}]}}"#),
        &opts,
    );
    assert_eq!(d.len(), 1);
    assert_eq!(d.changes()[0].path.to_string(), "server.hosts[1].name");
}

#[test]
fn array_lcs_reports_scalar_edit_as_remove_plus_add() {
    // LCS cannot know whether an element was edited or deleted-and-inserted, so
    // a changed scalar in a list is reported as a removal plus an addition.
    let d = diff_json(r#"{"a":["x","y"]}"#, r#"{"a":["x","z"]}"#);
    let s = d.summary();
    assert_eq!(s.removed, 1);
    assert_eq!(s.added, 1);
    assert_eq!(s.changed, 0);
}

#[test]
fn array_lcs_detects_insertion_without_cascading() {
    // Insert 99 at the front; only one addition should be reported, not three
    // "changed" elements shifted by one.
    let d = diff_json(r#"{"a":[1,2,3]}"#, r#"{"a":[99,1,2,3]}"#);
    assert_eq!(d.len(), 1);
    assert!(matches!(d.changes()[0].kind, ChangeKind::Added { .. }));
}

#[test]
fn array_positional_compares_by_index() {
    let opts = DiffOptions::default().array_strategy(ArrayStrategy::Positional);
    // Positionally, inserting at the front changes every element and appends one.
    let d = diff(
        &json(r#"{"a":[1,2,3]}"#),
        &json(r#"{"a":[99,1,2,3]}"#),
        &opts,
    );
    assert_eq!(d.summary().changed, 3);
    assert_eq!(d.summary().added, 1);
}

#[test]
fn array_keyed_matches_reordered_objects_by_key() {
    let opts = DiffOptions::default()
        .array_strategy(ArrayStrategy::Keyed)
        .array_keys(vec!["id".to_owned()]);
    let old = json(r#"{"u":[{"id":"a","role":"admin"},{"id":"b","role":"view"}]}"#);
    // Reordered, and b's role changed.
    let new = json(r#"{"u":[{"id":"b","role":"edit"},{"id":"a","role":"admin"}]}"#);
    let d = diff(&old, &new, &opts);
    assert_eq!(d.len(), 1);
    assert_eq!(d.changes()[0].path.to_string(), "u[1].role");
    assert!(matches!(d.changes()[0].kind, ChangeKind::Changed { .. }));
}

#[test]
fn array_keyed_reports_added_and_removed_elements() {
    let opts = DiffOptions::default()
        .array_strategy(ArrayStrategy::Keyed)
        .array_keys(vec!["id".to_owned()]);
    let old = json(r#"{"u":[{"id":"a"},{"id":"b"}]}"#);
    let new = json(r#"{"u":[{"id":"a"},{"id":"c"}]}"#);
    let d = diff(&old, &new, &opts);
    let s = d.summary();
    assert_eq!(s.removed, 1); // b gone
    assert_eq!(s.added, 1); // c new
}

#[test]
fn ignore_globs_skip_matching_paths() {
    let opts = DiffOptions::default()
        .ignore(["**/updated_at", "meta/*"])
        .unwrap();
    let old = json(r#"{"meta":{"a":1,"b":2},"updated_at":"t1","port":80}"#);
    let new = json(r#"{"meta":{"a":9,"b":9},"updated_at":"t2","port":81}"#);
    let d = diff(&old, &new, &opts);
    // Only `port` survives; meta/* and updated_at are ignored.
    assert_eq!(d.len(), 1);
    assert_eq!(d.changes()[0].path.to_string(), "port");
}

#[test]
fn cross_format_equality_toml_yaml_json() {
    let d = diff_str(
        "port = 8080\ntitle = \"svc\"\n",
        "{\"title\":\"svc\",\"port\":8080}",
        Some(Format::Toml),
        Some(Format::Json),
        &DiffOptions::default(),
    )
    .unwrap();
    assert!(d.is_empty(), "same data in TOML and JSON should be equal");
}

#[test]
fn toml_datetime_becomes_a_comparable_string() {
    // A TOML datetime should parse into a string and compare equal to the same
    // RFC 3339 string coming from JSON.
    let d = diff_str(
        "ts = 1979-05-27T07:32:00Z\n",
        "{\"ts\":\"1979-05-27T07:32:00Z\"}",
        Some(Format::Toml),
        Some(Format::Json),
        &DiffOptions::default(),
    )
    .unwrap();
    assert!(
        d.is_empty(),
        "toml datetime should equal its RFC 3339 string"
    );
}

#[test]
fn container_to_scalar_is_a_type_change() {
    let d = diff_json(r#"{"a":{"b":1}}"#, r#"{"a":5}"#);
    assert_eq!(d.len(), 1);
    assert!(matches!(
        d.changes()[0].kind,
        ChangeKind::TypeChanged { .. }
    ));
}

#[test]
fn pretty_render_plain_is_stable() {
    let old = parse("a = 1\nb = 2\n[s]\nx = \"old\"\n", Format::Toml).unwrap();
    let new = parse("a = 1\nb = 3\nc = 9\n[s]\nx = \"new\"\n", Format::Toml).unwrap();
    let d = diff(&old, &new, &DiffOptions::default());
    let out = render::pretty::render(&d, false);
    let expected = "~ b: 2 -> 3\n~ s.x: \"old\" -> \"new\"\n+ c: 9\n\n3 changes: +1  ~2\n";
    assert_eq!(out, expected);
}

#[test]
fn pretty_render_reports_no_differences() {
    let d = diff_json(r#"{"a":1}"#, r#"{"a":1}"#);
    assert_eq!(render::pretty::render(&d, false), "no differences\n");
}

#[test]
fn json_render_shape_is_stable() {
    let d = diff_json(r#"{"port":8080}"#, r#"{"port":9090}"#);
    let out = render::json::render(&d, false);
    assert_eq!(
        out,
        r#"{"summary":{"added":0,"removed":0,"changed":1,"type_changed":0,"total":1},"changes":[{"path":"port","kind":"changed","old":8080,"new":9090}]}"#
    );
}

#[test]
fn invalid_input_is_a_parse_error() {
    let err = diff_str(
        "this is : not : valid : json",
        "{}",
        Some(Format::Json),
        Some(Format::Json),
        &DiffOptions::default(),
    );
    assert!(err.is_err());
}

#[test]
fn without_expand_an_added_subtree_is_one_change() {
    let d = diff_json("{}", r#"{"server":{"host":"h","port":80}}"#);
    assert_eq!(d.len(), 1);
    assert_eq!(d.changes()[0].path.to_string(), "server");
}

#[test]
fn expand_reports_each_leaf_of_an_added_subtree() {
    let opts = DiffOptions::default().expand(true);
    let d = diff(
        &json("{}"),
        &json(r#"{"server":{"host":"h","port":80}}"#),
        &opts,
    );
    assert_eq!(d.len(), 2);
    let paths: Vec<String> = d.changes().iter().map(|c| c.path.to_string()).collect();
    assert!(paths.contains(&"server.host".to_owned()));
    assert!(paths.contains(&"server.port".to_owned()));
    assert_eq!(d.summary().added, 2);
}

#[test]
fn expand_reports_each_leaf_of_a_removed_subtree() {
    let opts = DiffOptions::default().expand(true);
    let d = diff(
        &json(r#"{"db":{"name":"prod","port":5432}}"#),
        &json("{}"),
        &opts,
    );
    assert_eq!(d.summary().removed, 2);
}

#[test]
fn ini_sections_become_nested_objects() {
    let a = parse("[server]\nhost = localhost\nport = 8080\n", Format::Ini).unwrap();
    let b = parse("[server]\nhost = 0.0.0.0\nport = 8080\n", Format::Ini).unwrap();
    let d = diff(&a, &b, &DiffOptions::default());
    assert_eq!(d.len(), 1);
    assert_eq!(d.changes()[0].path.to_string(), "server.host");
}

#[test]
fn dotenv_ignores_comments_and_quotes() {
    // Comments, an `export` prefix, and quoting should not affect the parsed
    // values, so these two documents are equal.
    let a = parse(
        "# a comment\nA=1\nexport B=\"two\"\n\nC='three'\n",
        Format::Dotenv,
    )
    .unwrap();
    let b = parse("A=1\nB=two\nC=three\n", Format::Dotenv).unwrap();
    let d = diff(&a, &b, &DiffOptions::default());
    assert!(d.is_empty(), "got: {:?}", d.changes());
}

#[test]
fn dotenv_values_are_strings() {
    // `.env` has no types: everything is a string, so `8080` here is "8080".
    let env = parse("PORT=8080\n", Format::Dotenv).unwrap();
    let json_int = json(r#"{"PORT":8080}"#);
    let d = diff(&env, &json_int, &DiffOptions::default());
    assert_eq!(d.summary().type_changed, 1);
}

fn nested_object(depth: usize) -> Value {
    let mut v = Value::Integer(0);
    for _ in 0..depth {
        let mut m = configdiff::Map::new();
        m.insert("a".to_owned(), v);
        v = Value::Object(m);
    }
    v
}

#[test]
fn deeply_nested_input_is_bounded_not_overflowing() {
    // A structure deeper than the internal recursion limit must not overflow the
    // stack. The diff terminates and reports a single bounded marker change at
    // the depth limit instead of recursing without end.
    let deep = nested_object(300);
    let d = diff(&deep, &deep, &DiffOptions::default());
    assert_eq!(d.len(), 1);
    assert!(matches!(d.changes()[0].kind, ChangeKind::Changed { .. }));
}

#[test]
fn yaml_nan_equals_itself() {
    // A `.nan` float compared with itself is not a change (reflexivity), even
    // though IEEE says NaN != NaN.
    let a = parse("x: .nan\n", Format::Yaml).unwrap();
    let d = diff(&a, &a, &DiffOptions::default());
    assert!(d.is_empty(), "got: {:?}", d.changes());

    // But a NaN versus a real number is still a change.
    let b = parse("x: 1.0\n", Format::Yaml).unwrap();
    let d = diff(&a, &b, &DiffOptions::default());
    assert_eq!(d.len(), 1);
}

#[test]
fn large_arrays_fall_back_from_lcs() {
    // Two arrays whose LCS matrix would exceed the cell budget must not allocate
    // it; the diff falls back to positional comparison and still completes.
    let a: Vec<Value> = (0..3000i64).map(Value::Integer).collect();
    let mut b = a.clone();
    b[0] = Value::Integer(999_999);
    let d = diff(&Value::Array(a), &Value::Array(b), &DiffOptions::default());
    // Positional fallback reports the single edited element (LCS would report a
    // removal plus an addition), proving the fallback engaged.
    assert_eq!(d.len(), 1);
    assert!(matches!(d.changes()[0].kind, ChangeKind::Changed { .. }));
}
