use std::cell::RefCell;
use std::rc::Rc;

use crate::selector::Selector;
use protofish::context::{MessageInfo, MessageRef};
use protofish::decode::{MessageValue, Value};
use rpds::HashTrieSet;

pub struct Visitor {
    actions: Vec<Action>,
}

#[derive(Clone)]
struct Action {
    selector: Selector,
    action: Rc<RefCell<dyn FnMut(&mut Value)>>,
}

#[derive(Clone)]
struct VisitorState {
    selectors: Vec<Action>,
}

impl VisitorState {}

impl Visitor {
    pub fn visit(&self, value: Value) -> Value {
        let mut value = value;
        let state = VisitorState { selectors: vec![] };
        self.visit_impl(&mut value, state);

        value
    }

    fn visit_impl(&self, value: &mut Value, state: VisitorState) {
        let selectors = state
            .selectors
            .into_iter()
            .filter_map(|s| {
                let action = s.action;
                s.selector.next_value(&value).map(|next| Action {
                    selector: next,
                    action,
                })
            })
            .chain(self.actions.iter().filter_map(|s| {
                s.selector.start_value(&value).map(|next| Action {
                    selector: next,
                    action: s.action.clone(),
                })
            }));

        let (done, pending): (Vec<_>, Vec<_>) = selectors.partition(|s| s.selector.is_done());
        for d in done {
            log::trace!("Visitor::visit_impl, invoking {:?}", d.selector);
            d.action.borrow_mut()(value);
        }

        let next_state = VisitorState { selectors: pending };

        match value {
            Value::Message(msg) => self.visit_msg(msg, next_state),
            _ => {}
        }
    }

    fn visit_msg(&self, value: &mut Box<MessageValue>, state: VisitorState) {
        log::trace!("Visitor::visit_msg: {:?}", value.msg_ref);
        let msg_ref = value.msg_ref;
        for f in &mut value.fields {
            log::trace!("Visitor::visit_msg, field: {:?}", f.number);
            let selectors = state
                .clone()
                .selectors
                .into_iter()
                .filter_map(|s| {
                    let action = s.action;
                    s.selector.next_field(msg_ref, f).map(|next| Action {
                        selector: next,
                        action,
                    })
                })
                .chain(self.actions.iter().filter_map(|s| {
                    s.selector.start_field(msg_ref, f).map(|next| Action {
                        selector: next,
                        action: s.action.clone(),
                    })
                }));
            let (done, pending): (Vec<_>, Vec<_>) = selectors.partition(|s| s.selector.is_done());
            for d in done {
                log::trace!("Visitor::visit_msg, invoking action on {:?}", f.value);
                d.action.borrow_mut()(&mut f.value);
            }

            self.visit_impl(&mut f.value, VisitorState { selectors: pending })
        }
    }
}

#[cfg(test)]
mod test {
    use test_env_log::test;

    use super::*;
    use crate::selector::{Selector, SelectorItem};
    use protofish::Context;
    use protofish::decode::{Value, FieldValue, MessageValue};

    #[test]
    pub fn visit_msg() {
        let ctx = Context::parse(&[r#"
            syntax = "proto3";
            package Proto;

            message Foo {}
            message Bar {}
        "#])
        .unwrap();

        let foo_ref = ctx.get_message("Proto.Foo").unwrap().self_ref;
        let bar_ref = ctx.get_message("Proto.Bar").unwrap().self_ref;

        let msg = Value::Message(Box::new(MessageValue {
            msg_ref: foo_ref.clone(),
            garbage: None,
            fields: vec![
                FieldValue {
                    number: 1,
                    value: Value::Message(Box::new(MessageValue {
                        garbage: None,
                        msg_ref: bar_ref.clone(),
                        fields: vec![
                            FieldValue {
                                number: 1,
                                value: Value::UInt64(1),
                            },
                            FieldValue {
                                number: 2,
                                value: Value::Bool(false),
                            },
                            FieldValue {
                                number: 3,
                                value: Value::String("original".to_string()),
                            },
                        ],
                    })),
                },
                FieldValue {
                    number: 2,
                    value: Value::Message(Box::new(MessageValue {
                        garbage: None,
                        msg_ref: bar_ref.clone(),
                        fields: vec![
                            FieldValue {
                                number: 1,
                                value: Value::UInt64(1),
                            },
                            FieldValue {
                                number: 2,
                                value: Value::Bool(false),
                            },
                            FieldValue {
                                number: 3,
                                value: Value::String("original".to_string()),
                            },
                        ],
                    })),
                },
            ],
        }));

        let visitor = {
            Visitor {
            actions: vec![
                Action {
                    selector: Selector::new(vec![SelectorItem::Field(bar_ref, 2)]),
                    action: Rc::new(RefCell::new(|value: &mut Value| *value = Value::Bool(true))),
                },
                Action {
                    selector: Selector::new(vec![
                          SelectorItem::Field(foo_ref, 1),
                          SelectorItem::Field(bar_ref, 3)
                    ]),
                    action: Rc::new(RefCell::new(|value: &mut Value| *value = Value::String("changed".to_string()))),
                },
            ],
        }};

        let visited = visitor.visit(msg);

        let foo_msg = match visited {
            Value::Message(msg) => msg,
            _ => panic!("Not a message"),
        };

        assert_eq!(foo_msg.fields[0].number, 1);
        let bar1_msg = match &foo_msg.fields[0].value {
            Value::Message(msg) => msg,
            _ => panic!("Not a message"),
        };

        assert_eq!(bar1_msg.fields[1].number, 2);
        let bool1_value = match &bar1_msg.fields[1].value {
            Value::Bool(b) => b,
            _ => panic!("Not a bool value"),
        };
        assert_eq!(bool1_value, &true);

        assert_eq!(bar1_msg.fields[2].number, 3);
        let string1_value = match &bar1_msg.fields[2].value {
            Value::String(s) => s,
            _ => panic!("Not a string value"),
        };
        assert_eq!(string1_value, "changed");

        assert_eq!(foo_msg.fields[1].number, 2);
        let bar2_msg = match &foo_msg.fields[1].value {
            Value::Message(msg) => msg,
            _ => panic!("Not a message"),
        };

        assert_eq!(bar2_msg.fields[1].number, 2);
        let bool2_value = match &bar2_msg.fields[1].value {
            Value::Bool(b) => b,
            _ => panic!("Not a bool value"),
        };
        assert_eq!(bool2_value, &true);

        assert_eq!(bar2_msg.fields[2].number, 3);
        let string2_value = match &bar2_msg.fields[2].value {
            Value::String(s) => s,
            _ => panic!("Not a string value"),
        };
        assert_eq!(string2_value, "original");

    }
}
