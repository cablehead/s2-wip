mod store;
mod view;

#[cfg(test)]
mod tests {
    use crate::store::{AddPacket, DeletePacket, ForkPacket, Packet, UpdatePacket};
    use crate::view::View;
    use std::collections::HashMap;

    fn assert_view_as_expected(view: &View, expected: Vec<(&str, Vec<&str>)>) {
        let mut mapped_expected: Vec<(ssri::Integrity, Vec<ssri::Integrity>)> = expected
            .iter()
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

        let mut view: Vec<(ssri::Integrity, Vec<ssri::Integrity>)> = view
            .root()
            .iter()
            .map(|item| {
                let children_hashes = item
                    .children
                    .iter()
                    .filter_map(|id| view.items.get(id))
                    .map(|child| child.hash.clone())
                    .collect();
                (item.hash.clone(), children_hashes)
            })
            .collect();

        // Sort the vectors before comparing
        mapped_expected.sort_by(|a, b| a.0.to_string().cmp(&b.0.to_string()));
        for (_, v) in &mut mapped_expected {
            v.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
        }
        view.sort_by(|a, b| a.0.to_string().cmp(&b.0.to_string()));
        for (_, v) in &mut view {
            v.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
        }

        assert_eq!(view, mapped_expected, "\n\nExpected: {:?}\n", &expected);
    }

    #[test]
    fn test_update_item() {
        let mut view = View {
            items: HashMap::new(),
        };

        let stack_id = scru128::new();
        view.merge(Packet::Add(AddPacket {
            id: stack_id,
            hash: ssri::Integrity::from("Stack 1"),
            stack_id: None,
            source: None,
        }));
        let item_id = scru128::new();
        view.merge(Packet::Add(AddPacket {
            id: item_id,
            hash: ssri::Integrity::from("Item 1"),
            stack_id: Some(stack_id),
            source: None,
        }));

        // User updates the item
        view.merge(Packet::Update(UpdatePacket {
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
        view.merge(Packet::Add(AddPacket {
            id: stack_id,
            hash: ssri::Integrity::from("Stack 1"),
            stack_id: None,
            source: None,
        }));
        let item_id = scru128::new();
        view.merge(Packet::Add(AddPacket {
            id: item_id,
            hash: ssri::Integrity::from("Item 1"),
            stack_id: Some(stack_id),
            source: None,
        }));

        // User forks the original item
        view.merge(Packet::Fork(ForkPacket {
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
        view.merge(Packet::Add(AddPacket {
            id: stack_id,
            hash: ssri::Integrity::from("Stack 1"),
            stack_id: None,
            source: None,
        }));
        let item_id = scru128::new();
        view.merge(Packet::Add(AddPacket {
            id: item_id,
            hash: ssri::Integrity::from("Item 1"),
            stack_id: Some(stack_id),
            source: None,
        }));

        // User creates a new Stack "Stack 2"
        let stack_id_2 = scru128::new();
        view.merge(Packet::Add(AddPacket {
            id: stack_id_2,
            hash: ssri::Integrity::from("Stack 2"),
            stack_id: None,
            source: None,
        }));

        // User moves the original item to "Stack 2"
        view.merge(Packet::Update(UpdatePacket {
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

    #[test]
    fn test_delete_item() {
        let mut view = View {
            items: HashMap::new(),
        };

        let stack_id = scru128::new();
        view.merge(Packet::Add(AddPacket {
            id: stack_id,
            hash: ssri::Integrity::from("Stack 1"),
            stack_id: None,
            source: None,
        }));
        let item_id_1 = scru128::new();
        view.merge(Packet::Add(AddPacket {
            id: item_id_1,
            hash: ssri::Integrity::from("Item 1"),
            stack_id: Some(stack_id),
            source: None,
        }));
        let item_id_2 = scru128::new();
        view.merge(Packet::Add(AddPacket {
            id: item_id_2,
            hash: ssri::Integrity::from("Item 2"),
            stack_id: Some(stack_id),
            source: None,
        }));

        // User deletes the first item
        view.merge(Packet::Delete(DeletePacket {
            id: scru128::new(),
            source_id: item_id_1,
        }));

        assert_view_as_expected(&view, vec![("Stack 1", vec!["Item 2"])]);
    }

    #[test]
    fn test_fork_stack() {
        let mut view = View {
            items: HashMap::new(),
        };

        let stack_id = scru128::new();
        view.merge(Packet::Add(AddPacket {
            id: stack_id,
            hash: ssri::Integrity::from("Stack 1"),
            stack_id: None,
            source: None,
        }));
        let item_id_1 = scru128::new();
        view.merge(Packet::Add(AddPacket {
            id: item_id_1,
            hash: ssri::Integrity::from("Item 1"),
            stack_id: Some(stack_id),
            source: None,
        }));
        let item_id_2 = scru128::new();
        view.merge(Packet::Add(AddPacket {
            id: item_id_2,
            hash: ssri::Integrity::from("Item 2"),
            stack_id: Some(stack_id),
            source: None,
        }));

        // User forks the stack
        let new_stack_id = scru128::new();
        view.merge(Packet::Fork(ForkPacket {
            id: new_stack_id,
            source_id: stack_id,
            hash: Some(ssri::Integrity::from("Stack 2")),
            stack_id: None,
            source: None,
        }));

        // User forks the items to the new stack
        let new_item_id_1 = scru128::new();
        view.merge(Packet::Fork(ForkPacket {
            id: new_item_id_1,
            source_id: item_id_1,
            hash: None,
            stack_id: Some(new_stack_id),
            source: None,
        }));
        let new_item_id_2 = scru128::new();
        view.merge(Packet::Fork(ForkPacket {
            id: new_item_id_2,
            source_id: item_id_2,
            hash: None,
            stack_id: Some(new_stack_id),
            source: None,
        }));

        assert_view_as_expected(
            &view,
            vec![
                ("Stack 1", vec!["Item 1", "Item 2"]),
                ("Stack 2", vec!["Item 1", "Item 2"]),
            ],
        );
    }
}
