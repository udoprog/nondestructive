use crate::yaml::{self, ErrorKind};

#[track_caller]
fn unterminated(input: &str, span: &str) {
    let error = yaml::from_slice(input).unwrap_err();
    assert_eq!(*error.kind(), ErrorKind::UnterminatedString, "{input:?}");
    assert_eq!(&input[error.span()], span, "{input:?}");
}

#[test]
fn single_quoted() {
    unterminated("'", "'");
    unterminated("'abc", "'abc");
    unterminated("['1", "'1");
    unterminated("a: 'it''s", "'it''s");
    unterminated("'a''b", "'a''b");
    unterminated("'a''", "'a''");
    unterminated("{'a': 'b", "'b");
    unterminated("- 'a\n- b\n", "'a\n- b\n");
}

#[test]
fn double_quoted() {
    unterminated("\"", "\"");
    unterminated("\"abc", "\"abc");
    unterminated("a: \"abc\n", "\"abc\n");
    unterminated("[\"1", "\"1");
    unterminated("\"a\\nb", "\"a\\nb");
}

#[test]
fn nul_is_not_end_of_input() {
    unterminated("'a\0b", "'a\0b");
    unterminated("\"a\0b", "\"a\0b");

    let doc = yaml::from_slice("'a\0b'").unwrap();
    assert_eq!(doc.as_ref().as_str(), Some("a\0b"));
    let doc = yaml::from_slice("\"a\0b\"").unwrap();
    assert_eq!(doc.as_ref().as_str(), Some("a\0b"));
}

#[test]
fn terminated() {
    let doc = yaml::from_slice("'it''s'").unwrap();
    assert_eq!(doc.as_ref().as_str(), Some("it's"));
    let doc = yaml::from_slice("\"a\\tb\"").unwrap();
    assert_eq!(doc.as_ref().as_str(), Some("a\tb"));
}
