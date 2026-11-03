//! The JSON cargo writes (`--message-format=json`, `cargo metadata`),
//! read into a [`Json`] tree. Numbers stay text: nothing here does
//! arithmetic on them.

/// A JSON value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Json {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    /// Fields in the order written.
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// The field `key` of an object.
    pub(crate) fn get(&self, key: &str) -> Option<&Json> {
        let Json::Obj(fields) = self else {
            return None;
        };
        fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub(crate) fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(items) => Some(items),
            _ => None,
        }
    }
}

/// Parses the JSON document `text`; `None` if it is not one.
pub(crate) fn parse(text: &str) -> Option<Json> {
    let mut p = Parser {
        chars: text.chars().peekable(),
    };
    let value = p.value()?;
    p.space();
    p.chars.peek().is_none().then_some(value)
}

struct Parser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl Parser<'_> {
    fn space(&mut self) {
        while self.chars.next_if(|c| c.is_ascii_whitespace()).is_some() {}
    }

    fn eat(&mut self, c: char) -> bool {
        self.space();
        self.chars.next_if_eq(&c).is_some()
    }

    fn value(&mut self) -> Option<Json> {
        self.space();
        match *self.chars.peek()? {
            '{' => self.object(),
            '[' => self.array(),
            '"' => {
                self.chars.next();
                self.string().map(Json::Str)
            }
            't' => self.word("true", Json::Bool(true)),
            'f' => self.word("false", Json::Bool(false)),
            'n' => self.word("null", Json::Null),
            _ => self.number(),
        }
    }

    fn word(&mut self, word: &str, value: Json) -> Option<Json> {
        word.chars()
            .all(|c| self.chars.next_if_eq(&c).is_some())
            .then_some(value)
    }

    fn number(&mut self) -> Option<Json> {
        let mut text = String::new();
        while let Some(c) = self
            .chars
            .next_if(|c| c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E'))
        {
            text.push(c);
        }
        (!text.is_empty()).then_some(Json::Num(text))
    }

    fn array(&mut self) -> Option<Json> {
        self.chars.next();
        let mut items = Vec::new();
        if self.eat(']') {
            return Some(Json::Arr(items));
        }
        loop {
            items.push(self.value()?);
            if self.eat(']') {
                return Some(Json::Arr(items));
            }
            if !self.eat(',') {
                return None;
            }
        }
    }

    fn object(&mut self) -> Option<Json> {
        self.chars.next();
        let mut fields = Vec::new();
        if self.eat('}') {
            return Some(Json::Obj(fields));
        }
        loop {
            if !self.eat('"') {
                return None;
            }
            let key = self.string()?;
            if !self.eat(':') {
                return None;
            }
            fields.push((key, self.value()?));
            if self.eat('}') {
                return Some(Json::Obj(fields));
            }
            if !self.eat(',') {
                return None;
            }
        }
    }

    /// The rest of a string after its opening quote, unescaped.
    fn string(&mut self) -> Option<String> {
        let mut out = String::new();
        loop {
            match self.chars.next()? {
                '"' => return Some(out),
                '\\' => out.push(self.escape()?),
                c => out.push(c),
            }
        }
    }

    fn escape(&mut self) -> Option<char> {
        match self.chars.next()? {
            'n' => Some('\n'),
            't' => Some('\t'),
            'r' => Some('\r'),
            'b' => Some('\u{8}'),
            'f' => Some('\u{c}'),
            'u' => self.unicode(),
            c @ ('"' | '\\' | '/') => Some(c),
            _ => None,
        }
    }

    /// `\uXXXX`, joining a surrogate pair.
    fn unicode(&mut self) -> Option<char> {
        let high = self.hex4()?;
        if !(0xD800..0xDC00).contains(&high) {
            return char::from_u32(high);
        }
        let low = match (self.chars.next()?, self.chars.next()?) {
            ('\\', 'u') => self.hex4()?,
            _ => return None,
        };
        if !(0xDC00..0xE000).contains(&low) {
            return None;
        }
        char::from_u32(0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00))
    }

    fn hex4(&mut self) -> Option<u32> {
        let hex: String = self.chars.by_ref().take(4).collect();
        if hex.len() != 4 {
            return None;
        }
        u32::from_str_radix(&hex, 16).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_parse_into_the_tree() {
        let got = parse(r#" {"a": [1, -2.5e3, true, false, null], "b": {"c": "d"}, "e": []} "#);
        let want = Json::Obj(vec![
            (
                "a".into(),
                Json::Arr(vec![
                    Json::Num("1".into()),
                    Json::Num("-2.5e3".into()),
                    Json::Bool(true),
                    Json::Bool(false),
                    Json::Null,
                ]),
            ),
            (
                "b".into(),
                Json::Obj(vec![("c".into(), Json::Str("d".into()))]),
            ),
            ("e".into(), Json::Arr(vec![])),
        ]);
        assert_eq!(got, Some(want));
    }

    #[test]
    fn strings_unescape() {
        let got = parse(r#""q\" b\\ \/ \n\t\r\b\f \u00e9 \ud83d\ude00 C:\\x""#);
        assert_eq!(
            got,
            Some(Json::Str("q\" b\\ / \n\t\r\u{8}\u{c} é 😀 C:\\x".into()))
        );
    }

    #[test]
    fn malformed_text_is_none() {
        for bad in [
            "",
            "{",
            "[1,]",
            "[1 2]",
            "{\"a\" 1}",
            "{a: 1}",
            "\"open",
            "tru",
            "1 2",
            "\"\\x\"",
            "\"\\ud83d\"",
            "\"\\u12\"",
            "{}x",
        ] {
            assert_eq!(parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn get_finds_a_field() {
        let v = parse(r#"{"k": "v", "n": 1}"#).unwrap();
        assert_eq!(v.get("k").and_then(Json::as_str), Some("v"));
        assert_eq!(v.get("n").and_then(Json::as_str), None);
        assert_eq!(v.get("x"), None);
        assert_eq!(Json::Null.get("k"), None);
    }
}
