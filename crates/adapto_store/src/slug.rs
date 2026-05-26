/// Transliterate and slugify a string for use in URLs.
///
/// Handles Cyrillic (Russian + Kazakh), removes trademark symbols,
/// converts whitespace/separators to hyphens, and deduplicates hyphens.
pub fn slugify(input: &str) -> String {
    static MAP: &[(char, &str)] = &[
        ('а', "a"), ('б', "b"), ('в', "v"), ('г', "g"), ('д', "d"),
        ('е', "e"), ('ё', "yo"), ('ж', "zh"), ('з', "z"), ('и', "i"),
        ('й', "i"), ('к', "k"), ('л', "l"), ('м', "m"), ('н', "n"),
        ('о', "o"), ('п', "p"), ('р', "r"), ('с', "s"), ('т', "t"),
        ('у', "u"), ('ф', "f"), ('х', "kh"), ('ц', "ts"), ('ч', "ch"),
        ('ш', "sh"), ('щ', "shch"), ('ъ', ""), ('ы', "y"), ('ь', ""),
        ('э', "e"), ('ю', "yu"), ('я', "ya"),
        // Kazakh-specific
        ('і', "i"), ('қ', "q"), ('ң', "ng"), ('ғ', "gh"),
        ('ү', "u"), ('ұ', "u"), ('ә', "a"), ('ө', "o"), ('һ', "h"),
    ];

    let lower = input.to_lowercase();
    let mut buf = String::with_capacity(lower.len());
    for c in lower.chars() {
        if c == '®' || c == '™' || c == '©' {
            continue;
        }
        if let Some((_, repl)) = MAP.iter().find(|(k, _)| *k == c) {
            buf.push_str(repl);
        } else if c.is_ascii_alphanumeric() || c == '-' {
            buf.push(c);
        } else if c == ' ' || c == '_' || c == '/' || c == ',' || c == '.' {
            buf.push('-');
        }
    }
    buf.split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Check whether a string is a valid URL slug.
///
/// Valid slugs contain only lowercase ASCII alphanumerics and hyphens,
/// do not start or end with a hyphen, and have no consecutive hyphens.
pub fn is_valid_slug(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    if s.starts_with('-') || s.ends_with('-') {
        return false;
    }
    if s.contains("--") {
        return false;
    }
    s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn russian_cyrillic() {
        assert_eq!(slugify("Привет Мир"), "privet-mir");
    }

    #[test]
    fn kazakh_chars() {
        assert_eq!(slugify("Қазақстан"), "qazaqstan");
        assert_eq!(slugify("Әділет"), "adilet");
        assert_eq!(slugify("Өзбекстан"), "ozbekstan");
    }

    #[test]
    fn trademark_removal() {
        assert_eq!(slugify("Аспирин® таблетки"), "aspirin-tabletki");
        assert_eq!(slugify("Brand™ Product©"), "brand-product");
    }

    #[test]
    fn separator_normalization() {
        assert_eq!(slugify("hello world"), "hello-world");
        assert_eq!(slugify("hello_world"), "hello-world");
        assert_eq!(slugify("hello/world"), "hello-world");
        assert_eq!(slugify("hello,world"), "hello-world");
        assert_eq!(slugify("hello.world"), "hello-world");
    }

    #[test]
    fn hyphen_deduplication() {
        assert_eq!(slugify("hello   world"), "hello-world");
        assert_eq!(slugify("a - b - c"), "a-b-c");
    }

    #[test]
    fn empty_input() {
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn ascii_passthrough() {
        assert_eq!(slugify("hello-world-123"), "hello-world-123");
    }

    #[test]
    fn mixed_script() {
        assert_eq!(slugify("Закон about образовании"), "zakon-about-obrazovanii");
    }

    #[test]
    fn valid_slug_check() {
        assert!(is_valid_slug("hello-world"));
        assert!(is_valid_slug("abc123"));
        assert!(is_valid_slug("a"));
        assert!(!is_valid_slug(""));
        assert!(!is_valid_slug("-hello"));
        assert!(!is_valid_slug("hello-"));
        assert!(!is_valid_slug("hello--world"));
        assert!(!is_valid_slug("Hello"));
        assert!(!is_valid_slug("hello world"));
    }
}
