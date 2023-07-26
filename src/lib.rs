use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum MimeType {
    #[serde(rename = "text/plain")]
    TextPlain,
    #[serde(rename = "image/png")]
    ImagePng,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Content {
    pub hash: Option<ssri::Integrity>,
    pub mime_type: MimeType,
    pub terse: String,
    pub tiktokens: usize,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum Packet {
    Add(Add),
    Update(Update),
    Fork(Fork),
    Delete(Delete),
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Add {
    pub id: scru128::Scru128Id,
    pub hash: ssri::Integrity,
    pub stack_id: Option<scru128::Scru128Id>,
    pub source: Option<String>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Update {
    pub id: scru128::Scru128Id,
    pub source_id: scru128::Scru128Id,
    pub hash: Option<ssri::Integrity>,
    pub stack_id: Option<scru128::Scru128Id>,
    pub source: Option<String>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Fork {
    pub id: scru128::Scru128Id,
    pub source_id: scru128::Scru128Id,
    pub hash: Option<ssri::Integrity>,
    pub stack_id: Option<scru128::Scru128Id>,
    pub source: Option<String>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Delete {
    pub id: scru128::Scru128Id,
    pub source_id: scru128::Scru128Id,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Item {
    pub id: scru128::Scru128Id,
    pub touched: Vec<scru128::Scru128Id>,
    pub hash: ssri::Integrity,
    pub parent: Option<scru128::Scru128Id>,
    pub children: Vec<scru128::Scru128Id>,
}

pub struct View {
    pub items: HashMap<scru128::Scru128Id, Item>,
}

impl View {
    pub fn merge(&mut self, packet: Packet) {
        match packet {
            Packet::Add(add) => {
                let item = Item {
                    id: add.id,
                    touched: vec![add.id],
                    hash: add.hash,
                    parent: add.stack_id,
                    children: Vec::new(),
                };
                if let Some(parent_id) = add.stack_id {
                    if let Some(parent) = self.items.get_mut(&parent_id) {
                        parent.children.push(add.id);
                    }
                }
                self.items.insert(add.id, item);
            }
            Packet::Update(update) => {
                if let Some(item) = self.items.get(&update.source_id).cloned() {
                    let mut item = item;
                    item.touched.push(update.id);
                    if let Some(new_hash) = update.hash {
                        item.hash = new_hash;
                    }
                    if let Some(new_stack_id) = update.stack_id {
                        if let Some(old_parent_id) = item.parent {
                            if let Some(old_parent) = self.items.get_mut(&old_parent_id) {
                                old_parent.children.retain(|&id| id != update.source_id);
                            }
                        }
                        item.parent = Some(new_stack_id);
                        if let Some(new_parent) = self.items.get_mut(&new_stack_id) {
                            new_parent.children.push(update.source_id);
                        }
                    }
                    self.items.insert(update.source_id, item);
                }
            }
            Packet::Fork(fork) => {
                if let Some(item) = self.items.get(&fork.source_id) {
                    let mut new_item = item.clone();
                    new_item.id = fork.id;
                    new_item.touched.push(fork.id);
                    if let Some(new_hash) = fork.hash {
                        new_item.hash = new_hash;
                    }
                    if let Some(new_stack_id) = fork.stack_id {
                        new_item.parent = Some(new_stack_id);
                    }
                    if let Some(parent_id) = item.parent {
                        if let Some(parent) = self.items.get_mut(&parent_id) {
                            parent.children.push(fork.id);
                        }
                    }
                    self.items.insert(fork.id, new_item);
                }
            }
            Packet::Delete(delete) => {
                if let Some(item) = self.items.remove(&delete.source_id) {
                    if let Some(parent_id) = item.parent {
                        if let Some(parent) = self.items.get_mut(&parent_id) {
                            parent.children.retain(|&id| id != delete.source_id);
                        }
                    }
                }
            }
        }
    }

    pub fn root(&self) -> Vec<Item> {
        self.items
            .values()
            .filter(|item| item.parent.is_none())
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_view_as_expected(view: &View, expected: Vec<(&str, Vec<&str>)>) {
        let expected: Vec<(ssri::Integrity, Vec<ssri::Integrity>)> = expected
            .into_iter()
            .map(|(stack, items)| {
                (
                    ssri::Integrity::from(stack),
                    items
                        .into_iter()
                        .map(|item| ssri::Integrity::from(item))
                        .collect(),
                )
            })
            .collect();

        let view: Vec<(ssri::Integrity, Vec<ssri::Integrity>)> = view
            .root()
            .iter()
            .map(|item| {
                let children_hashes = item
                    .children
                    .iter()
                    .filter_map(|id| view.items.get(id))
                    .map(|child| child.hash.clone())
                    .collect::<Vec<_>>();
                (item.hash.clone(), children_hashes)
            })
            .collect();
        assert_eq!(view, expected);
    }

    #[test]
    fn test_update_item() {
        let mut view = View {
            items: HashMap::new(),
        };

        let stack_id = scru128::new();
        view.merge(Packet::Add(Add {
            id: stack_id,
            hash: ssri::Integrity::from("Stack 1"),
            stack_id: None,
            source: None,
        }));
        let item_id = scru128::new();
        view.merge(Packet::Add(Add {
            id: item_id,
            hash: ssri::Integrity::from("Item 1"),
            stack_id: Some(stack_id),
            source: None,
        }));

        // User updates the item
        view.merge(Packet::Update(Update {
            id: scru128::new(),
            source_id: item_id,
            hash: Some(ssri::Integrity::from("Item 1 - updated")),
            stack_id: None,
            source: None,
        }));
        assert_view_as_expected(&view, vec![("Stack 1", vec!["Item 1 - updated"])]);
    }

    #[test]
    fn test_fork_item() {
        let mut view = View {
            items: HashMap::new(),
        };

        let stack_id = scru128::new();
        view.merge(Packet::Add(Add {
            id: stack_id,
            hash: ssri::Integrity::from("Stack 1"),
            stack_id: None,
            source: None,
        }));
        let item_id = scru128::new();
        view.merge(Packet::Add(Add {
            id: item_id,
            hash: ssri::Integrity::from("Item 1"),
            stack_id: Some(stack_id),
            source: None,
        }));

        // User forks the original item
        view.merge(Packet::Fork(Fork {
            id: scru128::new(),
            source_id: item_id,
            hash: Some(ssri::Integrity::from("Item 1 - forked")),
            stack_id: Some(stack_id),
            source: None,
        }));
        assert_view_as_expected(&view, vec![("Stack 1", vec!["Item 1", "Item 1 - forked"])]);
    }

    #[test]
    fn test_move_item_to_new_stack() {
        let mut view = View {
            items: HashMap::new(),
        };

        let stack_id = scru128::new();
        view.merge(Packet::Add(Add {
            id: stack_id,
            hash: ssri::Integrity::from("Stack 1"),
            stack_id: None,
            source: None,
        }));
        let item_id = scru128::new();
        view.merge(Packet::Add(Add {
            id: item_id,
            hash: ssri::Integrity::from("Item 1"),
            stack_id: Some(stack_id),
            source: None,
        }));

        // User creates a new Stack "Stack 2"
        let stack_id_2 = scru128::new();
        view.merge(Packet::Add(Add {
            id: stack_id_2,
            hash: ssri::Integrity::from("Stack 2"),
            stack_id: None,
            source: None,
        }));

        // User moves the original item to "Stack 2"
        view.merge(Packet::Update(Update {
            id: scru128::new(),
            source_id: item_id,
            hash: None,
            stack_id: Some(stack_id_2),
            source: None,
        }));

        assert_view_as_expected(
            &view,
            vec![("Stack 1", vec![]), ("Stack 2", vec!["Item 1"])],
        );
    }
}
