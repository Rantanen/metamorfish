use automafish::{Builder, MachineState};
use protofish::{Context, MessageValue, Value};

use super::*;
use crate::action::Input;

impl Visitor {
    /// Combine multiple actions into a single visitor.
    pub fn new<I>(actions: I) -> Self
    where
        I: IntoIterator<Item = Action>,
    {
        let mut builder = Builder::new();
        for a in actions {
            a.compile(&mut builder);
        }

        Self {
            state_machine: builder.build(),
        }
    }

    /// Execute the visitor on a value.
    ///
    /// Executing the visitor will walk through the value and perform any operations defined on the
    /// visitor actions on values matching their patterns.
    pub fn execute(&self, ctx: &Context, value: &mut Value) {
        self.visit_value(self.state_machine.start(), ctx, value);
    }

    fn visit_value(&self, state: MachineState, ctx: &Context, value: &mut Value) {
        let state = self.state_machine.step(state, &Input::Value(value));
        self.state_machine.execute(state, value);

        if let Value::Message(m) = value {
            self.visit_message(state, ctx, m);
        }
    }

    fn visit_message(&self, state: MachineState, ctx: &Context, msg: &mut MessageValue) {
        for f in &mut msg.fields {
            let state = self
                .state_machine
                .step(state, &Input::Field(msg.msg_ref, ctx, f.number));
            self.visit_value(state, ctx, &mut f.value);
        }
    }
}
