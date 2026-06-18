/// Result of a trie-style object match.
pub enum TrieMatch<T> {
    None,
    One(T),
    /// Only returned for `all` / `all.X` queries.
    Many(Vec<T>),
}

/// Trie-style MUD object matching over `(name, item)` pairs.
///
/// Input syntax:
/// - `"gob"`      — first item whose name starts with "gob" (exact match wins)
/// - `"2.gob"`    — second item whose name starts with "gob"
/// - `"all.gob"`  — all items whose name starts with "gob"
/// - `"all"`      — all items
///
/// Match priority (highest to lowest):
///   1. Exact full name:   `"goblin guard"` matches `"goblin guard"`
///   2. Full name prefix:  `"gob"`          matches `"goblin guard"`
///   3. Word exact:        `"guard"`        matches `"goblin guard"`
///   4. Word prefix:       `"gua"`          matches `"goblin guard"`
pub fn trie_match<S, T, I>(input: &str, items: I) -> TrieMatch<T>
where
    S: AsRef<str>,
    I: IntoIterator<Item = (S, T)>,
{
    if input.is_empty() {
        return TrieMatch::None;
    }

    let (index, query) = parse_input(input);
    let query_lower = query.to_lowercase();

    let mut exact_full: Vec<T> = Vec::new();
    let mut prefix_full: Vec<T> = Vec::new();
    let mut exact_word: Vec<T> = Vec::new();
    let mut prefix_word: Vec<T> = Vec::new();

    for (name, item) in items {
        if query_lower.is_empty() {
            exact_full.push(item);
            continue;
        }
        let name_lower = name.as_ref().to_lowercase();
        if name_lower == query_lower {
            exact_full.push(item);
        } else if name_lower.starts_with(&query_lower) {
            prefix_full.push(item);
        } else {
            // Check word-level matches: exact word beats prefix word
            let mut word_tier = 0u8; // 0=none, 1=prefix, 2=exact
            for word in name_lower.split_whitespace() {
                if word == query_lower {
                    word_tier = 2;
                    break;
                } else if word.starts_with(&query_lower) {
                    word_tier = 1;
                }
            }
            match word_tier {
                2 => exact_word.push(item),
                1 => prefix_word.push(item),
                _ => {}
            }
        }
    }

    exact_full.extend(prefix_full);
    exact_full.extend(exact_word);
    exact_full.extend(prefix_word);
    let mut candidates = exact_full;

    match index {
        Index::All => {
            if candidates.is_empty() {
                TrieMatch::None
            } else {
                TrieMatch::Many(candidates)
            }
        }
        Index::Nth(n) => {
            if n == 0 || n > candidates.len() {
                TrieMatch::None
            } else {
                TrieMatch::One(candidates.remove(n - 1))
            }
        }
        Index::First => {
            if candidates.is_empty() {
                TrieMatch::None
            } else {
                TrieMatch::One(candidates.remove(0))
            }
        }
    }
}

enum Index {
    First,
    Nth(usize),
    All,
}

fn parse_input(input: &str) -> (Index, &str) {
    let lower = input.to_lowercase();

    if lower == "all" {
        return (Index::All, "");
    }
    if lower.starts_with("all.") {
        return (Index::All, &input[4..]);
    }
    if let Some(dot) = input.find('.') {
        if let Ok(n) = input[..dot].parse::<usize>() {
            return (Index::Nth(n), &input[dot + 1..]);
        }
    }
    (Index::First, input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names<'a>(items: &'a [&'a str]) -> Vec<(&'a str, &'a str)> {
        items.iter().map(|&s| (s, s)).collect()
    }

    #[test]
    fn exact_match() {
        let items = names(&["goblin", "golem", "rat"]);
        let TrieMatch::One(v) = trie_match("goblin", items) else {
            panic!("expected One");
        };
        assert_eq!(v, "goblin");
    }

    #[test]
    fn prefix_match() {
        let items = names(&["rat", "goblin", "golem"]);
        let TrieMatch::One(v) = trie_match("go", items) else {
            panic!("expected One");
        };
        assert_eq!(v, "goblin");
    }

    #[test]
    fn exact_beats_prefix() {
        let items = names(&["goblin", "golem"]);
        let TrieMatch::One(v) = trie_match("golem", items) else {
            panic!("expected One");
        };
        assert_eq!(v, "golem");
    }

    #[test]
    fn word_exact_match() {
        let items = names(&["goblin guard", "goblin shaman"]);
        let TrieMatch::One(v) = trie_match("guard", items) else {
            panic!("expected One");
        };
        assert_eq!(v, "goblin guard");
    }

    #[test]
    fn word_prefix_match() {
        let items = names(&["goblin guard", "goblin shaman"]);
        let TrieMatch::One(v) = trie_match("gua", items) else {
            panic!("expected One");
        };
        assert_eq!(v, "goblin guard");
    }

    #[test]
    fn full_prefix_beats_word_exact() {
        // "gob" is a full-name prefix of both; "goblin" is a word exact of "goblin guard"
        // but full prefix should rank above word exact
        let items = names(&["goblin guard", "goblin"]);
        let TrieMatch::One(v) = trie_match("goblin", items) else {
            panic!("expected One");
        };
        assert_eq!(v, "goblin");
    }

    #[test]
    fn nth_match() {
        let items = names(&["goblin", "goblin", "rat"]);
        let TrieMatch::One(v) = trie_match("2.goblin", items) else {
            panic!("expected One");
        };
        assert_eq!(v, "goblin");
    }

    #[test]
    fn nth_word_match() {
        let items = names(&["goblin guard", "goblin guard", "goblin shaman"]);
        let TrieMatch::One(v) = trie_match("2.guard", items) else {
            panic!("expected One");
        };
        assert_eq!(v, "goblin guard");
    }

    #[test]
    fn nth_out_of_range() {
        let items = names(&["goblin"]);
        assert!(matches!(trie_match("2.goblin", items), TrieMatch::None));
    }

    #[test]
    fn all_prefix() {
        let items = names(&["goblin", "golem", "rat"]);
        let TrieMatch::Many(vs) = trie_match("all.go", items) else {
            panic!("expected Many");
        };
        assert_eq!(vs, vec!["goblin", "golem"]);
    }

    #[test]
    fn all_word() {
        let items = names(&["goblin guard", "goblin shaman", "rat"]);
        let TrieMatch::Many(vs) = trie_match("all.goblin", items) else {
            panic!("expected Many");
        };
        assert_eq!(vs.len(), 2);
    }

    #[test]
    fn all_bare() {
        let items = names(&["goblin", "rat"]);
        let TrieMatch::Many(vs) = trie_match("all", items) else {
            panic!("expected Many");
        };
        assert_eq!(vs.len(), 2);
    }

    #[test]
    fn no_match() {
        let items = names(&["goblin"]);
        assert!(matches!(trie_match("dragon", items), TrieMatch::None));
    }

    #[test]
    fn empty_input() {
        let items = names(&["goblin"]);
        assert!(matches!(trie_match("", items), TrieMatch::None));
    }

    #[test]
    fn case_insensitive() {
        let items = names(&["Goblin Guard"]);
        let TrieMatch::One(v) = trie_match("GUARD", items) else {
            panic!("expected One");
        };
        assert_eq!(v, "Goblin Guard");
    }
}
