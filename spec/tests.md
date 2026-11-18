---
id: TM-008
title: "filament-contract-codegen test matrix index"
type: TestMatrixIndex
---
# TM-008: filament-contract-codegen test matrix index

Each subsystem owns one matrix beside its requirements, per
`filament-contract-ir:ADR-0056`, whose registry is in [spec.md](./spec.md). This index declares no test case; the matrices are the coverage authority.

## Requirements Traceability

| Subsystem | Requirements | Local Matrix | Status |
|---|---|---|---|
| Core | StR-001, FR-005, FR-037, interface-001, NFR-001, NFR-002, NFR-005 | [core](./core/matrix/tests.md) | 🚧 TC-001 is partial: NFR-002 headers are covered; FR-005-AC-2 and NFR-001-AC-1 remain planned. TC-002, TC-003, TC-007 and TC-052 are planned |
| Strategy | FR-002, FR-008, FR-009, FR-010, FR-011, FR-012, FR-013, NFR-004 | [strategy](./strategy/matrix/tests.md) | 🚧 FR-008 to FR-013 and NFR-004 are covered; FR-002 (TC-004) is planned |
| Oracle | FR-014, FR-018, FR-021, FR-031, FR-035, FR-042 | [oracle](./oracle/matrix/tests.md) | 🚧 several criteria are partial or planned in the matrix rows; FR-031 (TC-044) has its FSL exemplar run pending and the active divide and remainder criteria FR-031-AC-2, AC-6, AC-16, AC-17, AC-18, AC-22 and AC-26 to AC-27, which are planned (IR-602); paths that call `filament_exact::divide` or remove the interim code are gated on FSL moving to the single-member `divide` first; known red tests of IR-1566 are listed under "Known red tests" below |
| Kani | FR-015, FR-017, FR-025, FR-028, FR-029, FR-030, [FR-047](./kani/functional/FR-047-v2-function-proof-producer.md), NFR-006 | [kani](./kani/matrix/tests.md) | 🚧 several criteria are planned in the matrix rows; FR-047 and TC-061 current-V2 function proof producer are Planned (IR-1121); NFR-006 (TC-045) is planned; the minimal run ownership of ADR-007 (FR-017-AC-65 to AC-68, FR-028-AC-21, FR-028-AC-25) is met on Linux by IR-1510, which removed the namespace/bubblewrap path; FR-034 and TC-049 are retired.; known red tests of IR-1566 are listed under "Known red tests" below |
| Routed | FR-019, FR-022, FR-026 | [routed](./routed/matrix/tests.md) | 🚧 Process cases of FR-019-AC-1, AC-11 to AC-13 and AC-16 to AC-23; FR-022-AC-17 to AC-22; and FR-026-AC-5 are planned for TC-046 or analysis. FR-019-AC-49 to AC-52 are covered; replacement AC-54 is planned in TC-030. FR-026-AC-1 and AC-4 remain planned for TC-037; FR-022-AC-1, AC-6 and AC-16 remain open |
| Replay | FR-016, FR-024, FR-032, FR-033 | [replay](./replay/matrix/tests.md) | 🚧 FR-016-AC-6 is covered by bounded witness decode tests; AC-7 and FR-024-AC-7, AC-8, AC-31 to AC-35 and replacement AC-47 to AC-51 are planned; FR-016-AC-12 is covered by an injected executor-fault test; FR-024-AC-5 and AC-9 are retired; FR-024-AC-2 to AC-4, AC-6 and AC-10 are covered by focused intake and source-layout tests; FR-032 (TC-047) is Covered by native and real-Kani tests (IR-631) except FR-032-AC-18, which is Partial, and its whole-route use awaits the driver's same-proved-artifact execution (IR-610); FR-033-AC-7/9/12/13 and TC-048 are Partial (IR-666 direct public report controls); IR-806 original-artifact builder, generated-family and content-authentication checks remain Planned, while FSL #645 has delivered the parity API |
| Evidence | FR-004, [FR-036](./evidence/functional/FR-036-source-obligation-census.md) | [evidence](./evidence/matrix/tests.md) | 🚧 FR-004 (TC-006) is planned; FR-036 (TC-051) complete producer/consumer conformance is Planned and Gated on IR-759 |


## Known red tests (IR-1566)

Moving CG to the latest first-party pins leaves 42 `--test it` tests red by design; each passes on the
previous pins. The criteria they back carry a "Known red" prefix in their requirement rows.

- QSL-2173 (38 tests, 18 `float_scalar_harness`, 17 `enum_scalar_harness`, 3 `scalar_source_forms`): the corpus
  sources contain a Float64 `=`, and FSL refuses the whole package with `FloatEqualityScheduleNotEmitted`. Criteria:
  FR-053-AC-1, AC-2, AC-3, AC-5, AC-6, AC-7, AC-11, AC-12, AC-14 (TC-067); FR-049-AC-1, AC-2, AC-4, AC-7 to AC-11
  (TC-063); FR-014-AC-12, AC-67 to AC-69 and FR-038-AC-3 (TC-024).
  - `enum_scalar_harness::a_decoded_witness_replays_through_the_owner_route_as_the_declared_members`
  - `enum_scalar_harness::a_member_the_owner_does_not_declare_is_refused_by_the_owner_route`
  - `enum_scalar_harness::a_missing_owner_declaration_is_unsupported_with_a_named_capability`
  - `enum_scalar_harness::a_node_of_another_family_is_not_an_enum_comparison`
  - `enum_scalar_harness::a_refused_enum_sibling_in_one_request_leaves_the_supported_enum_nodes_unchanged`
  - `enum_scalar_harness::a_timeout_and_a_missing_verdict_settle_the_enum_harness_inconclusive`
  - `enum_scalar_harness::a_valid_binding_pair_decodes_to_the_declared_members`
  - `enum_scalar_harness::an_enum_schema_with_bounds_off_the_member_list_is_refused`
  - `enum_scalar_harness::each_harness_ends_with_exactly_one_completion_cover`
  - `enum_scalar_harness::every_operator_generates_one_native_carrier_harness_from_its_node`
  - `enum_scalar_harness::perturbed_bindings_are_refused_with_typed_causes`
  - `enum_scalar_harness::the_assertion_requires_completed_and_no_assumption_reads_the_outcome`
  - `enum_scalar_harness::the_classifier_reads_the_enum_harness_cover_as_unsatisfied_missing_or_unwound`
  - `enum_scalar_harness::the_record_retains_declaration_operator_encoding_unwind_and_options`
  - `enum_scalar_harness::the_routed_path_generates_enum_nodes_and_leaves_other_families_byte_identical`
  - `enum_scalar_harness::the_symbolic_domain_is_exactly_the_declared_members`
  - `enum_scalar_harness::two_enum_nodes_of_one_operator_in_one_request_do_not_collide`
  - `float_scalar_harness::a_decoded_witness_replays_through_the_owner_route_with_exact_bits`
  - `float_scalar_harness::a_literal_operand_and_a_non_parameter_operand_are_refused_under_their_own_names`
  - `float_scalar_harness::a_routed_request_settles_comparisons_and_refuses_arithmetic_siblings`
  - `float_scalar_harness::an_unknown_witness_width_is_refused_not_replayed_as_binary64`
  - `float_scalar_harness::an_unpinned_float_operand_requires_a_bound_naming_its_type`
  - `float_scalar_harness::arithmetic_is_refused_with_a_named_not_tractable_capability`
  - `float_scalar_harness::each_harness_ends_with_exactly_one_completion_cover`
  - `float_scalar_harness::every_comparison_generates_one_native_carrier_harness_from_its_node`
  - `float_scalar_harness::float32_witnesses_are_range_checked_and_keep_their_bits`
  - `float_scalar_harness::float64_witnesses_keep_their_exact_bit_patterns`
  - `float_scalar_harness::malformed_float_bindings_and_schemas_are_refused_with_typed_causes`
  - `float_scalar_harness::the_assertion_requires_completed_and_no_assumption_reads_the_outcome`
  - `float_scalar_harness::the_draw_encoding_matches_the_class_boundary_set_and_every_encoding_mutant_fails`
  - `float_scalar_harness::the_expected_side_reaches_no_native_operation_and_a_sharing_mutant_fails_the_inspection`
  - `float_scalar_harness::the_fsl_equality_operator_generates_the_numeric_equal_row`
  - `float_scalar_harness::the_identity_records_the_operation_bindings_options_and_dependency_cohort`
  - `float_scalar_harness::the_symbolic_domain_is_every_bit_pattern_of_the_width`
  - `float_scalar_harness::unsupported_operand_shapes_are_refused_with_named_capabilities`
  - `scalar_source_forms::genuine_ieee_equality_and_intrinsics_generate`
  - `scalar_source_forms::ieee_extension_preserves_binary_support_and_refuses_other_shapes`
  - `scalar_source_forms::same_shape_ieee_descriptor_mismatch_retains_caller_declared_generation`
- IR-1568 (4 tests): the shipped IR-1530 table is recorded at kernel_version 1 and filament-exact is at KERNEL_VERSION 2, so
  every row is ignored (FR-056-AC-7) and the 31 `not_tractable` rows refuse nothing (FR-056-AC-15 is inert) until the rows are
  re-measured. Criteria: FR-056-AC-15 (TC-070); FR-057-AC-7 and AC-9 (TC-071).
  - `integer_division_harness::a_division_harness_at_or_above_its_row_width_is_refused_before_kani_and_below_it_runs`
  - `integer_division_harness::the_shipped_table_holds_the_kernel_native_division_rows_on_the_native_route`
  - `kani_tractability::tc_070_a_general_route_scalar_harness_is_refused_before_kani_with_its_row`
  - `kani_tractability::tc_070_native_route_and_v2_clause_features_are_not_refused_by_the_general_row`
