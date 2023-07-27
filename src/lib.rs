mod store;
mod view;

pub use crate::store::Store;
pub use crate::view::View;

#[cfg(test)]
mod tests {
    use crate::store::{MimeType, Store};
    use crate::view::View;

    fn assert_view_as_expected(store: &Store, view: &View, expected: Vec<(&str, Vec<&str>)>) {
        let mut actual: Vec<(String, Vec<String>)> = view
            .root()
            .iter()
            .filter_map(|item| {
                let children = item
                    .get_children()
                    .iter()
                    .filter_map(|child_id| {
                        view.items
                            .get(child_id)
                            .and_then(|child_item| store.cas_read(&child_item.hash))
                            .map(|content| String::from_utf8_lossy(&content).into_owned())
                    })
                    .collect::<Vec<_>>();
                store
                    .cas_read(&item.hash)
                    .map(|s| (String::from_utf8_lossy(&s).into_owned(), children))
            })
            .collect();

        // Sort the vectors before comparing
        actual.sort_by(|a, b| a.0.cmp(&b.0));
        for (_, v) in &mut actual {
            v.sort();
        }

        let mut expected: Vec<(String, Vec<String>)> = expected
            .into_iter()
            .map(|(s, children)| {
                (
                    s.to_string(),
                    children.into_iter().map(|c| c.to_string()).collect(),
                )
            })
            .collect();

        // Sort the expected vectors before comparing
        expected.sort_by(|a, b| a.0.cmp(&b.0));
        for (_, v) in &mut expected {
            v.sort();
        }

        assert_eq!(actual, expected);
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
        assert_view_as_expected(&store, &view, vec![("Stack 1", vec!["Item 1 - updated"])]);
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
        assert_view_as_expected(
            &store,
            &view,
            vec![("Stack 1", vec!["Item 1", "Item 1 - forked"])],
        );
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
            &store,
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
        assert_view_as_expected(&store, &view, vec![("Stack 1", vec!["Item 2"])]);
    }

    #[test]
    fn test_fork_stack() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let mut store = Store::new(path);

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

        let mut view = View::new();
        store.scan().for_each(|p| view.merge(p));
        assert_view_as_expected(
            &store, &view,
            vec![
                ("Stack 1", vec!["Item 1", "Item 2"]),
                ("Stack 2", vec!["Item 1", "Item 2"]),
            ],
        );

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

        let mut view = View::new();
        store.scan().for_each(|p| view.merge(p));
        assert_view_as_expected(
            &store, &view,
            vec![
                ("Stack 1", vec!["Item 1", "Item 2"]),
                ("Stack 2", vec!["Item 1", "Item 2"]),
            ],
        );
    }
}
