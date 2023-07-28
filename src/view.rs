use std::collections::HashMap;

use serde::Serialize;

use scru128::Scru128Id;
use ssri::Integrity;

use crate::store::Packet;

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub id: Scru128Id,
    pub last_touched: Scru128Id,
    pub touched: Vec<Scru128Id>,
    pub hash: Integrity,
    pub stack_id: Option<Scru128Id>,
    pub children: Vec<Scru128Id>,
    pub forked_children: Vec<Scru128Id>,
}

impl Item {
    pub fn get_children(&self) -> Vec<Scru128Id> {
        let mut children = self.children.clone();
        children.extend(&self.forked_children);
        children
    }
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
            Packet::Add(packet) => {
                let item = Item {
                    id: packet.id,
                    last_touched: packet.id,
                    touched: vec![packet.id],
                    hash: packet.hash,
                    stack_id: packet.stack_id,
                    children: Vec::new(),
                    forked_children: Vec::new(),
                };

                if let Some(stack_id) = packet.stack_id {
                    if let Some(stack) = self.items.get_mut(&stack_id) {
                        stack.children.push(packet.id);
                    }
                }
                self.items.insert(packet.id, item);
            }
            Packet::Update(update) => {
                if let Some(item) = self.items.get(&update.source_id).cloned() {
                    let mut item = item;
                    item.touched.push(update.id);
                    if let Some(new_hash) = update.hash {
                        item.hash = new_hash;
                    }
                    if let Some(new_stack_id) = update.stack_id {
                        if let Some(old_stack_id) = item.stack_id {
                            if let Some(old_stack) = self.items.get_mut(&old_stack_id) {
                                old_stack.children.retain(|&id| id != update.source_id);
                            }
                        }
                        item.stack_id = Some(new_stack_id);
                        if let Some(new_stack) = self.items.get_mut(&new_stack_id) {
                            new_stack.children.push(update.source_id);
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

                    new_item.forked_children = item.children.clone();
                    new_item.children = Vec::new();

                    if let Some(new_hash) = fork.hash {
                        new_item.hash = new_hash;
                    }

                    if let Some(new_stack_id) = fork.stack_id {
                        new_item.stack_id = Some(new_stack_id);
                    }

                    if let Some(stack_id) = new_item.stack_id {
                        if let Some(new_stack) = self.items.get_mut(&stack_id) {
                            // Remove the forked item from forked_children
                            new_stack.forked_children.retain(|&id| id != fork.source_id);
                            // And add the new item to children
                            new_stack.children.push(fork.id);
                        }
                    }

                    self.items.insert(fork.id, new_item);
                }
            }
            Packet::Delete(delete) => {
                if let Some(item) = self.items.remove(&delete.source_id) {
                    if let Some(stack_id) = item.stack_id {
                        if let Some(stack) = self.items.get_mut(&stack_id) {
                            stack.children.retain(|&id| id != delete.source_id);
                        }
                    }
                }
            }
        }
    }

    pub fn root(&self) -> Vec<Item> {
        self.items
            .values()
            .filter(|item| item.stack_id.is_none())
            .cloned()
            .collect()
    }
}
