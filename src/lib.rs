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
                if let Some(item) = self.items.get_mut(&update.id) {
                    item.touched.push(update.id);
                    if let Some(new_hash) = update.hash {
                        item.hash = new_hash;
                    }
                    if let Some(new_stack_id) = update.stack_id {
                        item.parent = Some(new_stack_id);
                    }
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

    #[test]
    fn test_merge() {
        let mut view = View {
            items: HashMap::new(),
        };

        // Scenario: User starts the app, a Stack is created with current timestamp and current
        // clipboard content
        let now = chrono::Utc::now();
        let stack_name = format!(
            "# {}, at {}",
            now.format("%a, %b %d, %Y"),
            now.format("%I:%M %p %Z")
        );
        let stack_id = scru128::new();
        let stack_packet = Packet::Add(Add {
            id: stack_id,
            hash: ssri::Integrity::from(stack_name.clone()),
            stack_id: None,
            source: Some(stack_name),
        });
        view.merge(stack_packet);

        // Current clipboard content is added to the Stack
        let clipboard_packet = Packet::Add(Add {
            id: scru128::new(),
            hash: ssri::Integrity::from("Hello"),
            stack_id: Some(stack_id),
            source: None,
        });
        view.merge(clipboard_packet);

        // Check that the Stack and the clipboard content are in the view
        assert!(view.items.contains_key(&stack_id));
        assert_eq!(view.items[&stack_id].children.len(), 1);

        // Check that the root items contain the Stack
        let root_items = view.root();
        assert_eq!(root_items.len(), 1);
        assert_eq!(root_items[0].id, stack_id);

        // Get the id of the clipboard content
        let clipboard_id = view.items[&stack_id].children[0];

        // User updates the item "Hello" to "Hello World"
        let update_packet = Packet::Update(Update {
            id: clipboard_id,
            source_id: stack_id,
            hash: Some(ssri::Integrity::from("Hello World")),
            stack_id: Some(stack_id),
            source: None,
        });
        view.merge(update_packet);

        // Check that the clipboard content has been updated
        assert_eq!(
            view.items[&clipboard_id].hash,
            ssri::Integrity::from("Hello World")
        );
    }
}
