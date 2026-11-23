//! Packages compiled from FSL functions whose bodies hold the control applications.

use filament_contract_model::{
    CheckedNodeId, CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2,
    CheckedPackageV2ReadResult, CheckedTerm,
};

const SOURCE: &str = concat!(
    "language \"ix:native\" edition \"1-draft\";\n",
    "profile v = \"filament.value.complete/v1\";\n",
    "union Kind { A, B, C }\n",
    "union Shape { Empty, Circle(Integer) }\n",
    "function pick using v(k: Kind): Integer pure { case k { C: 3; A: 1; B: 2; } }\n",
    "function words using v(k: Kind, t: Text[0, 8; nfc]): Text[0, 8; nfc] pure { case k { A: t; B: t; C: t; } }\n",
    "function area using v(s: Shape): Integer pure { case s { Empty: 0; Circle(r): r; } }\n",
    "function texts using v(c: Boolean, t: Text[0, 8; nfc]): Text[0, 8; nfc] pure { if c then t else t }\n",
    "function plain using v(c: Boolean): Integer pure { if c then 1 else 2 }\n",
);

pub(crate) fn package() -> CheckedPackageV2 {
    let compiled = fsl_replay::compile_admitted(
        fsl_replay::CompileUnit {
            source: fsl_replay::SourceIdentity::new("test", "controls.native"),
            package_name: "controls".to_owned(),
            path: "controls.native",
            bytes: SOURCE.as_bytes(),
        },
        [],
        &fsl_replay::DependencyInput::default(),
        fsl_replay::StageLimits::default(),
        fsl_replay::ReplayLimits::default(),
    )
    .expect("the control functions compile");
    let mut evidence = CheckedPackageEvidence::new();
    evidence.support_feature("filament.value.complete/v1");
    let read = CheckedPackageV2::read(
        compiled.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    );
    let CheckedPackageV2ReadResult::Admitted(package) = read else {
        panic!("the control package is admitted: {read:?}")
    };
    *package
}

/// The arguments of the application of `identity` that is the body of the source function
/// `function`.
pub(crate) fn function_application<'a>(
    package: &'a CheckedPackageV2,
    function: &str,
    identity: &str,
) -> &'a [CheckedTerm] {
    let graph = package.resolved_graph();
    let declaration = graph
        .nodes
        .iter()
        .find(|node| {
            node.declaration
                .as_ref()
                .is_some_and(|found| found.qualified_name == [Box::<str>::from(function)])
        })
        .unwrap_or_else(|| panic!("source function {function}"));
    let CheckedTerm::Aggregate { members, .. } = &declaration.body else {
        panic!("{function} has no function body binding");
    };
    let root = members
        .iter()
        .find_map(|member| match member {
            CheckedTerm::Binding { name, value, .. } if name.as_ref() == "body" => {
                match value.as_ref() {
                    CheckedTerm::Reference { target } => Some(target),
                    _ => None,
                }
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("{function} has no checked body root"));
    match package.term(root) {
        Some(CheckedTerm::Application {
            operation,
            arguments,
            ..
        }) if operation.identity() == Some(identity) => arguments.as_slice(),
        other => panic!("{function} body is not an application of {identity}: {other:?}"),
    }
}

/// The parameter node a control's scrutinee or condition reads.
pub(crate) fn read_node(term: &CheckedTerm) -> &CheckedNodeId {
    match term {
        CheckedTerm::Reference { target } => target,
        other => panic!("expected a reference, got {other:?}"),
    }
}

/// A scalar type node of the package, which is not a union declaration.
pub(crate) fn scalar_type_node(package: &CheckedPackageV2) -> CheckedNodeId {
    package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| &*node.node_tag == "scalar_type")
        .expect("a scalar type node")
        .node_id
        .clone()
}
