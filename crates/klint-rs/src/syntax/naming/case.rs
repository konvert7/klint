#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    Unknown,
    Number,
    Lower,
    Camel,
    Capital,
    Pascal,
    Upper,
    Snake,
    Constant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Format {
    Camel,
    Pascal,
    Upper,
    Snake,
}

impl Format {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Format::Camel => "camelCase",
            Format::Pascal => "PascalCase",
            Format::Upper => "UPPERCASE",
            Format::Snake => "snake_case",
        }
    }

    fn accepts(self, case: Case) -> bool {
        if case == Case::Number {
            return true;
        }

        match self {
            Format::Camel => matches!(case, Case::Camel | Case::Lower),
            Format::Pascal => matches!(case, Case::Pascal | Case::Capital),
            Format::Upper => matches!(case, Case::Upper | Case::Capital),
            Format::Snake => matches!(case, Case::Snake | Case::Lower),
        }
    }
}

pub(crate) fn is_in_any(name: &str, formats: &[Format], strict: bool) -> bool {
    let case = identify(name, strict);

    formats.iter().any(|format| format.accepts(case))
}

fn identify(name: &str, strict: bool) -> Case {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Case::Unknown;
    };
    let mut case = first_case(first);
    let mut walk = Walk {
        previous: first,
        consecutive_upper: false,
        strict,
    };

    for current in chars {
        case = walk.step(case, current);
        if case == Case::Unknown {
            return case;
        }

        walk.previous = current;
    }

    case
}

fn first_case(first: char) -> Case {
    if first.is_uppercase() {
        Case::Capital
    } else if first.is_lowercase() {
        Case::Lower
    } else if first.is_numeric() {
        Case::Number
    } else {
        Case::Unknown
    }
}

struct Walk {
    previous: char,
    consecutive_upper: bool,
    strict: bool,
}

impl Walk {
    fn step(&mut self, case: Case, current: char) -> Case {
        if current == '_' {
            return self.underscore(case);
        }

        if current.is_uppercase() {
            self.consecutive_upper |= self.previous.is_uppercase();

            return self.uppercase(case);
        }

        if current.is_lowercase() {
            return self.lowercase(case);
        }

        if current.is_numeric() {
            case
        } else {
            Case::Unknown
        }
    }

    fn underscore(&self, case: Case) -> Case {
        match case {
            _ if self.previous == '_' => Case::Unknown,
            Case::Constant | Case::Capital | Case::Upper => Case::Constant,
            Case::Lower | Case::Snake => Case::Snake,
            _ => Case::Unknown,
        }
    }

    fn uppercase(&self, case: Case) -> Case {
        match case {
            Case::Camel | Case::Pascal if self.strict && self.consecutive_upper => Case::Unknown,
            Case::Camel | Case::Pascal | Case::Constant => case,
            Case::Lower | Case::Number => Case::Camel,
            Case::Capital | Case::Upper => Case::Upper,
            _ => Case::Unknown,
        }
    }

    fn lowercase(&self, case: Case) -> Case {
        match case {
            Case::Camel | Case::Lower | Case::Snake => case,
            Case::Number => Case::Lower,
            Case::Pascal | Case::Capital => Case::Pascal,
            Case::Upper if !self.strict || !self.consecutive_upper => Case::Pascal,
            _ => Case::Unknown,
        }
    }
}

pub(crate) fn convert(name: &str, format: Format) -> String {
    let words = words(name);
    let mut converted = String::new();

    for (index, word) in words.iter().enumerate() {
        let joined = match format {
            Format::Snake if index > 0 => format!("_{}", word.to_lowercase()),
            Format::Snake => word.to_lowercase(),
            Format::Upper => word.to_uppercase(),
            Format::Camel if index == 0 => word.to_lowercase(),
            Format::Camel | Format::Pascal => capitalized(word),
        };
        converted.push_str(&joined);
    }

    converted
}

fn capitalized(word: &str) -> String {
    let lower = word.to_lowercase();
    let mut chars = lower.chars();

    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

fn words(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.chars().collect();
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();

    for (index, &ch) in chars.iter().enumerate() {
        if ch == '_' {
            push_word(&mut words, &mut current);
            continue;
        }

        if starts_word(&chars, index) {
            push_word(&mut words, &mut current);
        }

        current.push(ch);
    }

    push_word(&mut words, &mut current);
    words
}

fn starts_word(chars: &[char], index: usize) -> bool {
    let Some(previous) = index.checked_sub(1).map(|at| chars[at]) else {
        return false;
    };
    let current = chars[index];
    let next_is_lower = chars.get(index + 1).is_some_and(|next| next.is_lowercase());

    current.is_uppercase()
        && (previous.is_lowercase()
            || previous.is_numeric()
            || (previous.is_uppercase() && next_is_lower))
}

fn push_word(words: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        words.push(std::mem::take(current));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FUNCTION: &[Format] = &[Format::Camel, Format::Pascal, Format::Upper];

    #[test]
    fn global_function_names_follow_biome_in_strict_mode() {
        let passing = [
            "foo", "fooBar", "FooBar", "FOO", "URL", "a", "A", "x1", "i18n", "V2Api", "A1B", "aB",
        ];
        let failing = [
            "foo_bar",
            "FOO_BAR",
            "Foo_Bar",
            "HTTPClient",
            "getHTTP",
            "aBC",
            "fooBAr",
            "ABc",
            "fOO",
            "foo__bar",
        ];

        for name in passing {
            assert!(is_in_any(name, FUNCTION, true), "{name} should pass");
        }

        for name in failing {
            assert!(!is_in_any(name, FUNCTION, true), "{name} should fail");
        }
    }

    #[test]
    fn class_names_must_be_pascal_case() {
        for name in ["FooBar", "A", "X1", "Http2Client", "I18n", "Url"] {
            assert!(is_in_any(name, &[Format::Pascal], true), "{name}");
        }

        for name in ["foo", "a", "URL", "AB", "A1B", "HTTPClient", "userService"] {
            assert!(!is_in_any(name, &[Format::Pascal], true), "{name}");
        }
    }

    #[test]
    fn non_strict_pascal_case_allows_acronyms() {
        assert!(is_in_any("HTTPServer", &[Format::Pascal], false));
        assert!(is_in_any("IDisposable", &[Format::Pascal], false));
        assert!(!is_in_any("IDisposable", &[Format::Pascal], true));
    }

    #[test]
    fn snake_case_accepts_lowercase_words_and_digits() {
        for name in ["foo", "foo_bar", "utf8_decode", "x2"] {
            assert!(is_in_any(name, &[Format::Snake], false), "{name}");
        }

        for name in ["fooBar", "Foo", "foo__bar", "FOO_BAR"] {
            assert!(!is_in_any(name, &[Format::Snake], false), "{name}");
        }
    }

    #[test]
    fn conversion_splits_words_like_biome() {
        assert_eq!(convert("get_user", Format::Camel), "getUser");
        assert_eq!(convert("get_user", Format::Pascal), "GetUser");
        assert_eq!(convert("HTTPClient2", Format::Pascal), "HttpClient2");
        assert_eq!(convert("XMLHttpRequest", Format::Pascal), "XmlHttpRequest");
        assert_eq!(convert("getHTTP", Format::Camel), "getHttp");
        assert_eq!(convert("aBC", Format::Camel), "aBc");
        assert_eq!(convert("foo__bar", Format::Camel), "fooBar");
        assert_eq!(convert("getUserName", Format::Snake), "get_user_name");
        assert_eq!(convert("parseURL", Format::Snake), "parse_url");
    }
}
