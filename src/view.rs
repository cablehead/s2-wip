use std::collections::HashMap;

use serde::Serialize;

use scru128::Scru128Id;

use crate::store::Packet;

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub id: Scru128Id,
    pub touched: Vec<Scru128Id>,
    pub hash: ssri::Integrity,
    pub parent: Option<Scru128Id>,
    pub children: Vec<Scru128Id>,
}

pub struct View {
    pub items: HashMap<Scru128Id, Item>,
}

impl Default for View {
    fn default() -> Self {
        Self::new()
    }
}

impl View {
    pub fn new() -> Self {
        View {
            items: HashMap::new(),
        }
    }

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
                        if let Some(new_parent) = self.items.get_mut(&new_stack_id) {
                            new_parent.children.push(fork.id);
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
