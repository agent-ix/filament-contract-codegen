//! A drift pin over CG's own `verus_proof` text in the shipped table, generated from that text. Its
//! content was checked against FR-402, FR-403, FR-405, FR-408 and FR-409 (Status and Behaviors 11,
//! 41, 23, 18a, 24) and the Verus records of TC-950, TC-951, TC-953, TC-956 and TC-957 on
//! filament-exact at be9359f. Re-check it against those records per TC-070 step 16 when one changes.

const T0: &str = r####"The specification div_rem_post of the production function native_div_rem, for every i128 dividend a and every nonzero i128 divisor b, under each of the three profiles (truncating, floor, Euclidean): the remainder r is in the profile's window, a returned quotient q satisfies a = q b + r exactly, and the quotient is absent only for i128::MIN / -1, whose remainder is zero. This row concerns the quotient q of native_div_rem; the statement holds for every width at once and for symbolic and constant divisors alike."####;

const T1: &str = r####"The choice of the member, the result-interval check and the outcome kinds, which stay with the Kani harnesses of FR-409 Behaviors 13 and 14 and the host differential of Behavior 21. The proof does not change this row's Kani verdict, does not say which divisor shapes the table refuses (that split is IR-1561), and is not a Kani verification of any harness shape."####;

const T2: &str = r####"The prover library's specification of checked_div, checked_add, checked_sub, wrapping_add and wrapping_sub, the specification of i128::wrapping_rem in src/verus_std_specs.rs, and that the derived equality of DivisionProfile compares variants."####;

const T3: &str = r####"make verus on the whole crate: 221 verified, 0 errors; scripts/ir1556_verus_mutants.py: unmutated copy verified, 10 of 10 mutants killed by a failed postcondition, none UNRELIABLE (TC-957 Verus record)."####;

const T4: &str = r####"The specification div_rem_post of the production function native_div_rem, for every i128 dividend a and every nonzero i128 divisor b, under each of the three profiles (truncating, floor, Euclidean): the remainder r is in the profile's window, a returned quotient q satisfies a = q b + r exactly, and the quotient is absent only for i128::MIN / -1, whose remainder is zero. This row concerns the remainder r of native_div_rem; the statement holds for every width at once and for symbolic and constant divisors alike."####;

const T5: &str = r####"The specification div_rem_post of the production function native_div_rem, for every i128 dividend a and every nonzero i128 divisor b, under each of the three profiles (truncating, floor, Euclidean): the remainder r is in the profile's window, a returned quotient q satisfies a = q b + r exactly, and the quotient is absent only for i128::MIN / -1, whose remainder is zero. This row concerns the Euclidean remainder of native_div_rem (Modulo is the Euclidean remainder of the same function); the statement holds for every width at once and for symbolic and constant divisors alike."####;

const T6: &str = r####"Numeric path for two finite nonzero operands: the planner returns the exact intermediate (the quotient of the operands, floored, with a sticky bit and two guard bits), and round returns its correct rounding with the result bits, the overflow result of each direction, inexact and underflow, and decode reads the bits back as the rounded value. The theorems hold for both widths and all six rounding spellings at once."####;

const T7: &str = r####"The common set: the NaN result (its sign, quietness and payload) and the invalid flag; the result and flags for operands outside the row's numeric path (an infinite operand; a zero operand of Divide, SquareRoot or Multiply, with divide_by_zero for a zero divisor; a negative operand of SquareRoot); that no spelling other than Exact refuses; the provenance; the result-unit charge and the order of charges; and the composition inside arithmetic, that is, that the bits and flags round returns for the planner's intermediate are the ones the operation returns."####;

const T8: &str = r####"The prover library's specifications of u64::leading_zeros (with the library axiom axiom_u64_leading_zeros, which relates the count to the value), of u128::is_multiple_of (the divide sticky), of the widening u128::from on a u64, of u64::BITS, and of the fixed-width integer shifts, bit operations and casts; the specification of u128::leading_zeros in src/verus_std_specs.rs; and, from src/verus_kernel_model.rs, that RoundingMode, IeeeWidth and IeeeFlag are the enumerations their source declares and that the derived equality of RoundingMode compares variants. The library lemmas the proof calls (lemma_u64_shl_is_mul, lemma_u64_shr_is_div, lemma_u128_shl_is_mul, lemma_u128_shr_is_div, lemma_u128_low_bits_mask_is_mod, lemma_u128_pow2_no_overflow and the pow2 lemmas) are proved inside the library, which is trusted as shipped. The proof does not depend on the CBMC fma model, whose two faults are recorded in TC-953 under Tool faults."####;

const T9: &str = r####"make verus on the whole crate: 396 verified, 0 errors; scripts/ir1551_verus_mutants.py: unmutated copy verified, 33 of 33 mutants killed by a failed proof obligation, none UNRELIABLE (TC-953 Verus record)."####;

const T10: &str = r####"Numeric path for two finite nonzero operands: the planner returns the exact intermediate (the quotient of the operands, floored, with a sticky bit and two guard bits), and round returns its correct rounding with the result bits, the overflow result of each direction, inexact and underflow, and decode reads the bits back as the rounded value. The theorems hold for both widths and all six rounding spellings at once. Exact is rounded as nearest-even in this statement."####;

const T11: &str = r####"The common set: the NaN result (its sign, quietness and payload) and the invalid flag; the result and flags for operands outside the row's numeric path (an infinite operand; a zero operand of Divide, SquareRoot or Multiply, with divide_by_zero for a zero divisor; a negative operand of SquareRoot); that no spelling other than Exact refuses; the provenance; the result-unit charge and the order of charges; and the composition inside arithmetic, that is, that the bits and flags round returns for the planner's intermediate are the ones the operation returns. The strict Exact refusal is also not proved: the proof decides only when round raises inexact for the intermediate, not that arithmetic refuses on it, nor that it returns the refusal the owner returns."####;

const T12: &str = r####"Numeric path for a positive finite nonzero operand: the planner returns the exact intermediate (the floor square root), and round returns its correct rounding with the result bits, the overflow result of each direction, inexact and underflow, and decode reads the bits back as the rounded value. The theorems hold for both widths and all six rounding spellings at once."####;

const T13: &str = r####"Numeric path for three operands none of which is infinite or NaN: the planner returns the exact intermediate (the exact sum of the exact product and the addend), and round returns its correct rounding with the result bits, the overflow result of each direction, inexact and underflow, and decode reads the bits back as the rounded value. The theorems hold for both widths and all six rounding spellings at once."####;

const T14: &str = r####"Numeric path for two finite nonzero operands: the planner returns the exact intermediate (the exact product), and round returns its correct rounding with the result bits, the overflow result of each direction, inexact and underflow, and decode reads the bits back as the rounded value. The theorems hold for both widths and all six rounding spellings at once."####;

const T15: &str = r####"Numeric path for a positive finite nonzero operand: the planner returns the exact intermediate (the floor square root), and round returns its correct rounding with the result bits, the overflow result of each direction, inexact and underflow, and decode reads the bits back as the rounded value. The theorems hold for both widths and all six rounding spellings at once. Exact is rounded as nearest-even in this statement."####;

const T16: &str = r####"Numeric path for three operands none of which is infinite or NaN: the planner returns the exact intermediate (the exact sum of the exact product and the addend), and round returns its correct rounding with the result bits, the overflow result of each direction, inexact and underflow, and decode reads the bits back as the rounded value. The theorems hold for both widths and all six rounding spellings at once. Exact is rounded as nearest-even in this statement."####;

const T17: &str = r####"Over all i64 pairs, from_i64_parts returns ZeroDenominator for a zero denominator, otherwise NonCanonical unless the pair is canonical by Euclid's gcd (a positive denominator, gcd(|n|, d) = 1, zero only as 0/1), otherwise the carrier of exactly that pair."####;

const T18: &str = r####"Agreement with the owner's Rational::new roundtrip (host differential, FR-408 Behavior 19). No charge is involved."####;

const T19: &str = r####"The specifications of core functions in src/verus_std_specs.rs, and that Meter::charge_units appends an admitted request to the meter's admitted sequence and leaves the sequence unchanged on a denial (src/verus_kernel_model.rs)."####;

const T20: &str = r####"make verus on the whole crate: 211 verified, 0 errors; make verus-mutants (scripts/ir1544_verus_mutants.py): unmutated copy verified, 18 of 18 mutants killed by a failed proof obligation, none UNRELIABLE (TC-956 Verus record)."####;

const T21: &str = r####"The specification-level statement row_equal_structural_is_cross_product: two canonical carriers inside signed 64 bits are the same pair exactly when a q = c p. The production function native_rational_equal is NOT proved in Verus."####;

const T22: &str = r####"That native_rational_equal returns the same-pair answer on its schedule: that half is the Kani harness native_rational_equal_all_parts_is_structural (FR-408-AC-15, VERIFIED in the kernel's record), a Kani verdict this row does not carry. Which requests a given Meter admits (the theorems state the admitted requests in order and the completed value, not that a sufficient Meter admits them; admission stays with FR-359 and the CG-domain Kani rows of FR-408 Behavior 18 and the host tests). Agreement with the owner route is the host differential of FR-408 Behavior 19, not a proof."####;

const T23: &str = r####"For every two carriers with canonical parts inside signed 64 bits, native_rational_order never refuses, and when it completes it returns the comparison of the exact cross products a q and c p after the three requests of FR-408 Behavior 12; an incomplete outcome follows a proper prefix of the requests; no outcome is undefined or refused."####;

const T24: &str = r####"Which requests a given Meter admits (the theorems state the admitted requests in order and the completed value, not that a sufficient Meter admits them; admission stays with FR-359 and the CG-domain Kani rows of FR-408 Behavior 18 and the host tests). Agreement with the owner route is the host differential of FR-408 Behavior 19, not a proof."####;

const T25: &str = r####"For every two carriers with canonical parts inside signed 64 bits, native_rational_add never refuses, and when it completes it returns the exact sum in lowest terms (both exact parts divided by their gcd, canonical) after the four requests of FR-408 Behavior 11, in order, with the amounts computed from the exact parts; an incomplete outcome follows a proper prefix of the requests. The reduction in all three tiers returns the lowest-terms pair, and gcd8, gcd64 and gcd256 equal Euclid's gcd."####;

const T26: &str = r####"For every two carriers with canonical parts inside signed 64 bits, native_rational_subtract never refuses, and when it completes it returns the exact difference in lowest terms (both exact parts divided by their gcd, canonical) after the four requests of FR-408 Behavior 11, in order, with the amounts computed from the exact parts; an incomplete outcome follows a proper prefix of the requests. The reduction in all three tiers returns the lowest-terms pair, and gcd8, gcd64 and gcd256 equal Euclid's gcd."####;

const T27: &str = r####"For every two carriers with canonical parts inside signed 64 bits, native_rational_multiply never refuses, and when it completes it returns the exact product in lowest terms (both exact parts divided by their gcd, canonical) after the four requests of FR-408 Behavior 11, in order, with the amounts computed from the exact parts; an incomplete outcome follows a proper prefix of the requests. The reduction in all three tiers returns the lowest-terms pair, and gcd8, gcd64 and gcd256 equal Euclid's gcd."####;

const T28: &str = r####"For every signed 128-bit coefficient, every i64 coefficient and scale binding, every u32 scale and every native type, with no bound: (a) the normalized view returns the least-scale pair equal in value (zero is (0, 0); otherwise the sign is kept and every removable trailing decimal zero is removed, lowering the scale by the count, so either the scale reached zero or the coefficient is no longer divisible by ten); (b) the membership test on a normalized pair is the owner's rule in exact arithmetic (with s* the larger of the scale and min_scale, a member exactly when s* <= max_scale and lower <= coefficient x 10^(s* - scale) <= upper; a lift that leaves signed 128 bits is outside every native bounds); (c) the constructor returns OutsideCarrierDomain for a scale outside 0..=4294967295 and for a binding whose normalized pair is not a member, and otherwise retains exactly the pair drawn."####;

const T29: &str = r####"The checked type constructor, the ingress from the owner type, and the read-out. The agreement of the native membership with the owner's Decimal route, which the prover does not model; it rests on the host tests of FR-402-AC-1 to AC-10. FR-402-AC-11 (the CG measurement) is not executed by this row."####;

const T30: &str = r####"The prover library's specifications of i128::checked_mul, u32::try_from on an i64, i128::from on an i64 and the truncating division and remainder of i128; and one fact about the kernel crate: the 39-entry table POWERS_OF_TEN holds 10^0 through 10^38. The table is filled by a loop inside a constant initializer, which Verus cannot check, so it is covered by an independent host test in filament-exact (FR-402-AC-15), not by the proof."####;

const T31: &str = r####"make verus on the whole crate: 440 verified, 0 errors; scripts/ir1554_verus_mutants.py: unmutated copy verified, 12 of 12 mutants killed by a failed proof obligation, none UNRELIABLE (TC-950 Verus record)."####;

const T32: &str = r####"For every pair of retained operands (any signed 128-bit coefficient and any u32 scale), every native Decimal type and all six rounding spellings, with no bound, a mathematical statement: (a) admission refuses exactly when a ground of FR-403 Behavior 3 holds for the native plan and names the first in order, and the plan forms the exact intermediate (aligned sum or difference, product at the sum of the scales, negation, the operand itself for Round, and for Divide the quotient in lowest terms with the sign on the numerator); (b) for an admitted operation with a nonzero divisor the placement is exact when the result is an integer multiple of 10^-T (a zero included), the retained coefficient being that multiple; otherwise the placement is a rounding step, which is Inexact under the exact spelling (no rounded coefficient is retained) and, under each other mode, retains the exact result rounded to a multiple of 10^-T in that mode (the floor toward negative, the ceiling toward positive, the neighbour nearer zero toward zero, and for the nearest modes the nearer neighbour with a tie going to the even neighbour or to the one farther from zero), a relation that names one integer; (c) the loss record holds the exact value in lowest terms; (d) settle returns the zero-divisor and inexact decisions, the completed value retained at T exactly when the membership rule admits it, and the domain refusal otherwise, with the rounding flag set exactly for a rounding step; (f) bits_of and digits_of return the bit length and decimal digit count of every u128. None of these functions overflows, divides by zero or indexes out of range."####;

const T33: &str = r####"The agreement with evaluate_decimal and order_numbers, which the prover does not model: the proof is a mathematical statement and is NOT equality with evaluate_decimal and order_numbers (the executable owner route); that agreement rests on the host differential (FR-403-AC-11 and AC-12, FR-403 Behaviors 35 to 38). The charge amounts (power_of_ten_bits, sbits, sdigits and the size records are proved free of overflow only) and the charge sequence of evaluate and order, the Meter and the typed outcome built from the decision; OrderingOperator::holds; the read-out to the owner's types. FR-403-AC-13 (the CG measurement) is not executed by this row."####;

const T34: &str = r####"The prover library's specifications of i128::checked_add, checked_sub and checked_mul, u64::saturating_sub, u128::is_multiple_of, Option::ok_or, Option::is_some, Result::ok, i128::try_from on a u128, the integer and pair Ord::cmp, Ord::max and Ord::min, the lossless From conversions and the fixed-width shifts, bit operations and casts; the specifications in src/verus_std_specs.rs of bool::then_some, i128::checked_neg, i128::div_euclid and i128::rem_euclid for a positive divisor, i128::signum, i128::unsigned_abs, i128::wrapping_neg, Ordering::reverse, u128::leading_zeros, u128::trailing_zeros and u64::from on a bool; that the derived equality of RoundingMode compares variants; and the 39-entry table POWERS_OF_TEN holding 10^0 through 10^38 (not checkable by Verus; FR-402-AC-15)."####;

const T35: &str = r####"make verus on the whole crate: 630 verified, 0 errors; scripts/ir1555_verus_mutants.py: unmutated copy verified, 53 of 53 mutants killed by a failed proof obligation, none UNRELIABLE (TC-951 Verus record)."####;

const T36: &str = r####"For every pair of retained operands (any signed 128-bit coefficient and any u32 scale), with no bound, a mathematical statement: order_values returns the order of the two mathematical values (cl x 10^sr against cr x 10^sl) for every pair of scales, and widening_mul returns the exact 256-bit product; bits_of and digits_of return the bit length and decimal digit count of every u128. None of these functions overflows, divides by zero or indexes out of range."####;

/// Row index, `proved`, `not_proved`, `assumes`, `run`.
pub(crate) const EXPECTED: [(usize, &str, &str, &str, &str); 24] = [
    (11, T0, T1, T2, T3),
    (13, T4, T1, T2, T3),
    (15, T5, T1, T2, T3),
    (26, T6, T7, T8, T9),
    (27, T10, T11, T8, T9),
    (29, T12, T7, T8, T9),
    (32, T13, T7, T8, T9),
    (45, T14, T7, T8, T9),
    (48, T6, T7, T8, T9),
    (49, T10, T11, T8, T9),
    (50, T6, T7, T8, T9),
    (51, T12, T7, T8, T9),
    (52, T15, T11, T8, T9),
    (54, T13, T7, T8, T9),
    (55, T16, T11, T8, T9),
    (60, T17, T18, T19, T20),
    (69, T21, T22, T19, T20),
    (70, T23, T24, T19, T20),
    (71, T25, T24, T19, T20),
    (72, T26, T24, T19, T20),
    (73, T27, T24, T19, T20),
    (74, T28, T29, T30, T31),
    (75, T32, T33, T34, T35),
    (76, T36, T33, T34, T35),
];
