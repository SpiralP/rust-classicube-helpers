use super::*;

// --------------- last_color ---------------

#[test]
fn last_color_finds_last() {
    let chars: Vec<char> = "&asome &btext".chars().collect();
    assert_eq!(last_color(&chars), Some('b'));
}

#[test]
fn last_color_single() {
    let chars: Vec<char> = "&atext".chars().collect();
    assert_eq!(last_color(&chars), Some('a'));
}

#[test]
fn last_color_none() {
    let chars: Vec<char> = "no color here".chars().collect();
    assert_eq!(last_color(&chars), None);
}

#[test]
fn last_color_invalid_code_skipped() {
    // '&' followed by non-hex is not a color code
    let chars: Vec<char> = "&zfoo".chars().collect();
    assert_eq!(last_color(&chars), None);
}

#[test]
fn last_color_trailing_code() {
    let chars: Vec<char> = "text&a".chars().collect();
    assert_eq!(last_color(&chars), Some('a'));
}

// --------------- wordwrap ---------------

#[test]
fn empty_input() {
    assert_eq!(wordwrap("", 10), Vec::<String>::new());
}

#[test]
fn short_string_unchanged() {
    assert_eq!(wordwrap("hello", 10), vec!["hello".to_string()]);
}

#[test]
fn exact_limit_single_line() {
    let s: String = "a".repeat(10);
    assert_eq!(wordwrap(&s, 10), vec![s]);
}

#[test]
fn hard_cut_at_limit() {
    let s: String = "a".repeat(200);
    let result = wordwrap(&s, 96);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].len(), 96);
    assert!(result[1].starts_with("> "));
    assert!(result[2].starts_with("> "));
}

#[test]
fn splits_at_last_space_in_window() {
    // "aaaa bbbb cccc", limit=10: line 0 = "aaaa bbbb ", line 1 = "> cccc"
    let result = wordwrap("aaaa bbbb cccc", 10);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], "aaaa bbbb ");
    assert_eq!(result[1], "> cccc");
}

#[test]
fn continuation_always_has_gt_prefix() {
    let s = "x".repeat(12);
    let result = wordwrap(&s, 10);
    assert_eq!(result.len(), 2);
    assert!(result[1].starts_with("> "), "got: {}", result[1]);
}

#[test]
fn color_carried_to_continuation() {
    // "&a" + 12 x's, limit=10 -> continuation is "> &a<rest>"
    let s = format!("&a{}", "x".repeat(12));
    let result = wordwrap(&s, 10);
    assert_eq!(result.len(), 2);
    assert!(
        result[1].starts_with("> &a"),
        "expected '> &a' prefix, got: {}",
        result[1]
    );
}

#[test]
fn white_color_not_carried() {
    // "&f" is default white; continuation should be "> <text>" with no extra color code
    let s = format!("&f{}", "x".repeat(12));
    let result = wordwrap(&s, 10);
    assert_eq!(result.len(), 2);
    assert_eq!(&result[1][..2], "> ");
    assert!(
        !result[1].starts_with("> &"),
        "expected no extra color, got: {}",
        result[1]
    );
}

#[test]
fn no_color_continuation_prefix_only() {
    let s = "x".repeat(12);
    let result = wordwrap(&s, 10);
    assert_eq!(result.len(), 2);
    assert_eq!(result[1], format!("> {}", "x".repeat(2)));
}

#[test]
fn carry_persists_across_multiple_lines() {
    // "&a" + 30 x's, limit=10 -> all continuations are "> &a<text>"
    let s = format!("&a{}", "x".repeat(30));
    let result = wordwrap(&s, 10);
    assert!(result.len() >= 3);
    for line in &result[1..] {
        assert!(line.starts_with("> &a"), "expected '> &a' on: {line}");
    }
}
