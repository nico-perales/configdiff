//! Property-based tests: invariants that must hold for *any* pair of documents.
//!
//! These complement the example-based tests in `behavior.rs` by generating
//! arbitrary value trees and asserting structural laws — reflexivity, symmetry,
//! and JSON round-tripping — plus the fact that the engine never panics.

use configdiff::{ArrayStrategy, DiffOptions, Format, Value, diff, parse};
use indexmap::IndexMap;
use proptest::prelude::*;

/// A strategy that produces arbitrary [`Value`] trees.
///
/// Floats are bounded and finite: `NaN` is deliberately excluded because it is
/// never equal to itself, which would (correctly) break the reflexivity law we
/// assert below — that is a property of IEEE floats, not of the diff engine.
fn arb_value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(Value::Integer),
        (-1e9f64..1e9f64).prop_map(Value::Float),
        "[a-zA-Z0-9 _-]{0,12}".prop_map(Value::String),
    ];

    leaf.prop_recursive(4, 32, 6, |inner| {
        let keys = "[a-zA-Z][a-zA-Z0-9_]{0,7}";
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(Value::Array),
            prop::collection::vec((keys, inner), 0..6).prop_map(|pairs| {
                // Collecting into an IndexMap dedups keys, matching how a parsed
                // object behaves (no duplicate keys survive).
                let map: IndexMap<String, Value> = pairs.into_iter().collect();
                Value::Object(map)
            }),
        ]
    })
}

proptest! {
    /// A document never differs from itself.
    #[test]
    fn reflexive_no_self_difference(v in arb_value()) {
        let d = diff(&v, &v, &DiffOptions::default());
        prop_assert!(d.is_empty(), "self-diff was not empty: {:?}", d.changes());
    }

    /// Reflexivity also holds with subtree expansion enabled.
    #[test]
    fn reflexive_under_expand(v in arb_value()) {
        let opts = DiffOptions::default().expand(true);
        prop_assert!(diff(&v, &v, &opts).is_empty());
    }

    /// Whether two documents are equal does not depend on comparison order.
    #[test]
    fn equality_is_symmetric(a in arb_value(), b in arb_value()) {
        let opts = DiffOptions::default();
        prop_assert_eq!(
            diff(&a, &b, &opts).is_empty(),
            diff(&b, &a, &opts).is_empty()
        );
    }

    /// Serializing a value to JSON and parsing it back yields an equal value.
    #[test]
    fn json_round_trip(v in arb_value()) {
        let text = serde_json::to_string(&v).expect("serialize");
        let reparsed = parse(&text, Format::Json).expect("reparse");
        let d = diff(&v, &reparsed, &DiffOptions::default());
        prop_assert!(d.is_empty(), "round-trip changed value: {:?}", d.changes());
    }

    /// No array strategy ever panics, whatever the inputs.
    #[test]
    fn array_strategies_never_panic(a in arb_value(), b in arb_value()) {
        for strategy in [ArrayStrategy::Lcs, ArrayStrategy::Positional, ArrayStrategy::Keyed] {
            let opts = DiffOptions::default()
                .array_strategy(strategy)
                .array_keys(vec!["id".to_owned(), "name".to_owned()]);
            let _ = diff(&a, &b, &opts);
        }
    }

    /// Loose-number and tolerance options never panic and keep self-diff empty.
    #[test]
    fn number_options_are_stable(v in arb_value()) {
        let opts = DiffOptions::default()
            .numbers_loose(true)
            .float_tolerance(Some(0.001));
        prop_assert!(diff(&v, &v, &opts).is_empty());
    }
}
