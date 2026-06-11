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

// --------------- wordwrap: continuation lines stay within limit ---------------

#[test]
fn continuation_respects_limit_with_color_carry() {
    // Reproduces the original bug: &a + a 100-char space-free token at limit=96.
    // Before the fix the continuation line was 100 chars (96 content + ">&a"
    // prefix), which ClassiCube truncated, losing 4 chars.
    let token = "x".repeat(100);
    let s = format!("&a{token}");
    let result = wordwrap(&s, 96);
    for line in &result {
        assert!(
            line.chars().count() <= 96,
            "line exceeds limit: ({} chars) {:?}",
            line.chars().count(),
            line
        );
    }
    // All input characters must be present in the output (nothing dropped).
    // Strip continuation prefixes ("> &a" or "> ") from lines 1+; line 0
    // has no prefix so unwrap_or returns it as-is (including the "&a" code).
    let total_content: usize = result
        .iter()
        .map(|l| {
            let stripped = l
                .strip_prefix("> &a")
                .or_else(|| l.strip_prefix("> "))
                .unwrap_or(l);
            stripped.chars().count()
        })
        .sum();
    assert_eq!(total_content, s.chars().count());
}

#[test]
fn continuation_no_color_respects_limit() {
    // Same invariant for the "> " (2-char) prefix path.
    let s = "x".repeat(200);
    let result = wordwrap(&s, 96);
    for line in &result {
        assert!(
            line.chars().count() <= 96,
            "line exceeds limit: ({} chars) {:?}",
            line.chars().count(),
            line
        );
    }
}

// --------------- wordwrap: \n newline handling ---------------

#[test]
fn newline_splits_into_separate_lines() {
    assert_eq!(
        wordwrap("hello\nworld", 96),
        vec!["hello".to_string(), "world".to_string()]
    );
}

#[test]
fn newline_no_continuation_prefix() {
    // A hard \n starts a fresh line, not a soft-wrap continuation.
    let result = wordwrap("hello\nworld", 96);
    assert_eq!(result.len(), 2);
    assert!(
        !result[1].starts_with("> "),
        "expected no '> ' prefix after \\n, got: {}",
        result[1]
    );
}

#[test]
fn consecutive_newlines_collapse() {
    assert_eq!(
        wordwrap("a\n\nb", 96),
        vec!["a".to_string(), "b".to_string()]
    );
}

#[test]
fn trailing_newline_collapsed() {
    assert_eq!(wordwrap("foo\n", 96), vec!["foo".to_string()]);
}

#[test]
fn newline_resets_color_carry() {
    // The &a color from the first segment must not bleed into the second.
    let result = wordwrap("&ahello\nworld", 96);
    assert_eq!(result, vec!["&ahello".to_string(), "world".to_string()]);
}

#[test]
fn newline_then_long_line_wraps() {
    // The segment after \n is still soft-wrapped with "> " within itself.
    let s = format!("short\n{}", "x".repeat(12));
    let result = wordwrap(&s, 10);
    // "short" + hard-cut 10 x's + "> " + 2 x's
    assert_eq!(result.len(), 3, "got: {result:?}");
    assert_eq!(result[0], "short");
    assert_eq!(result[1], "x".repeat(10));
    assert_eq!(result[2], format!("> {}", "x".repeat(2)));
}
