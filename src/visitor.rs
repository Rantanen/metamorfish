
use protofish::{Context, Value, MessageValue};
use automafish::{Builder, StateMachine, MachineState};

use crate::action::{Criteria, Action, Input};

pub struct Visitor {
    state_machine: StateMachine<Criteria, Box<dyn Fn(&mut Value)>>,
}

impl Visitor {
    pub fn new<I>(actions: I) -> Self
    where
        I: IntoIterator<Item = Action>
    {
        let mut builder = Builder::new();
        for a in actions {
            a.compile(&mut builder);
        }

        Self {
            state_machine: builder.build()
        }
    }

    pub fn visit(&self, ctx: &Context, value: &mut Value)
    {
        self.visit_value(self.state_machine.start(), ctx, value);
    }

    fn visit_value(&self, state: MachineState, ctx: &Context, value: &mut Value)
    {
        let state = self.state_machine.step(state, &Input::Value(value));
        self.state_machine.execute(state, value);

        if let Value::Message(m) = value {
           self.visit_message(state, ctx, m);
        }
    }

    fn visit_message(&self, state: MachineState, ctx: &Context, msg: &mut MessageValue)
    {
        for f in &mut msg.fields {
            let state = self.state_machine.step(state, &Input::Field(msg.msg_ref, ctx, f.number));
            self.visit_value(state, ctx, &mut f.value);
        }
    }
}
