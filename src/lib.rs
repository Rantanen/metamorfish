//! Metamorfish
#![warn(missing_docs)]

use automafish::StateMachine;
use protofish::{context::MessageRef, Value};

mod action;
mod visitor;

use action::Criteria;

/// Visitor engine
///
/// The `Visitor` combines multiple [`Action`] definitions into a single state machine and executes
/// the actions for an input [`protofish::Value`].
pub struct Visitor {
    state_machine: StateMachine<Criteria, Box<dyn Fn(&mut Value)>>,
}

/// A single action definition.
///
/// An action consists of a pattern and an operation. The pattern selects the input value for the
/// operation. The pattern itself is built out of a chain of [`SelectorItem`].
pub struct Action {
    pattern: Vec<SelectorItem>,
    operation: Box<dyn Fn(&mut protofish::Value)>,
    absolute: bool,
}

/// A raw selector item definition.
///
/// Each selector item consists of a field selector and a type selector.
///
/// Note that matching the pattern starts on the root value, which is not a value of any field. If
/// the first selector defines a field condition, a dummy "any type" condition is added in front of
/// it when constructing the visitor.
///
/// Specifying [`Self::field`] allows limiting the selector to specific field. Specifying
/// [`Self::ty`] allows limiting the selector to specific types. While both of these can be
/// specified, the fields are already limited to a specific type and if the type specified on the
/// selector doesn't match that type, the selector ends up rejecting the value at runtime.
///
/// The only exception to the above is the case where multiple visitors are executed sequentially
/// and a previous visitor has replaced a value with a value of different type - however this
/// would result in an invalid message and probably indicates an error in the operation.
pub struct SelectorItem {

    /// Field selector.
    ///
    /// If specified, the selector matches only specific fields.
    pub field: Option<FieldSelector>,

    /// Type selector.
    ///
    /// If specified, the selector matches only specific types.
    pub ty: Option<TypeSelector>,
}

/// Type selector.
///
/// Specifies the condition to match the value type.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum TypeSelector {
    /// Matches a specific message type.
    Message(MessageRef),
}

/// Field selector.
///
/// Specifies how a field should be matched.
///
/// **Note:** During visitor execution, all fields are always matched by number. The option to
/// specify the field by name allows matching fields of the same name on different messages easily
/// but comes with a slight performance cost for having to resolve the field number at runtime.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum FieldSelector {
    /// Matches a field by number.
    Number(u64),

    /// Matches a field by name.
    Name(String),
}
