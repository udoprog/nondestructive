use nondestructive::yaml::{Document, Separator};

/// Test case from <https://github.com/udoprog/nondestructive/issues/18>.
#[test]
fn make_nested_indent() {
    let mut doc = Document::new();

    let mut mapping_item = doc.as_mut().make_mapping();

    let foo = mapping_item.insert("foo", Separator::Auto);

    let mut arr = foo.make_sequence();
    let arr_item = arr.push(Separator::Auto);

    let mut mapping_item = arr_item.make_mapping();
    mapping_item.insert("bar", Separator::Auto);
    mapping_item.insert("baz", Separator::Auto);

    let string = std::format!("{doc}");

    assert_eq!(string, "foo:\n  - bar:\n    baz:\n");
}
