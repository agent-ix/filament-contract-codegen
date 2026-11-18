// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Component execution of the production emitter, without claiming strict IR admission.

use super::{
    CompositeEqualityItem, EqualityOperandDescriptor, EqualityOperatorKind, RenderedItem,
    SourceBuilder,
};
use ix_trace_rs::trace;
use std::{fs, process::Command};

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nrecord Point { x: Boolean; }\nrecord OtherPoint { x: Boolean; }\nfunction subject using v(a: Boolean, b: Boolean): Boolean pure { a = b }\n";

#[trace("TC-029", "FR-018-AC-48", "FR-018-AC-49")]
#[test]
fn emitted_preparation_retains_native_errors_and_semantic_precedence() {
    let compiled = fsl_replay::compile_package(
        fsl_replay::CompileUnit {
            source: fsl_replay::SourceIdentity::new("test", "equality-preparation.native"),
            package_name: "equality-native-preparation".to_owned(),
            path: "equality-preparation.native",
            bytes: SOURCE.as_bytes(),
        },
        [],
        &fsl_replay::DependencyInput::default(),
        fsl_replay::StageLimits::default(),
        fsl_replay::ReplayLimits::default(),
    )
    .expect("the public producer issues real component identities");
    let packet: serde_json::Value = serde_json::from_slice(compiled.bytes()).unwrap();
    let nodes = packet["semantic_graph"]["nodes"].as_array().unwrap();
    let node_id = serde_json::from_value(
        nodes
            .iter()
            .find(|node| node["body"]["operation"]["identity"] == "filament.op.boolean.eq")
            .unwrap()["node_id"]
            .clone(),
    )
    .unwrap();
    let boolean = serde_json::from_value(
        nodes
            .iter()
            .find(|node| node["node_tag"] == "scalar_type" && node["semantic_form"] == "boolean")
            .unwrap()["node_id"]
            .clone(),
    )
    .unwrap();
    let item = CompositeEqualityItem {
        node_id,
        operator: EqualityOperatorKind::Equal,
        left: EqualityOperandDescriptor {
            source_type: boolean,
            conversion_target: None,
        },
        right: EqualityOperandDescriptor {
            source_type: serde_json::from_value(
                nodes
                    .iter()
                    .find(|node| {
                        node["node_tag"] == "scalar_type" && node["semantic_form"] == "boolean"
                    })
                    .unwrap()["node_id"]
                    .clone(),
            )
            .unwrap(),
            conversion_target: None,
        },
    };
    let mut builder = SourceBuilder::new(super::MAX_GENERATED_SOURCE_BYTES).unwrap();
    builder.expected_package = Some("equality-native-preparation".into());
    builder.expected_package_bytes = Some(String::from_utf8(compiled.bytes().to_vec()).unwrap());
    builder
        .item(
            "component",
            &item,
            &RenderedItem {
                union_seals: String::new(),
                requires_context: false,
                composites: Vec::new(),
                left_source: "exact::ValueType::Boolean".to_owned(),
                left_target: None,
                // Arm only after descriptor reconstruction, immediately before the real owner walk.
                right_source:
                    "{ native_checks::arm_at_type_boundary(); exact::ValueType::Boolean }"
                        .to_owned(),
                right_target: None,
            },
        )
        .unwrap();
    let artifacts = builder.finish(super::MAX_GENERATED_SOURCE_BYTES).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let scratch = root.join("target/tmp/equality-native-preparation");
    fs::create_dir_all(scratch.join("src")).unwrap();
    for artifact in artifacts {
        fs::write(scratch.join(artifact.path), artifact.contents).unwrap();
    }
    let library = scratch.join("src/lib.rs");
    let emitted = fs::read_to_string(&library).unwrap();
    fs::write(&library, format!("{emitted}\n{NATIVE_CHECKS}")).unwrap();
    fs::write(scratch.join("source.native"), SOURCE).unwrap();
    fs::write(scratch.join("checked-package.json"), compiled.bytes()).unwrap();
    let manifest = format!("{}\n[dev-dependencies]\nfsv-failing-allocator = {{ {} }}\nfilament-exact = {{ {}, features = [\"test-support\"] }}\n", crate::core::profile::equality_oracle_crate_manifest("equality-native-preparation"), crate::core::profile::SEMANTIC_VALUE_DEPENDENCY_SOURCE, crate::core::profile::EXACT_DEPENDENCY_SOURCE);
    fs::write(scratch.join("Cargo.toml"), manifest).unwrap();
    fs::copy(root.join("Cargo.lock"), scratch.join("Cargo.lock")).unwrap();
    // The scratch adds a dependency from a git source the lock already pins, and resolving that
    // offline would move every package of the source to the newest cached branch head. Hold each
    // such package at the revision the root lock names.
    let lock = fs::read_to_string(root.join("Cargo.lock")).unwrap();
    for package in ["filament-semantic-value", "filament-exact"] {
        let rev = locked_git_revision(&lock, package);
        let hold = Command::new(env!("CARGO"))
            .args(["update", "--offline", "-p", package, "--precise", &rev])
            .current_dir(&scratch)
            .output()
            .unwrap();
        assert!(
            hold.status.success(),
            "hold {package} at {rev}: {}",
            String::from_utf8_lossy(&hold.stderr)
        );
    }
    let metadata = Command::new(env!("CARGO"))
        .args(["metadata", "--offline", "--format-version", "1"])
        .current_dir(&scratch)
        .output()
        .unwrap();
    assert!(
        metadata.status.success(),
        "native metadata: {}",
        String::from_utf8_lossy(&metadata.stderr)
    );
    let run = Command::new(env!("CARGO"))
        .args(["test", "--locked", "--offline", "--lib"])
        .current_dir(&scratch)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "emitted native preparation failed:\n{}\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(String::from_utf8_lossy(&run.stdout)
        .contains("test native_checks::preparation_preserves_owner_errors_and_semantics ... ok"));
}

const NATIVE_CHECKS: &str = r#"
#[cfg(test)]
mod native_checks {
use super::*;
use std::cell::Cell;
use sv::{StorageUnit, TypeCheckError};
#[global_allocator]
static ALLOCATOR: fsv_failing_allocator::FailingAllocator = fsv_failing_allocator::FailingAllocator;
thread_local! { static DENY_AT_BOUNDARY: Cell<Option<usize>> = const { Cell::new(None) }; }
pub(super) fn arm_at_type_boundary() {
    if let Some(count) = DENY_AT_BOUNDARY.with(Cell::take) { fsv_failing_allocator::arm(count); }
}
const LIMITS: exact::ScalarLimits = exact::ScalarLimits {
    integer_bits: 4096, decimal_digits: 4096, scale_expansion: 4096,
    text_input_bytes: 4096, text_scalars: 4096, normalized_scalars: 4096,
    unit_edges: 4096, value_occurrences: 4096, work_units: 4096, result_units: 4096,
};
#[test]
fn preparation_preserves_owner_errors_and_semantics() {
    let compiled = fsl_replay::compile_admitted(fsl_replay::CompileUnit {
        source: fsl_replay::SourceIdentity::new("test", "equality-preparation.native"),
        package_name: "equality-native-preparation".to_owned(), path: "equality-preparation.native",
        bytes: include_bytes!("../source.native"),
    }, [], &fsl_replay::DependencyInput::default(), fsl_replay::StageLimits::default(), fsl_replay::ReplayLimits::default()).unwrap();
    let context = compiled.context();
    let direct_environment = sv::TypeEnvironment::default();
    fsv_failing_allocator::arm(0);
    let expected = direct_environment.try_check_type(&exact::ValueType::Boolean);
    let direct_bytes = fsv_failing_allocator::disarm();
    let Err(TypeCheckError::Allocation(expected)) = expected else { panic!("real owner type-walk denial"); };
    assert!(direct_bytes.is_some_and(|bytes| bytes > 0));
    assert_eq!(expected.unit(), StorageUnit::AdditionalElements);
    assert_eq!(expected.amount(), 1);
    // Both left-native and right-native denial run through the actual emitted constructor.
    for allowed in [0, 1] {
        DENY_AT_BOUNDARY.with(|slot| slot.set(Some(allowed)));
        let result = environment_component(Some(context));
        let denied = fsv_failing_allocator::disarm();
        assert_eq!(denied, direct_bytes);
        let Err(EnvironmentError::TypeCheck(TypeCheckError::Allocation(actual))) = result else { panic!("native error must stop preparation without a capability"); };
        assert_eq!(actual, expected);
    }
    let prepared = environment_component(Some(context)).unwrap();
    let left = exact::Value::Boolean(true);
    let right = exact::Value::Boolean(false);
    let mut meter = exact::Meter::new(LIMITS);
    let actual = oracle_component(&prepared, &left, &right, &mut meter);
    let checked = direct_environment.check_equality(sv::EqualityOperator::Equal, sv::EqualityOperand::typed(exact::ValueType::Boolean), sv::EqualityOperand::typed(exact::ValueType::Boolean), &filament_semantic_value::enumeration::EnumMemberIndex::default()).unwrap();
    let mut direct_meter = exact::Meter::new(LIMITS);
    assert_eq!(actual, checked.evaluate(&direct_environment, &left, &right, &mut direct_meter));
    assert_eq!(actual, exact::Outcome::Completed(false));
    assert_eq!(meter.admitted_charges(), direct_meter.admitted_charges());
    for kind in exact::LimitKind::ALL { assert_eq!(meter.consumed(kind), direct_meter.consumed(kind)); }
    let point = context.composites().find(|declaration| declaration.qualified_name().last().unwrap().as_str() == "Point").unwrap().key().clone();
    let other = context.composites().find(|declaration| declaration.qualified_name().last().unwrap().as_str() == "OtherPoint").unwrap().key().clone();
    let missing = exact::ValueType::Composite(point);
    let wrong = sv::TypeEnvironment::new([sv::CompositeDeclaration::new(other, "OtherPoint", sv::CompositeShape::Record(vec![sv::FieldDeclaration::new("x", exact::ValueType::Boolean, exact::Presence::Required)]))], []).unwrap();
    for environment in [sv::TypeEnvironment::default(), wrong] {
        let Err(TypeCheckError::Refused(expected)) = environment.try_check_type(&missing) else { panic!("real owner missing-key cause"); };
        let mut private_fault = environment_component(Some(context)).unwrap();
        private_fault.environment = environment;
        private_fault.left_source = missing.clone();
        private_fault.left_target = None;
        // Left consumes its root reservation and refuses. A right walk would hit this denial.
        fsv_failing_allocator::arm(1);
        let retained = comparison_type_refusal(&private_fault.environment, &private_fault.left_source, &private_fault.right_source);
        let denied = fsv_failing_allocator::disarm();
        assert_eq!(denied, None, "left semantic refusal prevents the right native failure");
        private_fault.type_refusal = retained.unwrap();
        assert_eq!(private_fault.type_refusal, Some(expected));
        let mut meter = exact::Meter::new(LIMITS);
        fsv_failing_allocator::arm(0);
        let refused = oracle_component(&private_fault, &left, &right, &mut meter);
        let repeated_walk = fsv_failing_allocator::disarm();
        assert_eq!(repeated_walk, None, "oracle consumes the retained cause without repeating type traversal");
        assert_eq!(refused, exact::Outcome::Refused(exact::Refusal::CheckedInvariant { cause: exact::CheckedInvariantCause::GeneratedTypeCheckRejected { cause: expected.cause } }));
        assert!(meter.admitted_charges().is_empty());
        private_fault.binding = "different";
        assert_eq!(oracle_component(&private_fault, &left, &right, &mut meter), exact::Outcome::Refused(exact::Refusal::CheckedInvariant { cause: exact::CheckedInvariantCause::GeneratedDescriptorReconstructionFailed }));
        assert!(meter.admitted_charges().is_empty());
        let right_refusal = comparison_type_refusal(&private_fault.environment, &exact::ValueType::Boolean, &private_fault.left_source).unwrap();
        assert_eq!(right_refusal, Some(expected));
    }
}
}
"#;

/// The git revision `Cargo.lock` records for `package`.
fn locked_git_revision(lock: &str, package: &str) -> String {
    let header = format!("name = \"{package}\"\n");
    let entry = lock
        .split("[[package]]")
        .find(|entry| entry.contains(&header));
    let source = entry
        .and_then(|entry| {
            entry
                .lines()
                .find(|line| line.starts_with("source = \"git+"))
        })
        .unwrap_or_else(|| panic!("{package} is a locked git package"));
    source
        .rsplit('#')
        .next()
        .unwrap()
        .trim_end_matches('"')
        .to_owned()
}
