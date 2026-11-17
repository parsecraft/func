//! "Did you mean ...?" suggestions for mistyped names.

/// Edit distance between two strings (insert, delete, substitute).
pub fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();

    for (i, ca) in a.chars().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(ca != *cb);
            let deletion = previous[j + 1] + 1;
            let insertion = current[j] + 1;
            current.push(substitution.min(deletion).min(insertion));
        }
        previous = current;
    }
    previous[b.len()]
}

/// Up to three candidates that look like what the person meant to type,
/// best match first. Prefixes count as close (`hel` -> `hello`).
pub fn closest<'a>(input: &str, candidates: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
    let length = input.chars().count();
    let limit = if length <= 2 { 1 } else { (length / 3).max(2) };

    let mut scored: Vec<(usize, &str)> = candidates
        .into_iter()
        .filter(|candidate| *candidate != input)
        .filter_map(|candidate| {
            let is_prefix = length >= 3 && candidate.starts_with(input);
            let score = if is_prefix { 0 } else { distance(input, candidate) };
            (score <= limit).then_some((score, candidate))
        })
        .collect();

    scored.sort();
    scored.dedup();
    scored.into_iter().take(3).map(|(_, name)| name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_counts_edits() {
        assert_eq!(distance("kitten", "sitting"), 3);
        assert_eq!(distance("", "abc"), 3);
        assert_eq!(distance("same", "same"), 0);
    }

    #[test]
    fn finds_typos() {
        let names = ["hello", "world", "list"];
        assert_eq!(closest("helo", names), ["hello"]);
        assert_eq!(closest("lst", names), ["list"]);
    }

    #[test]
    fn prefixes_are_close() {
        assert_eq!(closest("hel", ["hello-world", "other"]), ["hello-world"]);
    }

    #[test]
    fn nothing_close_means_nothing() {
        assert!(closest("zzzzzz", ["hello", "world"]).is_empty());
    }

    #[test]
    fn never_suggests_the_input_itself() {
        assert!(closest("hello", ["hello"]).is_empty());
    }
}
