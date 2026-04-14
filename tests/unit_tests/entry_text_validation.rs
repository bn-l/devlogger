use devlogger::entry::validate_entry_text;

#[test]
fn accepts_empty_text() {
    assert!(validate_entry_text("").is_ok());
}

#[test]
fn accepts_plain_ascii() {
    assert!(validate_entry_text("a normal entry").is_ok());
}

#[test]
fn accepts_unicode() {
    assert!(validate_entry_text("café — naïve résumé 漢字 🙂").is_ok());
}

#[test]
fn accepts_tab() {
    // Tab is the one exception in the control-char range.
    assert!(validate_entry_text("col1\tcol2\tcol3").is_ok());
}

#[test]
fn accepts_punctuation_and_symbols() {
    assert!(validate_entry_text("deploy: staging | !@#$%^&*()[]{}<>").is_ok());
}

#[test]
fn rejects_newline_lf() {
    let err = validate_entry_text("line1\nline2").unwrap_err().to_string();
    assert!(err.contains("newline"), "unexpected: {err}");
    assert!(err.contains("byte 5"), "should mention byte offset: {err}");
}

#[test]
fn rejects_newline_at_start() {
    let err = validate_entry_text("\nfoo").unwrap_err().to_string();
    assert!(err.contains("newline"));
    assert!(err.contains("byte 0"));
}

#[test]
fn rejects_newline_at_end() {
    assert!(validate_entry_text("foo\n").is_err());
}

#[test]
fn rejects_carriage_return() {
    let err = validate_entry_text("line1\rline2").unwrap_err().to_string();
    assert!(err.contains("carriage return"), "unexpected: {err}");
}

#[test]
fn rejects_crlf() {
    let err = validate_entry_text("a\r\nb").unwrap_err().to_string();
    // Either byte of the CRLF can trigger first; both are rejections.
    assert!(err.contains("carriage return") || err.contains("newline"));
}

#[test]
fn rejects_null_byte() {
    let err = validate_entry_text("a\0b").unwrap_err().to_string();
    assert!(err.contains("U+0000"), "should mention codepoint: {err}");
}

#[test]
fn rejects_control_characters() {
    for code in [0x01u32, 0x07, 0x08, 0x0B, 0x0C, 0x1B, 0x7F] {
        let s = format!("pre{}post", char::from_u32(code).unwrap());
        assert!(
            validate_entry_text(&s).is_err(),
            "U+{code:04X} should be rejected"
        );
    }
}
