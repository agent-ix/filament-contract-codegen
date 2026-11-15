//! Packages compiled from FSL functions whose bodies hold the control applications.

use quire_contract_model::{
    CheckedNodeId, CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2,
    CheckedPackageV2ReadResult, CheckedTerm,
};

const SOURCE: &str = concat!(
    "language \"ix:native\" edition \"1-draft\";\n",
    "profile v = \"quire.value.complete/v1\";\n",
    "union Kind { A, B, C }\n",
    "union Shape { Empty, Circle(Integer) }\n",
    "function pick using v(k: Kind): Integer pure { case k { C: 3; A: 1; B: 2; } }\n",
    "function words using v(k: Kind, t: Text[0, 8; nfc]): Text[0, 8; nfc] pure { case k { A: t; B: t; C: t; } }\n",
    "function area using v(s: Shape): Integer pure { case s { Empty: 0; Circle(r): r; } }\n",
    "function texts using v(c: Boolean, t: Text[0, 8; nfc]): Text[0, 8; nfc] pure { if c then t else t }\n",
    "function plain using v(c: Boolean): Integer pure { if c then 1 else 2 }\n",
);

pub(crate) fn package() -> CheckedPackageV2 {
    let compiled = qsl_replay::compile_admitted(
        qsl_replay::CompileUnit {
            source: qsl_replay::SourceIdentity::new("test", "controls.native"),
            package_name: "controls".to_owned(),
            path: "controls.native",
            bytes: SOURCE.as_bytes(),
        },
        [],
        &qsl_replay::DependencyInput::default(),
        qsl_replay::StageLimits::default(),
        qsl_replay::ReplayLimits::default(),
    )
    .expect("the control functions compile");
    let mut evidence = CheckedPackageEvidence::new();
    evidence.support_feature("quire.value.complete/v1");
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

/// The arguments of every application of `identity`, in node order.
pub(crate) fn applications<'a>(
    package: &'a CheckedPackageV2,
    identity: &str,
) -> Vec<&'a [CheckedTerm]> {
    package
        .resolved_graph()
        .nodes
        .iter()
        .filter_map(|node| package.term(&node.node_id))
        .filter_map(|term| match term {
            CheckedTerm::Application {
                operation,
                arguments,
                ..
            } if operation.identity() == Some(identity) => Some(arguments.as_slice()),
            _ => None,
        })
        .collect()
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
