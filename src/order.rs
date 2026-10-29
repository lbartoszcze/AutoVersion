//! Ordering of coordinates, triples or not.

use std::cmp::Ordering;

/// One token of a coordinate: a number sorts before any text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// Decimal digits without leading zeros (`"0"` for zero), so numbers of any
    /// size compare without overflow.
    Number(String),
    Text(String),
}

impl Ord for Token {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Token::Number(left), Token::Number(right)) => {
                left.len().cmp(&right.len()).then_with(|| left.cmp(right))
            }
            (Token::Number(_), Token::Text(_)) => Ordering::Less,
            (Token::Text(_), Token::Number(_)) => Ordering::Greater,
            (Token::Text(left), Token::Text(right)) => left.cmp(right),
        }
    }
}

impl PartialOrd for Token {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The digits of a token read as an integer the way the rule's first port
/// read it: surrounding whitespace and a leading `+` allowed, underscores only
/// between digits.
fn integer(token: &str) -> Option<String> {
    let token = token.trim();
    // One optional leading '+' sign: it is a single byte, so skipping it keeps
    // the slice on a character boundary.
    let token = &token[usize::from(token.starts_with('+'))..];
    let bytes = token.as_bytes();
    let well_formed = !bytes.is_empty()
        && bytes.first().is_some_and(u8::is_ascii_digit)
        && bytes.last().is_some_and(u8::is_ascii_digit)
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || *byte == b'_')
        && !token.contains("__");
    if !well_formed {
        return None;
    }
    let digits: String = token.chars().filter(char::is_ascii_digit).collect();
    let significant = digits.trim_start_matches('0');
    Some(if significant.is_empty() {
        "0".to_string()
    } else {
        significant.to_string()
    })
}

/// Split on `.` and `-`; numeric tokens as numbers, sorting before strings.
/// Ordering works on coordinates that are not triples, because a version that
/// can never be advanced can still be compared.
pub fn version_tuple(value: &str) -> Vec<Token> {
    value
        .split(['.', '-'])
        .map(|token| match integer(token) {
            Some(number) => Token::Number(number),
            None => Token::Text(token.to_string()),
        })
        .collect()
}

/// True when `candidate` is strictly newer than `installed`.
pub fn version_newer(installed: &str, candidate: &str) -> bool {
    version_tuple(installed) < version_tuple(candidate)
}
