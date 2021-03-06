use automafish::{Builder, Transition};
use protofish::context::MessageRef;
use protofish::decode::Value;

use super::*;

impl Action {
    /// Construct a new action out of the selector items and an operation.
    pub fn new<I>(items: I, operation: Box<dyn Fn(&mut protofish::Value)>) -> Self
    where
        I: IntoIterator<Item = SelectorItem>,
    {
        Self {
            pattern: items.into_iter().collect(),
            operation,
            absolute: false,
        }
    }

    pub(crate) fn compile(
        self,
        builder: &mut Builder<Criteria, Box<dyn Fn(&mut protofish::Value)>>,
    ) {
        let initial_state = builder.create_initial_state();

        // If the pattern isn't absolute, add a cycling state at the start that
        // repeats the initial state for each type.
        if !self.absolute {
            let initial_field = builder.new_state();
            builder.add_transition(Transition::new(
                initial_state,
                Criteria::Type(None),
                initial_field,
            ));
            builder.add_transition(Transition::new(
                initial_field,
                Criteria::Field(None),
                initial_state,
            ));
        }
        
        let mut last_ty = initial_state;
        for s in self.pattern {

            // If the selector starts with a field selector, add an any-type
            // type state as the first state.
            if s.field.is_some() && last_ty == initial_state {
                let initial_ty = builder.new_state();
                Transition::new(initial_state, Criteria::Type(None), initial_ty);
                last_ty = initial_ty;
            }

            let field_state = builder.new_state();
            Transition::new(last_ty, Criteria::Field(s.field), field_state);

            let ty_state = builder.new_state();
            Transition::new(field_state, Criteria::Type(s.ty), ty_state);
            last_ty = ty_state;
        }

        // If the selector was empty (ie, no type-state was added), we'll add one any-type state to
        // hold the possible action that ends up executed on the first value (or all values in case
        // absolute == false)
        if last_ty == initial_state {
            let initial_ty = builder.new_state();
            Transition::new(initial_state, Criteria::Type(None), initial_ty);
            last_ty = initial_ty;
        }

        builder.add_action(last_ty, self.operation);
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Criteria {
    Field(Option<FieldSelector>),
    Type(Option<TypeSelector>),
    Empty,
    Any,
}

pub enum Input<'a> {
    Value(&'a protofish::Value),
    Field(MessageRef, &'a protofish::Context, u64),
}

impl<'a> automafish::Criteria<'a> for Criteria {
    type Input = Input<'a>;
    fn is_match(&self, input: &Self::Input) -> bool {
        match (self, input) {
            (Criteria::Field(f), Input::Field(m, c, i)) => {
                f.as_ref().map(|f| f.is_match(*m, c, *i)).unwrap_or(true)
            }
            (Criteria::Type(t), Input::Value(i)) => {
                t.as_ref().map(|t| t.is_match(i)).unwrap_or(true)
            }
            (Criteria::Any, _) => true,
            _ => false,
        }
    }

    fn is_empty(&self) -> bool {
        self == &Criteria::Empty
    }

    fn and(&self, other: &Self) -> Self {
        if self == other {
            return self.clone();
        }

        // Most of our criteria are mutually exclusive. The only cases where 'and' doesn't result
        // in an empty criteria is if one of the criterias is "any field" or "any type".
        match (self, other) {
            (Criteria::Any, Criteria::Field(f))
            | (Criteria::Field(None), Criteria::Field(f))
            | (Criteria::Field(f), Criteria::Any)
            | (Criteria::Field(f), Criteria::Field(None)) => Criteria::Field(f.clone()),
            (Criteria::Any, Criteria::Type(f))
            | (Criteria::Type(None), Criteria::Type(f))
            | (Criteria::Type(f), Criteria::Any)
            | (Criteria::Type(f), Criteria::Type(None)) => Criteria::Type(f.clone()),
            _ => Criteria::Empty,
        }
    }

    fn not(&self, other: &Self) -> Self {
        if self == other {
            return Criteria::Empty;
        }

        // Given the criteria are different, only "any field" or "any type" may overlap with any
        // other criteria.
        //
        // Technically "any - a" should result in criteria that matches anything but 'a' but we'll
        // deal with this by weighing the any criteria below specific criteria so returning 'any'
        // for such case is just fine here.
        match (self, other) {
            (Criteria::Field(..), Criteria::Any)
            | (Criteria::Field(..), Criteria::Field(None))
            | (Criteria::Type(..), Criteria::Any)
            | (Criteria::Type(..), Criteria::Type(None)) => Criteria::Empty,
            _ => self.clone(),
        }
    }

    fn any() -> Self {
        Criteria::Any
    }

    fn evaluation_order(&self) -> usize {
        match self {
            Criteria::Any | Criteria::Type(None) | Criteria::Field(None) => 1,
            _ => 0,
        }
    }
}

impl TypeSelector {
    fn is_match(&self, value: &protofish::Value) -> bool {
        match (self, value) {
            (Self::Message(expected), Value::Message(msg)) => msg.msg_ref == *expected,
            _ => false,
        }
    }
}

impl FieldSelector {
    fn is_match(&self, msg: MessageRef, ctx: &protofish::Context, field_number: u64) -> bool {
        let number = match self {
            FieldSelector::Number(n) => *n,
            FieldSelector::Name(name) => {
                let msg_info = ctx.resolve_message(msg);
                match find_field_number(msg_info, name) {
                    Some(num) => num,
                    None => return false,
                }
            }
        };

        field_number == number
    }
}

fn find_field_number(msg_info: &protofish::MessageInfo, name: &str) -> Option<u64> {
    for (num, f) in &msg_info.fields {
        if f.name == name {
            return Some(*num);
        }
    }
    None
}
