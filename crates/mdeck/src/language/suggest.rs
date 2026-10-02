//! "Did you mean": the closest known name to a likely typo.

/// The candidate closest to `name`, when it is close enough to be a typo.
pub fn suggestion<'a>(
    name: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    candidates
        .into_iter()
        .map(|k| (edit_distance(name, k), k))
        .filter(|&(d, k)| d > 0 && d <= 2 && d < k.len() / 2 + 1)
        .min_by_key(|&(d, _)| d)
        .map(|(_, k)| k)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_names_are_suggested() {
        let known = ["design", "picture", "background"];
        assert_eq!(suggestion("desgin", known), Some("design"));
        assert_eq!(suggestion("pictur", known), Some("picture"));
        assert_eq!(suggestion("design", known), None, "exact is no typo");
        assert_eq!(suggestion("zebra", known), None);
        assert_eq!(edit_distance("kitten", "sitting"), 3);
    }
}
