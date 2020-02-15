use super::{AutomergeError, ChangeRequest};
use crate::change_context::ChangeContext;
use crate::error::InvalidChangeRequest;
use automerge_backend::OpSet;
use automerge_backend::Value;
use automerge_backend::{ActorID, Change};
use uuid;

pub struct Document {
    op_set: OpSet,
    actor_id: ActorID,
}

impl Document {
    /// Create a new, empty document
    pub fn init() -> Document {
        Document {
            op_set: OpSet::init(),
            actor_id: ActorID(uuid::Uuid::new_v4().to_string()),
        }
    }

    /// Create a new document from a set of changes
    pub fn load(changes: Vec<Change>) -> Result<Document, AutomergeError> {
        let mut doc = Document::init();
        for change in changes {
            doc.apply_change(change)?
        }
        Ok(doc)
    }

    /// Get the current state of the document as a serde_json value
    pub fn state(&self) -> &Value {
        self.op_set.root_value()
    }

    /// Add a single change to the document
    pub fn apply_change(&mut self, change: Change) -> Result<(), AutomergeError> {
        self.op_set.apply_change(change)
    }

    pub fn create_and_apply_change(
        &mut self,
        message: Option<String>,
        requests: Vec<ChangeRequest>,
    ) -> Result<Change, InvalidChangeRequest> {
        let mut change_ctx = ChangeContext::new(
            &self.op_set.object_store,
            self.actor_id.clone(),
            &self.op_set.actor_histories,
            self.op_set.clock.clone(),
        );
        let change = change_ctx.create_change(requests, message)?;
        self.apply_change(change.clone())
            .map_err(|e| InvalidChangeRequest(format!("Error applying change: {:?}", e)))?;
        Ok(change)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change_request::{ListIndex, Path};
    use automerge_backend::Value;
    use automerge_backend::{
        ActorID, Clock, DataType, ElementID, Key, ObjectID, Operation, PrimitiveValue,
    };
    use serde_json;
    use std::collections::HashMap;

    #[test]
    fn test_loading_from_changes() {
        let mut actor1_deps = HashMap::new();
        actor1_deps.insert(ActorID("id1".to_string()), 1);
        let changes = vec![
            Change {
                actor_id: ActorID("id1".to_string()),
                operations: vec![
                    Operation::MakeMap {
                        object_id: ObjectID::ID("2ce778e4-d23f-426f-98d7-e97fea47181c".to_string()),
                    },
                    Operation::Link {
                        object_id: ObjectID::Root,
                        key: Key("cards_by_id".to_string()),
                        value: ObjectID::ID("2ce778e4-d23f-426f-98d7-e97fea47181c".to_string()),
                    },
                    Operation::Set {
                        object_id: ObjectID::Root,
                        key: Key("numRounds".to_string()),
                        value: PrimitiveValue::Number(0.0),
                        datatype: Some(DataType::Counter),
                    },
                    Operation::Set {
                        object_id: ObjectID::Root,
                        key: Key("size_of_cards".to_string()),
                        value: PrimitiveValue::Number(10.0),
                        datatype: None,
                    },
                    Operation::Set {
                        object_id: ObjectID::ID("2ce778e4-d23f-426f-98d7-e97fea47181c".to_string()),
                        key: Key("deleted_key".to_string()),
                        value: PrimitiveValue::Boolean(false),
                        datatype: None,
                    },
                    Operation::Delete {
                        object_id: ObjectID::ID("2ce778e4-d23f-426f-98d7-e97fea47181c".to_string()),
                        key: Key("deleted_key".to_string()),
                    },
                    Operation::MakeList {
                        object_id: ObjectID::ID("87cef98c-246d-42b8-ada5-28524f5aefb3".to_string()),
                    },
                    Operation::Link {
                        object_id: ObjectID::Root,
                        key: Key("cards".to_string()),
                        value: ObjectID::ID("87cef98c-246d-42b8-ada5-28524f5aefb3".to_string()),
                    },
                    Operation::Insert {
                        list_id: ObjectID::ID("87cef98c-246d-42b8-ada5-28524f5aefb3".to_string()),
                        key: ElementID::Head,
                        elem: 1,
                    },
                    Operation::Set {
                        object_id: ObjectID::ID("87cef98c-246d-42b8-ada5-28524f5aefb3".to_string()),
                        key: Key("id1:1".to_string()),
                        value: PrimitiveValue::Number(1.0),
                        datatype: None,
                    },
                    Operation::Insert {
                        list_id: ObjectID::ID("87cef98c-246d-42b8-ada5-28524f5aefb3".to_string()),
                        key: ElementID::SpecificElementID(ActorID("id1".to_string()), 1),
                        elem: 2,
                    },
                    Operation::Set {
                        object_id: ObjectID::ID("87cef98c-246d-42b8-ada5-28524f5aefb3".to_string()),
                        key: Key("id1:2".to_string()),
                        value: PrimitiveValue::Boolean(false),
                        datatype: None,
                    },
                ],
                seq: 1,
                message: Some("initialization".to_string()),
                dependencies: Clock::empty(),
            },
            Change {
                actor_id: ActorID("id1".to_string()),
                operations: vec![
                    Operation::Increment {
                        object_id: ObjectID::Root,
                        key: Key("numRounds".to_string()),
                        value: 5.0,
                    },
                    Operation::Set {
                        object_id: ObjectID::Root,
                        key: Key("size_of_cards".to_string()),
                        value: PrimitiveValue::Number(12.0),
                        datatype: None,
                    },
                ],
                seq: 2,
                message: Some("incrementation".to_string()),
                dependencies: Clock(actor1_deps.clone()),
            },
            Change {
                actor_id: ActorID("id2".to_string()),
                operations: vec![
                    Operation::Increment {
                        object_id: ObjectID::Root,
                        key: Key("numRounds".to_string()),
                        value: 6.0,
                    },
                    Operation::Set {
                        object_id: ObjectID::Root,
                        key: Key("size_of_cards".to_string()),
                        value: PrimitiveValue::Number(13.0),
                        datatype: None,
                    },
                ],
                seq: 1,
                message: Some("actor 2 incrementation".to_string()),
                dependencies: Clock(actor1_deps),
            },
        ];
        let doc = Document::load(changes).unwrap();
        let expected: serde_json::Value = serde_json::from_str(
            r#"
            {
                "cards_by_id": {},
                "size_of_cards": 12.0,
                "numRounds": 11.0,
                "cards": [1.0, false]
            }
        "#,
        )
        .unwrap();
        let actual_state = doc.state().to_json();
        assert_eq!(actual_state, expected)
    }

    #[test]
    fn test_set_mutation() {
        let mut doc = Document::init();
        let json_value: serde_json::Value = serde_json::from_str(
            r#"
            {
                "cards_by_id": {},
                "size_of_cards": 12.0,
                "numRounds": 11.0,
                "cards": [1.0, false]
            }
        "#,
        )
        .unwrap();
        doc.create_and_apply_change(
            Some("Some change".to_string()),
            vec![ChangeRequest::Set {
                path: Path::root().key("the-state".to_string()),
                value: Value::from_json(&json_value),
            }],
        )
        .unwrap();
        let expected: serde_json::Value = serde_json::from_str(
            r#"
            {
                "the-state": {
                    "cards_by_id": {},
                    "size_of_cards": 12.0,
                    "numRounds": 11.0,
                    "cards": [1.0, false]
                }
            }
        "#,
        )
        .unwrap();
        assert_eq!(expected, doc.state().to_json());

        doc.create_and_apply_change(
            Some("another change".to_string()),
            vec![ChangeRequest::Set {
                path: Path::root()
                    .key("the-state".to_string())
                    .key("size_of_cards".to_string()),
                value: Value::from_json(&serde_json::Value::Number(
                    serde_json::Number::from_f64(10.0).unwrap(),
                )),
            }],
        )
        .unwrap();

        let expected: serde_json::Value = serde_json::from_str(
            r#"
            {
                "the-state": {
                    "cards_by_id": {},
                    "size_of_cards": 10.0,
                    "numRounds": 11.0,
                    "cards": [1.0, false]
                }
            }
        "#,
        )
        .unwrap();
        assert_eq!(expected, doc.state().to_json());
    }

    #[test]
    fn test_move_ops() {
        let mut doc = Document::init();
        let json_value: serde_json::Value = serde_json::from_str(
            r#"
            {
                "cards_by_id": {
                    "jack": {"value": 11}
                },
                "size_of_cards": 12.0,
                "numRounds": 11.0,
                "cards": [1.0, false]
            }
        "#,
        )
        .unwrap();
        doc.create_and_apply_change(
            Some("Init".to_string()),
            vec![ChangeRequest::Set {
                path: Path::root(),
                value: Value::from_json(&json_value),
            }],
        )
        .unwrap();
        println!("Doc state: {:?}", doc.state().to_json());
        doc.create_and_apply_change(
            Some("Move jack".to_string()),
            vec![
                ChangeRequest::Move {
                    from: Path::root()
                        .key("cards_by_id".to_string())
                        .key("jack".to_string()),
                    to: Path::root()
                        .key("cards_by_id".to_string())
                        .key("jill".to_string()),
                },
                ChangeRequest::Move {
                    from: Path::root().key("size_of_cards".to_string()),
                    to: Path::root().key("number_of_cards".to_string()),
                },
            ],
        )
        .unwrap();

        let expected: serde_json::Value = serde_json::from_str(
            r#"
            {
                "cards_by_id": {
                    "jill": {"value": 11.0}
                },
                "number_of_cards": 12.0,
                "numRounds": 11.0,
                "cards": [1.0, false]
            }
        "#,
        )
        .unwrap();
        assert_eq!(expected, doc.state().to_json());
    }

    #[test]
    fn test_delete_op() {
        let json_value: serde_json::Value = serde_json::from_str(
            r#"
            {
                "cards_by_id": {
                    "jack": {"value": 11}
                },
                "size_of_cards": 12.0,
                "numRounds": 11.0,
                "cards": [1.0, false]
            }
        "#,
        )
        .unwrap();
        let mut doc = Document::init();
        doc.create_and_apply_change(
            Some("Init".to_string()),
            vec![ChangeRequest::Set {
                path: Path::root(),
                value: Value::from_json(&json_value),
            }],
        )
        .unwrap();
        doc.create_and_apply_change(
            Some("Delete everything".to_string()),
            vec![
                ChangeRequest::Delete {
                    path: Path::root()
                        .key("cards_by_id".to_string())
                        .key("jack".to_string()),
                },
                ChangeRequest::Delete {
                    path: Path::root()
                        .key("cards".to_string())
                        .index(ListIndex::Index(1)),
                },
            ],
        )
        .unwrap();

        let expected: serde_json::Value = serde_json::from_str(
            r#"
            {
                "cards_by_id": {},
                "size_of_cards": 12.0,
                "numRounds": 11.0,
                "cards": [1.0]
            }
        "#,
        )
        .unwrap();
        assert_eq!(expected, doc.state().to_json());
    }

    #[test]
    fn test_insert_ops() {
        let json_value: serde_json::Value = serde_json::from_str(
            r#"
            {
                "values": [1.0, false]
            }
        "#,
        )
        .unwrap();
        let mut doc = Document::init();
        doc.create_and_apply_change(
            Some("Initial".to_string()),
            vec![ChangeRequest::Set {
                path: Path::root(),
                value: Value::from_json(&json_value),
            }],
        )
        .unwrap();
        let person_json: serde_json::Value = serde_json::from_str(
            r#"
            {
                "name": "fred",
                "surname": "johnson"
            }
            "#,
        )
        .unwrap();
        doc.create_and_apply_change(
            Some("list additions".to_string()),
            vec![
                ChangeRequest::InsertAfter {
                    path: Path::root()
                        .key("values".to_string())
                        .index(ListIndex::Head),
                    value: Value::from_json(&person_json),
                },
                ChangeRequest::InsertAfter {
                    path: Path::root()
                        .key("values".to_string())
                        .index(ListIndex::Index(1)),
                    value: Value::from_json(&serde_json::Value::String("final".to_string())),
                },
            ],
        )
        .unwrap();
        let expected: serde_json::Value = serde_json::from_str(
            r#"
            {
                "values": [
                    {
                        "name": "fred",
                        "surname": "johnson"
                    },
                    1.0,
                    false,
                    "final"
                ]
            }
            "#,
        )
        .unwrap();
        assert_eq!(expected, doc.state().to_json());
    }
}
