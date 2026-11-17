use std::fmt::Write as _;

/// Emit one YAML scalar with the same rules for every hand-rolled emitter.
///
/// Plain scalars are intentionally conservative: `:` is allowed only where YAML
/// cannot treat it as a mapping delimiter (`http://example` is plain, `key: value`
/// and `label:` are quoted), and a serde_yaml probe must parse the candidate back
/// as the same string. Everything else is quoted losslessly.
pub fn scalar(value: &str) -> String {
    if can_emit_plain_scalar(value) {
        value.to_owned()
    } else if needs_double_quoted_scalar(value) {
        double_quoted_scalar(value)
    } else {
        single_quoted_scalar(value)
    }
}

/// Emit one YAML mapping key.
///
/// Quoted scalars can represent most hostile key text, but physical line-break
/// characters make single-line hand-rolled mappings too easy to corrupt. Callers
/// with fallible parse/validation paths should reject those keys before emit.
pub fn key(value: &str) -> Option<String> {
    is_emittable_key(value).then(|| scalar(value))
}

/// Return whether a key can be emitted by the hand-rolled single-line mapping writers.
pub fn is_emittable_key(value: &str) -> bool {
    !value.contains(['\n', '\r'])
}

/// Write `key: value` using a block scalar for multiline values and `scalar` otherwise.
///
/// An explicit `2` indentation indicator is added when YAML auto-detection would
/// corrupt a value whose first content line starts with whitespace. Chomp
/// indicators preserve the exact trailing-newline shape. Block scalars are used
/// only for text that YAML will not reinterpret as line separators or controls;
/// unsafe multiline text falls back to double-quoted escapes.
pub fn write_multiline_or_scalar(out: &mut String, indent: &str, key: &str, value: &str) {
    let key = self::key(key).expect("emitted YAML key was not prevalidated");
    if can_emit_block_scalar(value) {
        let chomp = match trailing_newline_count(value) {
            0 => "-",
            1 => "",
            _ => "+",
        };
        let indentation = if needs_explicit_block_indent(value) {
            "2"
        } else {
            ""
        };
        writeln!(out, "{indent}{key}: |{indentation}{chomp}")
            .expect("writing to a string cannot fail");
        for line in block_content_lines(value) {
            writeln!(out, "{indent}  {line}").expect("writing to a string cannot fail");
        }
    } else {
        writeln!(out, "{indent}{key}: {}", scalar(value)).expect("writing to a string cannot fail");
    }
}

fn can_emit_plain_scalar(value: &str) -> bool {
    has_plain_scalar_shape(value) && parses_as_same_string(value)
}

fn has_plain_scalar_shape(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with([
            ' ', '-', '?', ':', '@', '`', '&', '*', '!', '|', '>', '#', '{', '[', ',', '\t',
        ])
        && !value.ends_with([' ', '\t', ':'])
        && !contains_colon_mapping_indicator(value)
        && value.chars().all(is_allowed_plain_char)
}

fn is_allowed_plain_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '.' | ',' | '_' | '-' | '/' | ':')
}

fn contains_colon_mapping_indicator(value: &str) -> bool {
    value
        .as_bytes()
        .windows(2)
        .any(|window| window[0] == b':' && matches!(window[1], b' ' | b'\t'))
}

fn parses_as_same_string(value: &str) -> bool {
    !may_resolve_to_non_string(value) || probe_parses_as_same_string(value)
}

/// serde_yaml resolves an untagged plain scalar to a non-string only when it is
/// a null or boolean keyword or starts like a number. Any other plain-shaped
/// value reads back as itself, so only these candidates need the parser probe.
fn may_resolve_to_non_string(value: &str) -> bool {
    value.starts_with(|ch: char| ch.is_ascii_digit() || matches!(ch, '.' | '+' | '-'))
        || matches!(
            value,
            "~" | "null" | "Null" | "NULL" | "true" | "True" | "TRUE" | "false" | "False" | "FALSE"
        )
}

fn probe_parses_as_same_string(value: &str) -> bool {
    let input = format!("value: {value}\n");
    let Ok(serde_yaml::Value::Mapping(mapping)) = serde_yaml::from_str::<serde_yaml::Value>(&input)
    else {
        return false;
    };
    mapping
        .get(serde_yaml::Value::String("value".to_owned()))
        .is_some_and(|parsed| parsed == &serde_yaml::Value::String(value.to_owned()))
}

fn needs_double_quoted_scalar(value: &str) -> bool {
    value.chars().any(|ch| {
        matches!(
            ch,
            '\0'..='\u{1f}' | '\u{7f}' | '\u{85}' | '\u{2028}' | '\u{2029}'
        )
    })
}

fn single_quoted_scalar(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn double_quoted_scalar(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\0' => out.push_str("\\0"),
            '\u{07}' => out.push_str("\\a"),
            '\u{08}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{0b}' => out.push_str("\\v"),
            '\u{0c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            '\u{1b}' => out.push_str("\\e"),
            '\u{85}' => out.push_str("\\N"),
            '\u{2028}' => out.push_str("\\L"),
            '\u{2029}' => out.push_str("\\P"),
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{01}'..='\u{06}' | '\u{0e}'..='\u{1a}' | '\u{1c}'..='\u{1f}' | '\u{7f}' => {
                write!(out, "\\x{:02X}", ch as u32).expect("writing to a string cannot fail");
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn can_emit_block_scalar(value: &str) -> bool {
    value.contains('\n') && !value.starts_with('\n') && value.chars().all(is_block_safe_char)
}

fn is_block_safe_char(ch: char) -> bool {
    ch == '\n'
        || !matches!(
            ch,
            '\0'..='\u{1f}' | '\u{7f}' | '\u{85}' | '\u{2028}' | '\u{2029}'
        )
}

fn needs_explicit_block_indent(value: &str) -> bool {
    value.starts_with([' ', '\t'])
}

fn trailing_newline_count(value: &str) -> usize {
    value
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'\n')
        .count()
}

fn block_content_lines(value: &str) -> Vec<&str> {
    let mut lines = value.split('\n').collect::<Vec<_>>();
    if value.ends_with('\n') {
        lines.pop();
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every plain-shaped value the probe would accept without help must be one
    /// the keyword/number screen lets through unprobed, and vice versa.
    #[test]
    fn unprobed_plain_scalars_always_read_back_as_the_same_string() {
        let alphabet = [
            'a', 'n', 'e', 'x', 'Z', '0', '7', '.', ' ', ',', '_', '-', '/', ':',
        ];
        let mut corpus = vec![String::new()];
        let mut frontier = vec![String::new()];
        for _ in 0..4 {
            frontier = frontier
                .iter()
                .flat_map(|prefix| alphabet.iter().map(move |ch| format!("{prefix}{ch}")))
                .collect();
            corpus.extend(frontier.iter().cloned());
        }
        corpus.extend(
            [
                "null", "Null", "NULL", "true", "True", "TRUE", "false", "False", "FALSE", "yes",
                "no", "on", "off", "inf", "Infinity", "nan", "NaN", "e5", "x1", "y", "n",
                "infinity", "nul", "truex", "Ture", "ok/0x1F", "a.inf",
            ]
            .map(str::to_owned),
        );
        let mut unprobed = 0;
        for value in corpus.iter().filter(|value| has_plain_scalar_shape(value)) {
            if !may_resolve_to_non_string(value) {
                unprobed += 1;
                assert!(
                    probe_parses_as_same_string(value),
                    "{value:?} skipped the probe but does not read back as itself"
                );
            }
        }
        assert!(
            unprobed > 1_000,
            "corpus exercised only {unprobed} unprobed values"
        );
    }

    #[test]
    fn ambiguous_plain_scalars_are_still_quoted() {
        for value in ["1", "0x1F", ".5", ".inf", "+1", "1e5", "null", "True", "~"] {
            assert_ne!(scalar(value), value, "{value:?} must not be emitted plain");
        }
        for value in ["Infinity", "nan", "yes", "plain text", "http://x"] {
            assert_eq!(scalar(value), value, "{value:?} should stay plain");
        }
    }
}
