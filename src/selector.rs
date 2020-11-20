use protofish::context::MessageRef;
use protofish::decode::{FieldValue, Value};
use rpds::Queue;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Selector {
    chain: Queue<SelectorItem>,
}

impl Selector {
    pub fn new<I, T>(items: I) -> Self
    where
        I: IntoIterator<Item = SelectorItem, IntoIter = T>,
        T: Iterator<Item = SelectorItem> + DoubleEndedIterator,
    {
        // The items are specified in encounter order but queued in reverse
        // order so they can be popped off the end as they are encountered.
        let iter = items.into_iter();
        Self {
            chain: iter.collect(),
        }
    }

    pub fn start_value(&self, value: &Value) -> Option<Selector> {
        log::trace!("Selector::start_value");
        match self.chain.peek() {
            None => {
                log::trace!("Selector::start_value -> exhausted");
                None
            },
            Some(first) => {
                match first.match_value(value) {
                    Some(true) => {
                        log::trace!("Selector::start_value({:?}) -> match", first);
                        Some(Selector {
                            chain: self
                                .chain
                                .dequeue()
                                .expect("Peek guarantees there's at least one item"),
                        })
                    },
                    Some(false) => {
                        log::trace!("Selector::start_value({:?}) -> reject", first);
                        None
                    },
                    None => None
                }
            }
        }
    }

    pub fn next_value(self, value: &Value) -> Option<Selector> {
        log::trace!("Selector::next_value");
        match self.chain.peek() {
            None => {
                log::trace!("Selector::next_value -> exhausted");
                None
            },
            Some(first) => {
                match first.match_value(value) {
                    Some(true) => {
                        log::trace!("Selector::next_value({:?}) -> match", first);
                        Some(Selector {
                            chain: self
                                .chain
                                .dequeue()
                                .expect("Peek guarantees there's at least one item"),
                        })
                    },
                    Some(false) => {
                        log::trace!("Selector::next_value({:?}) -> reject", first);
                        None
                    },
                    None => {
                        log::trace!("Selector::next_value({:?}) -> skip", first);
                        Some(Selector {
                            chain: self.chain
                        })
                    }
                }
            }
        }
    }

    pub fn start_field(&self, owner: MessageRef, value: &FieldValue) -> Option<Selector> {
        log::trace!("Selector::start_field");
        match self.chain.peek() {
            None => {
                log::trace!("Selector::start_field -> exhausted");
                None
            }
            Some(first) => {
                if first.match_field(owner, value) {
                    log::trace!("Selector::start_field({:?}) -> match, remaining {}", first, self.chain.len()-1);
                    Some(Selector {
                        chain: self
                            .chain
                            .dequeue()
                            .expect("Peek guarantees there's at least one item"),
                    })
                } else {
                    log::trace!("Selector::start_field({:?}) -> reject", first);
                    None
                }
            }
        }
    }

    pub fn next_field(self, owner: MessageRef, value: &FieldValue) -> Option<Selector> {
        log::trace!("Selector::next_field");
        match self.chain.peek() {
            None => {
                log::trace!("Selector::next_field -> exhausted");
                None
            }
            Some(first) => {
                if first.match_field(owner, value) {
                    log::trace!("Selector::next_field({:?}) -> match, remaining {}", first, self.chain.len()-1);
                    Some(Selector {
                        chain: self
                            .chain
                            .dequeue()
                            .expect("Peek guarantees there's at least one item"),
                    })
                } else {
                    log::trace!("Selector::next_field({:?}) -> reject", first);
                    None
                }
            }
        }
    }

    pub fn is_done(&self) -> bool {
        self.chain.is_empty()
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum SelectorItem {
    Any,
    Message(MessageRef),
    Field(MessageRef, u64),
}

impl SelectorItem {
    pub fn match_value(&self, value: &Value) -> Option<bool> {
        match (self, value) {
            (SelectorItem::Field(..), _) => None,
            (SelectorItem::Any, _) => Some(true),
            (SelectorItem::Message(msg_ref), Value::Message(msg)) => Some(&msg.msg_ref == msg_ref),
            _ => Some(false),
        }
    }

    pub fn match_field(&self, msg: MessageRef, value: &FieldValue) -> bool {
        if let SelectorItem::Field(expected_ref, field) = self {
            return expected_ref == &msg && field == &value.number;
        }

        // Non-field selectors match any field.
        true
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use protofish::{
        decode::{MessageValue, Value},
        Context,
    };

    #[test]
    pub fn match_message() {
        let ctx = Context::parse(&[r#"
            syntax = "proto3";
            package Proto;

            message Foo {}
            message Bar {}
        "#])
        .unwrap();

        let foo_ref = ctx.get_message("Proto.Foo").unwrap();
        let foo_msg = Value::Message(Box::new(MessageValue {
            msg_ref: foo_ref.self_ref.clone(),
            fields: vec![],
            garbage: None,
        }));
        assert!(SelectorItem::Message(foo_ref.self_ref).match_value(&foo_msg).unwrap());

        let bar_ref = ctx.get_message("Proto.Bar").unwrap();
        let bar_msg = Value::Message(Box::new(MessageValue {
            msg_ref: bar_ref.self_ref.clone(),
            fields: vec![],
            garbage: None,
        }));
        assert!(!SelectorItem::Message(foo_ref.self_ref).match_value(&bar_msg).unwrap());
    }

    #[test]
    pub fn match_any() {
        let ctx = Context::parse(&[r#"
            syntax = "proto3";
            package Proto;

            message Foo {}
            message Bar {}
        "#])
        .unwrap();

        let foo_ref = ctx.get_message("Proto.Foo").unwrap();
        let foo_msg = Value::Message(Box::new(MessageValue {
            msg_ref: foo_ref.self_ref.clone(),
            fields: vec![],
            garbage: None,
        }));
        assert!(SelectorItem::Any.match_value(&foo_msg).unwrap());

        let bar_ref = ctx.get_message("Proto.Bar").unwrap();
        let bar_msg = Value::Message(Box::new(MessageValue {
            msg_ref: bar_ref.self_ref.clone(),
            fields: vec![],
            garbage: None,
        }));
        assert!(SelectorItem::Any.match_value(&bar_msg).unwrap());
    }
}
