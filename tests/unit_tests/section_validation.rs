use devlogger::section::validate_section_name;

// ---- valid ----

#[test]
fn accepts_single_letter() {
    assert!(validate_section_name("a").is_ok());
    assert!(validate_section_name("z").is_ok());
}

#[test]
fn accepts_simple_word() {
    assert!(validate_section_name("backend").is_ok());
}

#[test]
fn accepts_hyphenated_word() {
    assert!(validate_section_name("foo-bar").is_ok());
}

#[test]
fn accepts_multi_hyphenated_word() {
    assert!(validate_section_name("api-v-two-beta").is_ok());
}

// ---- invalid: empty / wrong chars ----

#[test]
fn rejects_empty() {
    let err = validate_section_name("").unwrap_err().to_string();
    assert!(err.contains("empty"), "unexpected error: {err}");
}

#[test]
fn rejects_bare_digit_five() {
    // The user specifically called out that a section named "5" is an error.
    let err = validate_section_name("5").unwrap_err().to_string();
    assert!(err.contains("illegal character '5'"), "unexpected error: {err}");
    assert!(err.contains("position 0"), "error should name the offending position: {err}");
}

#[test]
fn rejects_digits_in_name() {
    assert!(validate_section_name("abc123").is_err());
    assert!(validate_section_name("v2").is_err());
    assert!(validate_section_name("123").is_err());
}

#[test]
fn rejects_uppercase() {
    let err = validate_section_name("Foo").unwrap_err().to_string();
    assert!(err.contains("illegal character 'F'"), "unexpected error: {err}");
}

#[test]
fn rejects_any_uppercase() {
    assert!(validate_section_name("FOO").is_err());
    assert!(validate_section_name("fooBar").is_err());
    assert!(validate_section_name("foo-Bar").is_err());
}

#[test]
fn rejects_underscore() {
    assert!(validate_section_name("foo_bar").is_err());
}

#[test]
fn rejects_whitespace() {
    assert!(validate_section_name("foo bar").is_err());
    assert!(validate_section_name(" foo").is_err());
    assert!(validate_section_name("foo ").is_err());
}

#[test]
fn rejects_path_separators() {
    assert!(validate_section_name("foo/bar").is_err());
    assert!(validate_section_name("foo\\bar").is_err());
    assert!(validate_section_name("..").is_err());
}

#[test]
fn rejects_non_ascii_letters() {
    // Unicode lowercase letters are still rejected — strict a-z only.
    assert!(validate_section_name("café").is_err());
    assert!(validate_section_name("ñame").is_err());
}

// ---- invalid: hyphen rules ----

#[test]
fn rejects_leading_hyphen() {
    let err = validate_section_name("-foo").unwrap_err().to_string();
    assert!(err.contains("must not start with '-'"), "unexpected error: {err}");
}

#[test]
fn rejects_trailing_hyphen() {
    let err = validate_section_name("foo-").unwrap_err().to_string();
    assert!(err.contains("must not end with '-'"), "unexpected error: {err}");
}

#[test]
fn rejects_consecutive_hyphens() {
    let err = validate_section_name("foo--bar").unwrap_err().to_string();
    assert!(err.contains("consecutive hyphens"), "unexpected error: {err}");
}

#[test]
fn rejects_triple_hyphen() {
    assert!(validate_section_name("a---b").is_err());
}

#[test]
fn rejects_only_hyphen() {
    assert!(validate_section_name("-").is_err());
}

// ---- error message shape ----

#[test]
fn error_quotes_the_bad_name() {
    let err = validate_section_name("Bad Name").unwrap_err().to_string();
    assert!(err.contains("'Bad Name'"), "error should quote the name: {err}");
}
