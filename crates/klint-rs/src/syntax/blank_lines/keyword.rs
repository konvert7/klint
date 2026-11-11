#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Keyword {
    If,
    For,
    While,
    Switch,
    Try,
    Return,
    Guard,
    Loop,
    With,
}

impl Keyword {
    pub(crate) fn parse(word: &str) -> Option<Keyword> {
        match word {
            "if" => Some(Keyword::If),
            "for" => Some(Keyword::For),
            "while" => Some(Keyword::While),
            "switch" | "match" => Some(Keyword::Switch),
            "try" => Some(Keyword::Try),
            "return" => Some(Keyword::Return),
            "guard" => Some(Keyword::Guard),
            "loop" => Some(Keyword::Loop),
            "with" => Some(Keyword::With),
            _ => None,
        }
    }

    const fn bit(self) -> u16 {
        1 << (self as u16)
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct KeywordSet(u16);

impl KeywordSet {
    pub(crate) const ALL_BEFORE: KeywordSet = KeywordSet(0b1_1111_1111);
    pub(crate) const ALL_AFTER: KeywordSet =
        KeywordSet(Self::ALL_BEFORE.0 & !Keyword::Return.bit());

    pub(crate) fn with(self, keyword: Keyword) -> KeywordSet {
        KeywordSet(self.0 | keyword.bit())
    }

    pub(crate) fn contains(self, keyword: Keyword) -> bool {
        self.0 & keyword.bit() != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_maps_if_to_if() {
        assert_eq!(Keyword::parse("if"), Some(Keyword::If));
    }

    #[test]
    fn parse_maps_match_to_switch() {
        assert_eq!(Keyword::parse("match"), Some(Keyword::Switch));
    }

    #[test]
    fn parse_maps_switch_to_switch() {
        assert_eq!(Keyword::parse("switch"), Some(Keyword::Switch));
    }

    #[test]
    fn parse_maps_return_to_return() {
        assert_eq!(Keyword::parse("return"), Some(Keyword::Return));
    }

    #[test]
    fn parse_maps_with_to_with() {
        assert_eq!(Keyword::parse("with"), Some(Keyword::With));
    }

    #[test]
    fn parse_rejects_foreach() {
        assert_eq!(Keyword::parse("foreach"), None);
    }

    #[test]
    fn parse_is_case_sensitive() {
        assert_eq!(Keyword::parse("If"), None);
    }

    #[test]
    fn parse_rejects_the_empty_string() {
        assert_eq!(Keyword::parse(""), None);
    }

    #[test]
    fn default_set_contains_no_keyword() {
        assert!(!KeywordSet::default().contains(Keyword::If));
    }

    #[test]
    fn with_adds_the_keyword_to_the_set() {
        assert!(
            KeywordSet::default()
                .with(Keyword::Switch)
                .contains(Keyword::Switch)
        );
    }

    #[test]
    fn with_the_same_keyword_twice_equals_once() {
        assert_eq!(
            KeywordSet::default()
                .with(Keyword::Switch)
                .with(Keyword::Switch),
            KeywordSet::default().with(Keyword::Switch)
        );
    }

    #[test]
    fn all_after_does_not_contain_return() {
        assert!(!KeywordSet::ALL_AFTER.contains(Keyword::Return));
    }

    #[test]
    fn all_before_contains_return() {
        assert!(KeywordSet::ALL_BEFORE.contains(Keyword::Return));
    }
}
