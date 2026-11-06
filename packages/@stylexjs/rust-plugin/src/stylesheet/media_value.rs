//! Values accepted by MediaQuery's simple-pair and Calc parsers.
use super::numbers::{dimension, js_number};

fn error() -> String {
    "Invalid media query syntax.".to_owned()
}
fn ident(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '-' || first == '_' || first.is_alphabetic() || first as u32 >= 128)
        && chars.all(|c| c == '-' || c == '_' || c.is_alphanumeric() || c as u32 >= 128)
}
fn number(s: &str) -> Result<String, String> {
    let (num, unit) = dimension(s).ok_or_else(error)?;
    if !unit.is_empty() {
        return Err(error());
    }
    num.parse::<f64>().map(js_number).map_err(|_| error())
}
pub(crate) fn normalize(s: &str) -> Result<String, String> {
    let s = s.trim();
    if s.starts_with("calc(") && s.ends_with(')') {
        let mut parser = Calc {
            input: &s[5..s.len() - 1],
            at: 0,
        };
        let expression = parser.expression(false)?;
        if parser.at != parser.input.len() {
            return Err(error());
        }
        return Ok(format!("calc({expression})"));
    }
    let fraction = s.split('/').collect::<Vec<_>>();
    if fraction.len() == 2 {
        return Ok(format!(
            "{} / {}",
            number(fraction[0].trim())?,
            number(fraction[1].trim())?
        ));
    }
    if let Some((num, unit)) = dimension(s) {
        if !unit.is_empty() && !ident(unit) {
            return Err(error());
        }
        return Ok(format!(
            "{}{}",
            num.parse::<f64>().map(js_number).map_err(|_| error())?,
            unit
        ));
    }
    if ident(s) {
        Ok(s.to_owned())
    } else {
        Err(error())
    }
}
struct Calc<'a> {
    input: &'a str,
    at: usize,
}
impl Calc<'_> {
    fn whitespace(&mut self) {
        while self
            .input
            .as_bytes()
            .get(self.at)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.at += 1;
        }
    }
    fn expression(&mut self, group: bool) -> Result<String, String> {
        let mut out = self.term()?;
        loop {
            self.whitespace();
            let Some(&c) = self.input.as_bytes().get(self.at) else {
                return if group { Err(error()) } else { Ok(out) };
            };
            if c == b')' && group {
                self.at += 1;
                return Ok(out);
            }
            if !matches!(c, b'+' | b'-' | b'*' | b'/') {
                return Err(error());
            }
            // A sign next to a number belongs to it, so the second value is unseparated.
            if matches!(c, b'+' | b'-')
                && self
                    .input
                    .as_bytes()
                    .get(self.at + 1)
                    .is_some_and(|c| c.is_ascii_digit() || *c == b'.')
            {
                return Err(error());
            }
            self.at += 1;
            let right = self.term()?;
            out.push(' ');
            out.push(c as char);
            out.push(' ');
            out.push_str(&right);
        }
    }
    fn term(&mut self) -> Result<String, String> {
        self.whitespace();
        if self.input.as_bytes().get(self.at) == Some(&b'(') {
            self.at += 1;
            return Ok(format!("({})", self.expression(true)?));
        }
        let tail = &self.input[self.at..];
        if let Some((num, _)) = dimension(tail) {
            self.at += num.len();
            let begin = self.at;
            if self.input.as_bytes().get(self.at) == Some(&b'%') {
                self.at += 1;
            } else {
                while let Some(c) = self.input[self.at..].chars().next() {
                    if c.is_alphanumeric() || matches!(c, '_' | '-') || c as u32 >= 128 {
                        self.at += c.len_utf8();
                    } else {
                        break;
                    }
                }
            }
            return Ok(format!(
                "{}{}",
                num.parse::<f64>().map(js_number).map_err(|_| error())?,
                &self.input[begin..self.at]
            ));
        }
        for constant in ["-infinity", "infinity", "NaN", "pi", "e"] {
            if tail.starts_with(constant) {
                self.at += constant.len();
                return Ok(constant.to_owned());
            }
        }
        Err(error())
    }
}
