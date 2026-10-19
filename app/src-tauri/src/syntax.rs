use regex::Regex;
use serde_json::{Map, Value};

#[derive(Debug)]
pub struct Token {
    pub value: String,
    pub start: usize,
    pub end: usize,
    pub brace: bool,
}

pub fn tokens(text: &str) -> Result<Vec<Token>, String> {
    let pattern = Regex::new(r#""(?:\\.|[^"\\])*"|//[^\r\n]*|[{}]|[^\s{}"]+"#).unwrap();
    let escapes = Regex::new(r#"\\([\\"])"#).unwrap();
    let mut result = Vec::new();
    let mut position = 0;
    let offset = text.len() - text.trim_start_matches('\u{feff}').len();
    for item in pattern.find_iter(&text[offset..]) {
        let start = item.start() + offset;
        let end = item.end() + offset;
        if !text[position..start]
            .trim_matches(['\u{feff}', ' ', '\t', '\r', '\n'])
            .is_empty()
        {
            return Err("Invalid Steam configuration text.".into());
        }
        position = end;
        let raw = item.as_str();
        if raw.starts_with("//") {
            continue;
        }
        let quoted = raw.starts_with('"');
        let value = if quoted {
            escapes
                .replace_all(&raw[1..raw.len() - 1], "$1")
                .into_owned()
        } else {
            raw.to_owned()
        };
        result.push(Token {
            value,
            start,
            end,
            brace: !quoted && matches!(raw, "{" | "}"),
        });
    }
    if !text[position..].trim().is_empty() {
        return Err("Incomplete Steam configuration text.".into());
    }
    Ok(result)
}

pub fn keyvalues(text: &str) -> Result<Value, String> {
    fn object(
        items: &[Token],
        index: &mut usize,
        depth: usize,
    ) -> Result<Map<String, Value>, String> {
        if depth > 64 {
            return Err("Steam configuration nesting is too deep.".into());
        }
        let mut map = Map::new();
        while *index < items.len() {
            let key = &items[*index];
            *index += 1;
            if key.brace && key.value == "}" {
                return if depth > 0 {
                    Ok(map)
                } else {
                    Err("Unexpected closing brace.".into())
                };
            }
            if key.brace || *index >= items.len() {
                return Err("Missing Steam configuration value.".into());
            }
            let value = &items[*index];
            *index += 1;
            let value = if value.brace && value.value == "{" {
                Value::Object(object(items, index, depth + 1)?)
            } else if value.brace {
                return Err("Missing Steam configuration value.".into());
            } else {
                Value::String(value.value.clone())
            };
            if map.insert(key.value.clone(), value).is_some() {
                return Err("Duplicate Steam configuration key.".into());
            }
        }
        if depth > 0 {
            Err("Incomplete Steam configuration block.".into())
        } else {
            Ok(map)
        }
    }
    let items = tokens(text)?;
    Ok(Value::Object(object(&items, &mut 0, 0)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn steam_paths_and_comments_are_decoded() {
        let parsed = keyvalues(
            "\u{feff}\"libraryfolders\" { // hello\n \"1\" { \"path\" \"E:\\\\Games\\\\Steam\" } }",
        )
        .unwrap();
        assert_eq!(parsed["libraryfolders"]["1"]["path"], "E:\\Games\\Steam");
    }
    #[test]
    fn malformed_and_duplicate_entries_are_rejected() {
        for text in ["\"a", "a { b c", "a b a c", "a { b }"] {
            assert!(keyvalues(text).is_err(), "{text}");
        }
    }
}
