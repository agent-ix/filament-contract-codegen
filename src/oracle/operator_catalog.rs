//! Closed typed identities consumed by admitted V2 generators.
//!
//! This is a CG-side projection of identities already admitted by Contract IR. It does not
//! participate in admission and deliberately contains only operations with current V2 consumers.

use filament_exact::OrderingOperator;

/// Integer `+`, `-`, `*` and unary `-` shared by the catalog and scalar oracle API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntegerOperator {
    /// `a + b`.
    Add,
    /// `a - b`.
    Subtract,
    /// `a * b`.
    Multiply,
    /// `-a`.
    Negate,
}

/// Typed identity for an operation shared by scalar oracle and Kani consumers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Operator {
    Add,
    Subtract,
    Multiply,
    Negate,
    Divide,
    Remainder,
    Modulo,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Equal,
    NotEqual,
}

/// Complete metadata that differs between consumers for one catalog entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Entry {
    pub(crate) identity: &'static str,
    pub(crate) family: Family,
    pub(crate) operands: OperandOrder,
    pub(crate) result: ResultKind,
    pub(crate) scalar_operator: Option<IntegerOperator>,
    pub(crate) scalar_kani: Option<ScalarKaniOperator>,
    pub(crate) ordering_operator: Option<OrderingOperator>,
    pub(crate) frame_symbol: Option<FrameSymbol>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Family {
    Arithmetic,
    Division,
    Ordering,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OperandOrder {
    Unary,
    LeftThenRight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResultKind {
    Integer,
    Boolean,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FrameSymbol {
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Equal,
    NotEqual,
}

impl FrameSymbol {
    pub(crate) const fn rust(self) -> &'static str {
        match self {
            Self::Less => "<",
            Self::LessOrEqual => "<=",
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
            Self::Equal => "==",
            Self::NotEqual => "!=",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ScalarKaniOperator {
    Add,
    Subtract,
    Multiply,
    Negate,
    /// `div`: the quotient under the claim's law.
    Divide,
    /// `rem`: the remainder under the claim's law.
    Remainder,
    /// `mod`: the Euclidean remainder.
    Modulo,
}

impl Operator {
    pub(crate) const ALL: [Self; 13] = [
        Self::Add,
        Self::Subtract,
        Self::Multiply,
        Self::Negate,
        Self::Divide,
        Self::Remainder,
        Self::Modulo,
        Self::Less,
        Self::LessOrEqual,
        Self::Greater,
        Self::GreaterOrEqual,
        Self::Equal,
        Self::NotEqual,
    ];

    pub(crate) const fn entry(self) -> Entry {
        use Operator as O;
        let (
            identity,
            family,
            operands,
            result,
            scalar_operator,
            scalar_kani,
            ordering_operator,
            frame_symbol,
        ) = match self {
            O::Add => (
                "filament.op.integer.add",
                Family::Arithmetic,
                OperandOrder::LeftThenRight,
                ResultKind::Integer,
                Some(IntegerOperator::Add),
                Some(ScalarKaniOperator::Add),
                None,
                None,
            ),
            O::Subtract => (
                "filament.op.integer.sub",
                Family::Arithmetic,
                OperandOrder::LeftThenRight,
                ResultKind::Integer,
                Some(IntegerOperator::Subtract),
                Some(ScalarKaniOperator::Subtract),
                None,
                None,
            ),
            O::Multiply => (
                "filament.op.integer.mul",
                Family::Arithmetic,
                OperandOrder::LeftThenRight,
                ResultKind::Integer,
                Some(IntegerOperator::Multiply),
                Some(ScalarKaniOperator::Multiply),
                None,
                None,
            ),
            O::Negate => (
                "filament.op.integer.negate",
                Family::Arithmetic,
                OperandOrder::Unary,
                ResultKind::Integer,
                Some(IntegerOperator::Negate),
                Some(ScalarKaniOperator::Negate),
                None,
                None,
            ),
            O::Divide => (
                "filament.op.integer.div",
                Family::Division,
                OperandOrder::LeftThenRight,
                ResultKind::Integer,
                None,
                Some(ScalarKaniOperator::Divide),
                None,
                None,
            ),
            O::Remainder => (
                "filament.op.integer.rem",
                Family::Division,
                OperandOrder::LeftThenRight,
                ResultKind::Integer,
                None,
                Some(ScalarKaniOperator::Remainder),
                None,
                None,
            ),
            O::Modulo => (
                "filament.op.integer.mod",
                Family::Division,
                OperandOrder::LeftThenRight,
                ResultKind::Integer,
                None,
                Some(ScalarKaniOperator::Modulo),
                None,
                None,
            ),
            O::Less => (
                "filament.op.integer.lt",
                Family::Ordering,
                OperandOrder::LeftThenRight,
                ResultKind::Boolean,
                None,
                None,
                Some(OrderingOperator::Less),
                Some(FrameSymbol::Less),
            ),
            O::LessOrEqual => (
                "filament.op.integer.le",
                Family::Ordering,
                OperandOrder::LeftThenRight,
                ResultKind::Boolean,
                None,
                None,
                Some(OrderingOperator::LessOrEqual),
                Some(FrameSymbol::LessOrEqual),
            ),
            O::Greater => (
                "filament.op.integer.gt",
                Family::Ordering,
                OperandOrder::LeftThenRight,
                ResultKind::Boolean,
                None,
                None,
                Some(OrderingOperator::Greater),
                Some(FrameSymbol::Greater),
            ),
            O::GreaterOrEqual => (
                "filament.op.integer.ge",
                Family::Ordering,
                OperandOrder::LeftThenRight,
                ResultKind::Boolean,
                None,
                None,
                Some(OrderingOperator::GreaterOrEqual),
                Some(FrameSymbol::GreaterOrEqual),
            ),
            O::Equal => (
                "filament.op.integer.eq",
                Family::Ordering,
                OperandOrder::LeftThenRight,
                ResultKind::Boolean,
                None,
                None,
                None,
                Some(FrameSymbol::Equal),
            ),
            O::NotEqual => (
                "filament.op.integer.ne",
                Family::Ordering,
                OperandOrder::LeftThenRight,
                ResultKind::Boolean,
                None,
                None,
                None,
                Some(FrameSymbol::NotEqual),
            ),
        };
        Entry {
            identity,
            family,
            operands,
            result,
            scalar_operator,
            scalar_kani,
            ordering_operator,
            frame_symbol,
        }
    }

    pub(crate) fn from_identity(identity: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|operation| operation.entry().identity == identity)
    }

    pub(crate) fn from_ordering_operator(operator: OrderingOperator) -> Option<Self> {
        match operator {
            OrderingOperator::Less => Some(Self::Less),
            OrderingOperator::LessOrEqual => Some(Self::LessOrEqual),
            OrderingOperator::Greater => Some(Self::Greater),
            OrderingOperator::GreaterOrEqual => Some(Self::GreaterOrEqual),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("FR-038-AC-1", "FR-038-AC-2", "FR-038-AC-3")]
    #[test]
    fn catalog_entries_preserve_identity_family_arity_result_and_consumer_projection() {
        let expected = [
            (
                "filament.op.integer.add",
                Family::Arithmetic,
                OperandOrder::LeftThenRight,
                ResultKind::Integer,
                Some(ScalarKaniOperator::Add),
                None,
            ),
            (
                "filament.op.integer.sub",
                Family::Arithmetic,
                OperandOrder::LeftThenRight,
                ResultKind::Integer,
                Some(ScalarKaniOperator::Subtract),
                None,
            ),
            (
                "filament.op.integer.mul",
                Family::Arithmetic,
                OperandOrder::LeftThenRight,
                ResultKind::Integer,
                Some(ScalarKaniOperator::Multiply),
                None,
            ),
            (
                "filament.op.integer.negate",
                Family::Arithmetic,
                OperandOrder::Unary,
                ResultKind::Integer,
                Some(ScalarKaniOperator::Negate),
                None,
            ),
            (
                "filament.op.integer.div",
                Family::Division,
                OperandOrder::LeftThenRight,
                ResultKind::Integer,
                Some(ScalarKaniOperator::Divide),
                None,
            ),
            (
                "filament.op.integer.rem",
                Family::Division,
                OperandOrder::LeftThenRight,
                ResultKind::Integer,
                Some(ScalarKaniOperator::Remainder),
                None,
            ),
            (
                "filament.op.integer.mod",
                Family::Division,
                OperandOrder::LeftThenRight,
                ResultKind::Integer,
                Some(ScalarKaniOperator::Modulo),
                None,
            ),
            (
                "filament.op.integer.lt",
                Family::Ordering,
                OperandOrder::LeftThenRight,
                ResultKind::Boolean,
                None,
                Some(FrameSymbol::Less),
            ),
            (
                "filament.op.integer.le",
                Family::Ordering,
                OperandOrder::LeftThenRight,
                ResultKind::Boolean,
                None,
                Some(FrameSymbol::LessOrEqual),
            ),
            (
                "filament.op.integer.gt",
                Family::Ordering,
                OperandOrder::LeftThenRight,
                ResultKind::Boolean,
                None,
                Some(FrameSymbol::Greater),
            ),
            (
                "filament.op.integer.ge",
                Family::Ordering,
                OperandOrder::LeftThenRight,
                ResultKind::Boolean,
                None,
                Some(FrameSymbol::GreaterOrEqual),
            ),
            (
                "filament.op.integer.eq",
                Family::Ordering,
                OperandOrder::LeftThenRight,
                ResultKind::Boolean,
                None,
                Some(FrameSymbol::Equal),
            ),
            (
                "filament.op.integer.ne",
                Family::Ordering,
                OperandOrder::LeftThenRight,
                ResultKind::Boolean,
                None,
                Some(FrameSymbol::NotEqual),
            ),
        ];
        assert_eq!(Operator::ALL.len(), expected.len());
        for (operation, expected) in Operator::ALL.into_iter().zip(expected) {
            let entry = operation.entry();
            assert_eq!(
                (
                    entry.identity,
                    entry.family,
                    entry.operands,
                    entry.result,
                    entry.scalar_kani,
                    entry.frame_symbol
                ),
                (expected.0, expected.1, expected.2, expected.3, expected.4, expected.5)
            );
            assert_eq!(Operator::from_identity(entry.identity), Some(operation));
        }
        assert_eq!(
            Operator::from_identity("filament.op.integer.eq")
                .unwrap()
                .entry()
                .scalar_operator,
            None
        );
        assert_eq!(Operator::from_identity("filament.op.boolean.and"), None);
    }
}
