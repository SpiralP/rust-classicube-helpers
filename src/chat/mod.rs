pub mod message_hook;

use classicube_sys::{Chat_Add, Chat_Send, OwnedString};
pub use message_hook::ProtocolMessageHook;
use tracing::info;

#[cfg(test)]
mod tests;

// TEXTGROUPWIDGET_LEN = STRING_SIZE + STRING_SIZE / 2 = 64 + 32
const TEXTGROUPWIDGET_LEN: usize = 96;

/// Returns the last `&X` color code character in `text`, scanning backward.
/// Returns `None` if there is no color code.
fn last_color(text: &[char]) -> Option<char> {
    let mut i = text.len();
    while i >= 2 {
        i -= 1;
        if text[i - 1] == '&' && text[i].is_ascii_hexdigit() {
            return Some(text[i]);
        }
    }
    None
}

/// Append soft-wrapped lines for a single `\n`-free segment to `out`.
/// Continuation lines are prefixed with `> ` (2 chars) or `> &X` (4 chars,
/// with color carry) followed by their content. Each produced line, including
/// the prefix, fits within `limit` chars. Empty `text` appends nothing.
fn wrap_line(text: &str, limit: usize, out: &mut Vec<String>) {
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;
    let mut carry: Option<char> = None;
    let mut first = true;

    while start < chars.len() {
        // Continuation lines have a "> " (2) or "> &X" (4) prefix; reserve
        // that width so the rendered line stays within `limit`. First lines
        // have no prefix and use the full limit.
        let prefix_len = if first {
            0
        } else {
            2 + if carry.is_some() { 2 } else { 0 }
        };
        let effective = limit.saturating_sub(prefix_len).max(1);

        let end = if chars.len() - start <= effective {
            chars.len()
        } else {
            let window_end = start + effective;
            match chars[start..window_end].iter().rposition(|&c| c == ' ') {
                Some(rel) if rel > 0 => start + rel + 1,
                _ => window_end,
            }
        };

        let segment: String = chars[start..end].iter().collect();
        out.push(if first {
            segment
        } else {
            match carry {
                Some(c) => format!("> &{c}{segment}"),
                None => format!("> {segment}"),
            }
        });
        first = false;

        if let Some(code) = last_color(&chars[start..end]) {
            carry = if matches!(code, 'f' | 'F') {
                None
            } else {
                Some(code)
            };
        }

        start = end;
    }
}

/// Word-wrap `text` into lines of at most `limit` characters.
///
/// The input is first split on `\n`; each resulting segment is wrapped
/// independently (fresh continuation prefix and color carry per segment).
/// Within a segment, splits occur at the last space within the limit
/// (hard-cut at the limit if there is none). Continuation lines are prefixed
/// with `> ` (2 chars) or `> &X` (4 chars, with active color carry); the
/// content of those lines is wrapped at `limit - prefix_len` so that the
/// rendered line including the prefix stays within `limit`. Empty segments
/// (from consecutive or trailing `\n`) produce no output.
#[must_use]
pub fn wordwrap(text: &str, limit: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for segment in text.split('\n') {
        wrap_line(segment, limit, &mut lines);
    }
    lines
}

pub fn print<S: Into<String>>(s: S) {
    let s: String = s.into();
    info!("{}", s);

    for line in wordwrap(&s, TEXTGROUPWIDGET_LEN) {
        let owned_string = OwnedString::new(line);
        unsafe {
            Chat_Add(owned_string.as_cc_string());
        }
    }
}

pub fn send<S: Into<String>>(s: S) {
    let s = s.into();
    info!("{}", s);

    let owned_string = OwnedString::new(s);
    unsafe {
        Chat_Send(owned_string.as_cc_string(), 0);
    }
}
