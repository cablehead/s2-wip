mod store;
mod view;

pub use crate::store::Store;
pub use crate::view::View;

#[cfg(test)]
mod tests {
    use ssri::Integrity;

    use crate::store::{MimeType, Store};
    use crate::view::View;

    fn assert_view_as_expected(view: &View, expected: Vec<(&str, Vec<&str>)>) {
        let mut mapped_expected: Vec<(Integrity, Vec<Integrity>)> = expected
            .iter()
            .map(|(stack, items)| {
                (
                    Integrity::from(stack),
                    items
                        .into_iter()
                        .map(|item| Integrity::from(item))
                        .collect(),
                )
            })
            .collect();

        let mut view: Vec<(Integrity, Vec<Integrity>)> = view
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
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let mut store = Store::new(path);
        let mut view = View::new();

        let stack_id = store.add(b"Stack 1", MimeType::TextPlain, None, None).id();
        let item_id = store
            .add(b"Item 1", MimeType::TextPlain, Some(stack_id), None)
            .id();
        // User updates the item
        store.update(
            item_id,
            Some(b"Item 1 - updated"),
            MimeType::TextPlain,
            None,
            None,
        );

        store.scan().for_each(|p| view.merge(p));
        assert_view_as_expected(&view, vec![("Stack 1", vec!["Item 1 - updated"])]);
    }

    #[test]
    fn test_fork_item() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let mut store = Store::new(path);
        let mut view = View::new();

        let stack_id = store.add(b"Stack 1", MimeType::TextPlain, None, None).id();
        let item_id = store
            .add(b"Item 1", MimeType::TextPlain, Some(stack_id), None)
            .id();

        // User forks the original item
        store.fork(
            item_id,
            Some(b"Item 1 - forked"),
            MimeType::TextPlain,
            Some(stack_id),
            None,
        );

        store.scan().for_each(|p| view.merge(p));
        assert_view_as_expected(&view, vec![("Stack 1", vec!["Item 1", "Item 1 - forked"])]);
    }

    #[test]
    fn test_move_item_to_new_stack() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let mut store = Store::new(path);
        let mut view = View::new();

        let stack_id = store.add(b"Stack 1", MimeType::TextPlain, None, None).id();
        let item_id = store
            .add(b"Item 1", MimeType::TextPlain, Some(stack_id), None)
            .id();

        // User creates a new Stack "Stack 2"
        let stack_id_2 = store.add(b"Stack 2", MimeType::TextPlain, None, None).id();

        // User moves the original item to "Stack 2"
        store.update(item_id, None, MimeType::TextPlain, Some(stack_id_2), None);

        store.scan().for_each(|p| view.merge(p));
        assert_view_as_expected(
            &view,
            vec![("Stack 1", vec![]), ("Stack 2", vec!["Item 1"])],
        );
    }

    #[test]
    fn test_delete_item() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let mut store = Store::new(path);
        let mut view = View::new();

        let stack_id = store.add(b"Stack 1", MimeType::TextPlain, None, None).id();
        let item_id_1 = store
            .add(b"Item 1", MimeType::TextPlain, Some(stack_id), None)
            .id();
        let _item_id_2 = store
            .add(b"Item 2", MimeType::TextPlain, Some(stack_id), None)
            .id();

        // User deletes the first item
        store.delete(item_id_1);

        store.scan().for_each(|p| view.merge(p));
        assert_view_as_expected(&view, vec![("Stack 1", vec!["Item 2"])]);
    }

    #[test]
    fn test_fork_stack() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let mut store = Store::new(path);
        let mut view = View::new();

        let stack_id = store.add(b"Stack 1", MimeType::TextPlain, None, None).id();
        let item_id_1 = store
            .add(b"Item 1", MimeType::TextPlain, Some(stack_id), None)
            .id();
        let item_id_2 = store
            .add(b"Item 2", MimeType::TextPlain, Some(stack_id), None)
            .id();

        // User forks the stack
        let new_stack_id = store
            .fork(stack_id, Some(b"Stack 2"), MimeType::TextPlain, None, None)
            .id();

        // User forks the items to the new stack
        store.fork(
            item_id_1,
            None,
            MimeType::TextPlain,
            Some(new_stack_id),
            None,
        );
        store.fork(
            item_id_2,
            None,
            MimeType::TextPlain,
            Some(new_stack_id),
            None,
        );

        store.scan().for_each(|p| view.merge(p));
        /*
        assert_view_as_expected(
            &view,
            vec![
                ("Stack 1", vec!["Item 1", "Item 2"]),
                ("Stack 2", vec!["Item 1", "Item 2"]),
            ],
        );
        */
    }
}
