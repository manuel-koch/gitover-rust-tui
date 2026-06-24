// Copyright © 2026 Manuel Koch
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::path::PathBuf;

/// Expand `${VAR}` references in `s` using `lookup` to resolve each name.
/// Unknown references are left unexpanded.
/// Returns `(expanded_string, names_that_could_not_be_resolved)`.
fn substitute(s: &str, lookup: impl Fn(&str) -> Option<String>) -> (String, Vec<String>) {
    let mut result = String::with_capacity(s.len());
    let mut missing: Vec<String> = Vec::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '$' || chars.peek() != Some(&'{') {
            result.push(c);
            continue;
        }
        chars.next(); // consume '{'
        let name: String = chars.by_ref().take_while(|&c| c != '}').collect();
        if name.is_empty() {
            result.push_str("${}");
            continue;
        }
        match lookup(&name) {
            Some(val) => result.push_str(&val),
            None => {
                missing.push(name.clone());
                result.push_str(&format!("${{{name}}}"));
            }
        }
    }

    (result, missing)
}

/// Replace `${VAR}` references in `s` using the provided key-value dict.
/// Environment variables are not consulted.
/// Returns `(expanded_string, names_that_could_not_be_resolved)`.
pub fn expand_vars(s: &str, vars: &[(&str, &str)]) -> (String, Vec<String>) {
    substitute(s, |name| {
        vars.iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.to_string())
    })
}

/// Replace `${VAR}` references in `s` using environment variables.
/// The provided key-value dict is not consulted.
/// Returns `(expanded_string, names_that_could_not_be_resolved)`.
pub fn expand_env_vars(s: &str) -> (String, Vec<String>) {
    substitute(s, |name| std::env::var(name).ok())
}

/// Truncate `text` to at most `max_chars` display characters by removing the
/// middle and replacing it with `"..."`. If the text already fits, it is
/// returned unchanged.
pub fn truncate_middle(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }
    if max_chars <= 3 {
        return "...".to_string();
    }
    let available = max_chars - 3;
    let left_count = available / 2;
    let right_count = available - left_count;
    let left: String = text.chars().take(left_count).collect();
    let right: String = text.chars().skip(char_count - right_count).collect();
    format!("{left}...{right}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── truncate_middle ───────────────────────────────────────────────────────

    #[test]
    fn truncate_middle_short_text_unchanged() {
        assert_eq!(truncate_middle("hello", 10), "hello");
    }

    #[test]
    fn truncate_middle_exact_fit_unchanged() {
        assert_eq!(truncate_middle("hello", 5), "hello");
    }

    #[test]
    fn truncate_middle_max_three_or_less_returns_ellipsis() {
        assert_eq!(truncate_middle("hello", 3), "...");
        assert_eq!(truncate_middle("hello", 1), "...");
        assert_eq!(truncate_middle("hello", 0), "...");
    }

    #[test]
    fn truncate_middle_even_split() {
        // available = 7-3 = 4, left = 2, right = 2 → "ab...gh"
        assert_eq!(truncate_middle("abcdefgh", 7), "ab...gh");
    }

    #[test]
    fn truncate_middle_odd_split_right_gets_extra() {
        // available = 6-3 = 3, left = 1, right = 2 → "a...gh"
        assert_eq!(truncate_middle("abcdefgh", 6), "a...gh");
    }

    #[test]
    fn truncate_middle_unicode_chars() {
        // "αβγδεζ" 6 chars, max=5: available=2, left=1, right=1 → "α...ζ"
        assert_eq!(truncate_middle("αβγδεζ", 5), "α...ζ");
    }

    // ── expand_vars ───────────────────────────────────────────────────────────

    #[test]
    fn expand_vars_substitutes_known_var() {
        let (result, missing) = expand_vars("Hello ${NAME}!", &[("NAME", "World")]);
        assert_eq!(result, "Hello World!");
        assert!(missing.is_empty());
    }

    #[test]
    fn expand_vars_leaves_unknown_var_unexpanded() {
        let (result, missing) = expand_vars("${UNKNOWN}", &[]);
        assert_eq!(result, "${UNKNOWN}");
        assert_eq!(missing, vec!["UNKNOWN"]);
    }

    #[test]
    fn expand_vars_multiple_vars() {
        let (result, missing) = expand_vars("${A}-${B}", &[("A", "foo"), ("B", "bar")]);
        assert_eq!(result, "foo-bar");
        assert!(missing.is_empty());
    }

    #[test]
    fn expand_vars_empty_braces_left_as_is() {
        let (result, missing) = expand_vars("${}ok", &[]);
        assert_eq!(result, "${}ok");
        assert!(missing.is_empty());
    }

    #[test]
    fn expand_vars_empty_input() {
        let (result, missing) = expand_vars("", &[]);
        assert_eq!(result, "");
        assert!(missing.is_empty());
    }

    // ── expand_env_vars ───────────────────────────────────────────────────────

    #[test]
    fn expand_env_vars_resolves_existing_var() {
        let path_val = std::env::var("PATH").unwrap_or_default();
        let (result, missing) = expand_env_vars("${PATH}");
        assert_eq!(result, path_val);
        assert!(missing.is_empty());
    }

    #[test]
    fn expand_env_vars_reports_missing_var() {
        let (result, missing) = expand_env_vars("${__GITOVER_NO_SUCH_VAR__}");
        assert_eq!(result, "${__GITOVER_NO_SUCH_VAR__}");
        assert_eq!(missing, vec!["__GITOVER_NO_SUCH_VAR__"]);
    }

    // ── expand_path ───────────────────────────────────────────────────────────

    #[test]
    fn expand_path_tilde_prefix_expands_to_home() {
        let (path, missing) = expand_path("~/projects");
        let path_str = path.to_str().unwrap();
        assert!(!path_str.starts_with('~'), "tilde must be expanded");
        assert!(path_str.ends_with("/projects"));
        assert!(missing.is_empty());
    }

    #[test]
    fn expand_path_no_tilde_passes_through() {
        let (path, missing) = expand_path("/absolute/path");
        assert_eq!(path.to_str().unwrap(), "/absolute/path");
        assert!(missing.is_empty());
    }

    #[test]
    fn expand_path_var_in_path_expanded() {
        let path_val = std::env::var("PATH").unwrap_or_default();
        let (path, missing) = expand_path("${PATH}");
        assert_eq!(path.to_str().unwrap(), path_val);
        assert!(missing.is_empty());
    }
}

/// Expand `~` (home directory) and `${VAR}` environment variable references
/// in a path string.
/// Returns `(expanded_path, names_that_could_not_be_resolved)`.
pub fn expand_path(s: &str) -> (PathBuf, Vec<String>) {
    let s = if s == "~" || s.starts_with("~/") || s.starts_with("~\\") {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{}{}", home, &s[1..])
    } else {
        s.to_string()
    };
    let (expanded, missing) = expand_env_vars(&s);
    (PathBuf::from(expanded), missing)
}
