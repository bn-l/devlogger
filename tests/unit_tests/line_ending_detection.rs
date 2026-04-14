use devlogger::entry::detect_line_ending;

#[test]
fn empty_string_defaults_to_lf() {
    assert_eq!(detect_line_ending(""), "\n");
}

#[test]
fn no_terminator_defaults_to_lf() {
    assert_eq!(detect_line_ending("single line with no newline"), "\n");
}

#[test]
fn pure_lf_file_detected_as_lf() {
    assert_eq!(detect_line_ending("one\ntwo\nthree\n"), "\n");
}

#[test]
fn pure_crlf_file_detected_as_crlf() {
    assert_eq!(detect_line_ending("one\r\ntwo\r\nthree\r\n"), "\r\n");
}

#[test]
fn single_trailing_lf_detected_as_lf() {
    assert_eq!(detect_line_ending("x\n"), "\n");
}

#[test]
fn single_trailing_crlf_detected_as_crlf() {
    assert_eq!(detect_line_ending("x\r\n"), "\r\n");
}

#[test]
fn first_terminator_wins_when_mixed_crlf_first() {
    assert_eq!(detect_line_ending("a\r\nb\nc\n"), "\r\n");
}

#[test]
fn first_terminator_wins_when_mixed_lf_first() {
    assert_eq!(detect_line_ending("a\nb\r\nc\r\n"), "\n");
}

#[test]
fn lone_cr_is_not_crlf() {
    // A bare \r with no following \n isn't CRLF; we fall through to the
    // next real terminator.
    assert_eq!(detect_line_ending("x\ry\n"), "\n");
}
