use regex::Regex;
use std::collections::HashSet;

pub fn normalize_session_name(name: &str) -> String {
    name.replace('.', "_")
}

pub fn find_matching_keys(pattern: &str, candidates: &[String]) -> Vec<String> {
    let normalized_pattern = normalize_session_name(pattern);

    if pattern.contains('*') {
        let regex = glob_to_regex(&normalized_pattern);
        return candidates
            .iter()
            .filter(|candidate| regex.is_match(&normalize_session_name(candidate)))
            .cloned()
            .collect();
    }

    let normalized_candidates: Vec<String> =
        candidates.iter().map(|c| normalize_session_name(c)).collect();

    if let Some(index) = find_best_match_index(&normalized_pattern, &normalized_candidates) {
        return vec![candidates[index].clone()];
    }

    Vec::new()
}

pub fn resolve_entry_keys(patterns: &[String], candidates: &[String]) -> (Vec<String>, Vec<String>) {
    let mut matched = Vec::new();
    let mut unmatched = Vec::new();
    let mut seen = HashSet::new();

    for pattern in patterns {
        let keys = find_matching_keys(pattern, candidates);

        if keys.is_empty() {
            unmatched.push(pattern.clone());
            continue;
        }

        for key in keys {
            if seen.insert(key.clone()) {
                matched.push(key);
            }
        }
    }

    (matched, unmatched)
}

fn glob_to_regex(glob: &str) -> Regex {
    let mut escaped = String::with_capacity(glob.len() * 2);

    for ch in glob.chars() {
        match ch {
            '*' => escaped.push_str(".*"),
            '?' => escaped.push('.'),
            c if ".+^${}()|[]\\".contains(c) => {
                escaped.push('\\');
                escaped.push(c);
            }
            c => escaped.push(c),
        }
    }

    Regex::new(&format!("(?i)^{escaped}$")).expect("valid glob regex")
}

fn find_best_match_index(pattern: &str, candidates: &[String]) -> Option<usize> {
    let mut best_index: Option<usize> = None;
    let mut best_score = 0;

    for (index, candidate) in candidates.iter().enumerate() {
        let score = fuzzy_match(pattern, candidate);
        if score > best_score {
            best_score = score;
            best_index = Some(index);
        }
    }

    best_index
}

fn fuzzy_match(pattern: &str, candidate: &str) -> i64 {
    let pattern_lower = pattern.to_lowercase();
    let candidate_lower = candidate.to_lowercase();

    if pattern_lower == candidate_lower {
        return i64::MAX;
    }

    if candidate_lower.starts_with(&pattern_lower) {
        return 1000 - (candidate_lower.len() as i64 - pattern_lower.len() as i64);
    }

    if candidate_lower.contains(&pattern_lower) {
        return 500 - (candidate_lower.len() as i64 - pattern_lower.len() as i64);
    }

    let mut score = 0i64;
    let mut pattern_index = 0usize;
    let mut consecutive = 0i64;
    let pattern_chars: Vec<char> = pattern_lower.chars().collect();

    for ch in candidate_lower.chars() {
        if pattern_index < pattern_chars.len() && ch == pattern_chars[pattern_index] {
            pattern_index += 1;
            consecutive += 1;
            score += consecutive * 5;
        } else {
            consecutive = 0;
        }
    }

    if pattern_index == pattern_chars.len() {
        score
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_matches_all_supriyoroy_entries() {
        let keys = vec![
            "root".into(),
            "supriyoroy.com/root".into(),
            "supriyoroy.com/web".into(),
        ];

        let matched = find_matching_keys("supriyoroy.com/*", &keys);
        assert_eq!(matched.len(), 2);
    }

    #[test]
    fn dot_and_underscore_are_equivalent() {
        let keys = vec!["supriyoroy.com/web".into()];
        assert_eq!(
            find_matching_keys("supriyoroy_com/web", &keys),
            vec!["supriyoroy.com/web".to_string()]
        );
    }
}
