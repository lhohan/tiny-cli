//! Acceptance tests for config-source semantics
//!
//! This file provides DSL-style behavioural tests for user-visible config-source
//! contracts: mandatory OpenCode config, optional Weave, display names, and
//! source-aware label collection.

use std::collections::HashMap;

use opencode_config_lens::{
    collect_active_usage, load_config_bundle, ConfigError, ConfigSourceFamily, UsageClass,
};

mod support;
use support::scenario::given_config_sources;

#[test]
fn config_sources_should_require_opencode_config() {
    let home = given_config_sources().with_no_opencode().build_home();

    let result = load_config_bundle(&home);

    assert!(
        matches!(result, Err(ConfigError::MissingConfig(_))),
        "should error when opencode.jsonc is missing, got {:?}",
        result
    );
}

#[test]
fn config_sources_should_load_opencode_general_config_as_default_usage_class() {
    let home = given_config_sources()
        .with_opencode_jsonc(r#"{"model": "provider/alpha"}"#)
        .build_home();

    let bundle = load_config_bundle(&home).expect("should load required config");
    let usage = collect_active_usage(&bundle);
    let by_model: HashMap<_, _> = usage.into_iter().collect();

    assert!(by_model.contains_key("provider/alpha"));
    assert_eq!(
        by_model.get("provider/alpha").unwrap()[0].family,
        ConfigSourceFamily::OpenCode
    );
    assert_eq!(
        by_model.get("provider/alpha").unwrap()[0].class,
        UsageClass::Default
    );
    assert_eq!(by_model.get("provider/alpha").unwrap()[0].label, "default");
}

#[test]
fn config_sources_should_load_opencode_agent_config_as_custom_usage_class() {
    let home = given_config_sources()
        .with_opencode_jsonc(
            r#"{
            "agent": {
                "coder": { "model": "provider/beta" }
            }
        }"#,
        )
        .build_home();

    let bundle = load_config_bundle(&home).expect("should load config");
    let usage = collect_active_usage(&bundle);
    let by_model: HashMap<_, _> = usage.into_iter().collect();

    assert!(by_model.contains_key("provider/beta"));
    assert_eq!(
        by_model.get("provider/beta").unwrap()[0].family,
        ConfigSourceFamily::OpenCode
    );
    assert_eq!(
        by_model.get("provider/beta").unwrap()[0].class,
        UsageClass::Custom
    );
    assert_eq!(by_model.get("provider/beta").unwrap()[0].label, "coder");
}

#[test]
fn config_sources_should_allow_weave_config_to_be_absent() {
    let home = given_config_sources()
        .with_opencode_jsonc(r#"{"model": "provider/alpha"}"#)
        .with_no_weave()
        .build_home();

    let bundle = load_config_bundle(&home).expect("should load without weave");

    assert!(bundle.weave.is_none());
}

#[test]
fn config_sources_should_load_weave_config_when_present() {
    let home = given_config_sources()
        .with_opencode_jsonc(r#"{"model": "provider/alpha"}"#)
        .with_weave_jsonc(
            r#"{
            "agents": {
                "reviewer": { "model": "provider/gamma" }
            }
        }"#,
        )
        .build_home();

    let bundle = load_config_bundle(&home).expect("should load with weave");
    let usage = collect_active_usage(&bundle);
    let by_model: HashMap<_, _> = usage.into_iter().collect();

    assert!(bundle.weave.is_some());
    assert!(by_model.contains_key("provider/gamma"));
    assert_eq!(
        by_model.get("provider/gamma").unwrap()[0].family,
        ConfigSourceFamily::Weave
    );
    assert_eq!(
        by_model.get("provider/gamma").unwrap()[0].class,
        UsageClass::Default
    );
}

#[test]
fn config_sources_should_distinguish_weave_agents_from_custom_agents() {
    let home = given_config_sources()
        .with_opencode_jsonc(r#"{"model": "provider/alpha"}"#)
        .with_weave_jsonc(
            r#"{
            "agents": {
                "reviewer": { "model": "provider/beta" }
            },
            "custom_agents": {
                "ops": { "model": "provider/gamma" }
            }
        }"#,
        )
        .build_home();

    let bundle = load_config_bundle(&home).expect("should load config");
    let usage = collect_active_usage(&bundle);
    let by_model: HashMap<_, _> = usage.into_iter().collect();

    assert_eq!(
        by_model.get("provider/beta").unwrap()[0].class,
        UsageClass::Default
    );
    assert_eq!(
        by_model.get("provider/gamma").unwrap()[0].class,
        UsageClass::Custom
    );
}

#[test]
fn config_sources_should_use_weave_display_names_for_labels() {
    let home = given_config_sources()
        .with_opencode_jsonc(r#"{"model": "provider/alpha"}"#)
        .with_weave_jsonc(
            r#"{
            "agents": {
                "reviewer": {
                    "model": "provider/beta",
                    "display_name": "Review Bot"
                }
            },
            "custom_agents": {
                "ops": {
                    "model": "provider/gamma",
                    "display_name": "Ops Assistant"
                }
            }
        }"#,
        )
        .build_home();

    let bundle = load_config_bundle(&home).expect("should load config");
    let usage = collect_active_usage(&bundle);
    let by_model: HashMap<_, _> = usage.into_iter().collect();

    assert_eq!(
        by_model.get("provider/beta").unwrap()[0].label,
        "Review Bot"
    );
    assert_eq!(
        by_model.get("provider/gamma").unwrap()[0].label,
        "Ops Assistant"
    );
}

#[test]
fn config_sources_should_fallback_to_key_when_display_name_missing() {
    let home = given_config_sources()
        .with_opencode_jsonc(r#"{"model": "provider/alpha"}"#)
        .with_weave_jsonc(
            r#"{
            "agents": {
                "reviewer": { "model": "provider/beta" }
            }
        }"#,
        )
        .build_home();

    let bundle = load_config_bundle(&home).expect("should load config");
    let usage = collect_active_usage(&bundle);
    let by_model: HashMap<_, _> = usage.into_iter().collect();

    assert_eq!(by_model.get("provider/beta").unwrap()[0].label, "reviewer");
}

#[test]
fn config_sources_should_collect_usage_from_all_sources_together() {
    let home = given_config_sources()
        .with_opencode_jsonc(
            r#"{
            "model": "provider/shared",
            "agent": {
                "helper": { "model": "provider/shared" }
            }
        }"#,
        )
        .with_weave_jsonc(
            r#"{
            "agents": {
                "reviewer": { "model": "provider/shared" }
            }
        }"#,
        )
        .build_home();

    let bundle = load_config_bundle(&home).expect("should load config");
    let usage = collect_active_usage(&bundle);
    let by_model: HashMap<_, _> = usage.into_iter().collect();

    let shared_usage = by_model
        .get("provider/shared")
        .expect("should have shared model");
    assert_eq!(shared_usage.len(), 3);

    let families: Vec<_> = shared_usage.iter().map(|u| u.family).collect();
    let classes: Vec<_> = shared_usage.iter().map(|u| u.class).collect();
    assert!(families.contains(&ConfigSourceFamily::OpenCode));
    assert!(families.contains(&ConfigSourceFamily::Weave));
    assert!(classes.contains(&UsageClass::Default));
    assert!(classes.contains(&UsageClass::Custom));
}
