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
