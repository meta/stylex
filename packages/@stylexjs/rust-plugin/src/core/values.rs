//! CSS-value lexical normalization; never run numeric rewrites over strings or URLs.
//! Mirrors Babel shared/utils/transform-value and its ordered AST normalizers.
use super::{CreateOptions, StyleValue};
use regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Debug)]
pub(crate) enum Token {
    Word(String),
    Space,
    Div(char),
    String(char, String),
    Comment(String),
    Function(String, Vec<Token>),
    Url(String, String),
}

fn whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}')
}
fn next(input: &str, at: usize) -> Option<(char, usize)> {
    input
        .get(at..)?
        .chars()
        .next()
        .map(|c| (c, at + c.len_utf8()))
}

pub(crate) fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut at = 0;
    let tokens = tokens_until(input, &mut at, false, false)?;
    Ok(tokens)
}

/// `calc` mirrors postcss-value-parser: directly inside a function named exactly
/// `calc`, `/` and `*` are operator words rather than dividers.
fn tokens_until(
    input: &str,
    at: &mut usize,
    function: bool,
    calc: bool,
) -> Result<Vec<Token>, String> {
    let mut out = Vec::new();
    while let Some((c, end)) = next(input, *at) {
        if c == ')' && function {
            *at = end;
            return Ok(out);
        }
        if whitespace(c) {
            *at = end;
            while let Some((c, end)) = next(input, *at) {
                if !whitespace(c) {
                    break;
                }
                *at = end;
            }
            out.push(Token::Space);
        } else if input[*at..].starts_with("/*") {
            let begin = *at + 2;
            if let Some(end) = input[begin..].find("*/") {
                out.push(Token::Comment(input[begin..begin + end].to_owned()));
                *at = begin + end + 2;
            } else {
                out.push(Token::Comment(input[begin..].to_owned()));
                *at = input.len();
            }
        } else if c == '\'' || c == '"' {
            let begin = end;
            *at = end;
            let mut closed = false;
            while let Some((v, end)) = next(input, *at) {
                if v == c {
                    out.push(Token::String(c, input[begin..*at].to_owned()));
                    *at = end;
                    closed = true;
                    break;
                }
                *at = end;
                if v == '\\' {
                    if let Some((_, end)) = next(input, *at) {
                        *at = end;
                    }
                }
            }
            if !closed {
                return Err(if function {
                    "Rule contains an unclosed function"
                } else {
                    "Rule contains an unclosed string"
                }
                .to_owned());
            }
        } else if calc && matches!(c, '/' | '*') {
            out.push(Token::Word(c.to_string()));
            *at = end;
        } else if matches!(c, ',' | '/' | ':') {
            out.push(Token::Div(c));
            *at = end;
        } else if c == '(' {
            *at = end;
            out.push(Token::Function(
                String::new(),
                tokens_until(input, at, true, false)?,
            ));
        } else {
            let begin = *at;
            *at = end;
            if c == '\\' {
                if let Some((_, end)) = next(input, *at) {
                    *at = end;
                }
            }
            while let Some((v, end)) = next(input, *at) {
                if whitespace(v)
                    || matches!(v, '(' | ')' | ',' | '/' | ':' | '\'' | '"')
                    || (calc && v == '*')
                {
                    break;
                }
                *at = end;
                if v == '\\' {
                    if let Some((_, end)) = next(input, *at) {
                        *at = end;
                    }
                }
            }
            let word = input[begin..*at].to_owned();
            if input[*at..].starts_with('(') {
                *at += 1;
                if word.eq_ignore_ascii_case("url") {
                    let begin = *at;
                    let mut quote = None;
                    let mut closed = false;
                    while let Some((v, end)) = next(input, *at) {
                        if v == ')' && quote.is_none() {
                            out.push(Token::Url(
                                word.clone(),
                                input[begin..*at].trim_matches(whitespace).to_owned(),
                            ));
                            *at = end;
                            closed = true;
                            break;
                        }
                        if v == '\'' || v == '"' {
                            if quote == Some(v) {
                                quote = None;
                            } else if quote.is_none() {
                                quote = Some(v);
                            }
                        }
                        *at = end;
                        if v == '\\' {
                            if let Some((_, end)) = next(input, *at) {
                                *at = end;
                            }
                        }
                    }
                    if !closed {
                        return Err("Rule contains an unclosed function".to_owned());
                    }
                } else {
                    let calc = word == "calc";
                    out.push(Token::Function(word, tokens_until(input, at, true, calc)?));
                }
            } else {
                out.push(Token::Word(word));
            }
        }
    }
    if function {
        Err("Rule contains an unclosed function".to_owned())
    } else {
        Ok(out)
    }
}

pub(crate) fn js_number(value: f64) -> String {
    ryu_js::Buffer::new().format(value).to_owned()
}

pub(crate) fn dimension(value: &str) -> Option<(&str, &str)> {
    static NUMBER: OnceLock<Regex> = OnceLock::new();
    let re = NUMBER.get_or_init(|| {
        Regex::new(r"^[+-]?(?:[0-9]*\.[0-9]+|[0-9]+\.?[0-9]*)(?:[eE][+-]?[0-9]+)?").unwrap()
    });
    let number = re.find(value)?;
    Some((&value[..number.end()], &value[number.end()..]))
}

pub fn get_number_suffix(key: &str) -> &'static str {
    if key.starts_with("--")
        || matches!(
            key,
            "WebkitLineClamp"
                | "animationIterationCount"
                | "aspectRatio"
                | "borderImageOutset"
                | "borderImageSlice"
                | "borderImageWidth"
                | "counterSet"
                | "counterReset"
                | "columnCount"
                | "flex"
                | "flexGrow"
                | "flexShrink"
                | "flexOrder"
                | "gridRow"
                | "gridRowStart"
                | "gridRowEnd"
                | "gridColumn"
                | "gridColumnStart"
                | "gridColumnEnd"
                | "gridArea"
                | "fontSizeAdjust"
                | "fontWeight"
                | "hyphenateLimitChars"
                | "lineClamp"
                | "lineHeight"
                | "maskBorderOutset"
                | "maskBorderSlice"
                | "maskBorderWidth"
                | "opacity"
                | "order"
                | "orphans"
                | "tabSize"
                | "widows"
                | "zIndex"
                | "fillOpacity"
                | "floodOpacity"
                | "rotate"
                | "scale"
                | "shapeImageThreshold"
                | "stopOpacity"
                | "strokeDasharray"
                | "strokeDashoffset"
                | "strokeMiterlimit"
                | "strokeOpacity"
                | "strokeWidth"
                | "mathDepth"
                | "zoom"
        )
    {
        ""
    } else if matches!(
        key,
        "animationDelay"
            | "animationDuration"
            | "transitionDelay"
            | "transitionDuration"
            | "voiceDuration"
    ) {
        "ms"
    } else {
        "px"
    }
}

pub fn dynamic_number_suffix(key: &str) -> &'static str {
    let suffix = get_number_suffix(key);
    if is_length_property(key) || suffix == "ms" {
        suffix
    } else {
        ""
    }
}

pub fn is_length_property(key: &str) -> bool {
    matches!(
        key,
        "backgroundPositionX"
            | "backgroundPositionY"
            | "blockSize"
            | "borderBlockEndWidth"
            | "borderBlockStartWidth"
            | "borderBlockWidth"
            | "borderVerticalWidth"
            | "borderBottomLeftRadius"
            | "borderBottomRightRadius"
            | "borderBottomWidth"
            | "borderEndEndRadius"
            | "borderEndStartRadius"
            | "borderInlineEndWidth"
            | "borderEndWidth"
            | "borderInlineStartWidth"
            | "borderStartWidth"
            | "borderInlineWidth"
            | "borderHorizontalWidth"
            | "borderLeftWidth"
            | "borderRightWidth"
            | "borderSpacing"
            | "borderStartEndRadius"
            | "borderStartStartRadius"
            | "borderTopLeftRadius"
            | "borderTopRightRadius"
            | "borderTopWidth"
            | "bottom"
            | "columnGap"
            | "columnRuleWidth"
            | "columnWidth"
            | "containIntrinsicBlockSize"
            | "containIntrinsicHeight"
            | "containIntrinsicInlineSize"
            | "containIntrinsicWidth"
            | "flexBasis"
            | "fontSize"
            | "fontSmooth"
            | "height"
            | "inlineSize"
            | "insetBlockEnd"
            | "insetBlockStart"
            | "insetInlineEnd"
            | "insetInlineStart"
            | "left"
            | "letterSpacing"
            | "marginBlockEnd"
            | "marginBlockStart"
            | "marginBottom"
            | "marginInlineEnd"
            | "marginEnd"
            | "marginInlineStart"
            | "marginStart"
            | "marginLeft"
            | "marginRight"
            | "marginTop"
            | "maxBlockSize"
            | "maxHeight"
            | "maxInlineSize"
            | "maxWidth"
            | "minBlockSize"
            | "minHeight"
            | "minInlineSize"
            | "minWidth"
            | "outlineOffset"
            | "outlineWidth"
            | "paddingBlockEnd"
            | "paddingBlockStart"
            | "paddingBottom"
            | "paddingInlineEnd"
            | "paddingEnd"
            | "paddingInlineStart"
            | "paddingStart"
            | "paddingLeft"
            | "paddingRight"
            | "paddingTop"
            | "perspective"
            | "right"
            | "rowGap"
            | "scrollMarginBlockEnd"
            | "scrollMarginBlockStart"
            | "scrollMarginBottom"
            | "scrollMarginInlineEnd"
            | "scrollMarginInlineStart"
            | "scrollMarginLeft"
            | "scrollMarginRight"
            | "scrollMarginTop"
            | "scrollPaddingBlockEnd"
            | "scrollPaddingBlockStart"
            | "scrollPaddingBottom"
            | "scrollPaddingInlineEnd"
            | "scrollPaddingInlineStart"
            | "scrollPaddingLeft"
            | "scrollPaddingRight"
            | "scrollPaddingTop"
            | "textIndent"
            | "textUnderlineOffset"
            | "top"
            | "verticalAlign"
            | "width"
            | "wordSpacing"
            | "offsetDistance"
            | "overflowClipMargin"
            | "scrollSnapMarginBottom"
            | "scrollSnapMarginLeft"
            | "scrollSnapMarginRight"
            | "scrollSnapMarginTop"
            | "shapeMargin"
            | "tabSize"
            | "textDecorationThickness"
            | "transformOrigin"
            | "translate"
            | "border"
            | "borderBlock"
            | "borderBlockEnd"
            | "borderBlockStart"
            | "borderBottom"
            | "borderLeft"
            | "borderRadius"
            | "borderRight"
            | "borderTop"
            | "borderWidth"
            | "columnRule"
            | "containIntrinsicSize"
            | "gap"
            | "inset"
            | "insetBlock"
            | "insetInline"
            | "margin"
            | "marginBlock"
            | "marginVertical"
            | "marginInline"
            | "marginHorizontal"
            | "offset"
            | "outline"
            | "padding"
            | "paddingBlock"
            | "paddingVertical"
            | "paddingInline"
            | "paddingHorizontal"
            | "scrollMargin"
            | "scrollMarginBlock"
            | "scrollMarginInline"
            | "scrollPadding"
            | "scrollPaddingBlock"
            | "scrollPaddingInline"
            | "scrollSnapMargin"
    )
}

pub fn transform_value(
    key: &str,
    value: &StyleValue,
    options: &CreateOptions,
) -> Result<String, String> {
    let raw = match value {
        StyleValue::String(value) => value.clone(),
        StyleValue::Number(value) => {
            // JS Math.round ties toward +infinity, unlike f64::round for negatives.
            let scaled = *value * 10000.0;
            let rounded = if scaled.is_finite() {
                let floor = scaled.floor();
                (if scaled - floor < 0.5 {
                    floor
                } else {
                    floor + 1.0
                }) / 10000.0
            } else {
                scaled / 10000.0
            };
            format!("{}{}", js_number(rounded), get_number_suffix(key))
        }
        _ => return Err(format!("Invalid value for property {key}")),
    };
    if matches!(
        key,
        "content" | "hyphenateCharacter" | "hyphenate-character"
    ) {
        return Ok(quote_content(&raw));
    }
    let mut tokens = tokenize(&raw)?;
    normalize_tokens(&mut tokens, key, options, false, true);
    Ok(render(&tokens))
}

fn normalize_tokens(
    tokens: &mut Vec<Token>,
    key: &str,
    options: &CreateOptions,
    in_function: bool,
    top: bool,
) {
    while matches!(tokens.first(), Some(Token::Space)) {
        tokens.remove(0);
    }
    while matches!(tokens.last(), Some(Token::Space)) {
        tokens.pop();
    }
    let mut out = Vec::new();
    for token in std::mem::take(tokens) {
        match token {
            Token::Space => {
                if !matches!(out.last(), Some(Token::Space | Token::Div(_))) {
                    out.push(Token::Space);
                }
            }
            Token::Div(c) => {
                if matches!(out.last(), Some(Token::Space)) {
                    out.pop();
                }
                out.push(Token::Div(c));
            }
            Token::Word(mut word) => {
                if word == "!important" && top && matches!(out.last(), Some(Token::Space)) {
                    out.pop();
                }
                if let Some((number, unit)) =
                    dimension(&word).map(|(number, unit)| (number.to_owned(), unit.to_owned()))
                {
                    if let Ok(mut value) = number.parse::<f64>() {
                        let mut unit = unit.to_owned();
                        if unit == "ms" && value >= 10.0 {
                            value /= 1000.0;
                            unit = "s".to_owned();
                            word = format!("{}s", js_number(value));
                        }
                        if !key.starts_with("--") && number == "0" {
                            match unit.as_str() {
                                "deg" | "grad" | "turn" | "rad" => {
                                    word = "0deg".to_owned();
                                    unit = "deg".to_owned();
                                }
                                "ms" | "s" => {
                                    word = "0s".to_owned();
                                    unit = "s".to_owned();
                                }
                                "fr" => word = "0fr".to_owned(),
                                "%" => word = "0%".to_owned(),
                                _ if !in_function => {
                                    word = "0".to_owned();
                                    unit.clear();
                                }
                                _ => {}
                            }
                        }
                        if (0.0..1.0).contains(&value) {
                            word = format!("{}{}", js_number(value).replacen("0.", ".", 1), unit);
                        }
                    }
                }
                if top
                    && matches!(key, "transitionProperty" | "willChange")
                    && !word.starts_with("--")
                {
                    word = super::dashify(&word);
                }
                // This normalizer is last, after leading-zero removal.
                if key == "fontSize" && options.enable_font_size_px_to_rem {
                    if let Some((number, "px")) = dimension(&word) {
                        if let Ok(value) = number.parse::<f64>() {
                            word = format!("{}rem", js_number(value / 16.0));
                        }
                    }
                }
                out.push(Token::Word(word));
            }
            Token::Function(name, mut children) => {
                normalize_tokens(&mut children, key, options, true, false);
                out.push(Token::Function(name, children));
            }
            Token::String(quote, value) => out.push(Token::String(
                if value.is_empty() { '"' } else { quote },
                value,
            )),
            Token::Url(name, raw) => out.push(Token::Url(
                name,
                if raw == "''" { "\"\"".to_owned() } else { raw },
            )),
            token => out.push(token),
        }
    }
    *tokens = out;
}

pub(crate) fn render(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|token| match token {
            Token::Word(word) => word.clone(),
            Token::Space => " ".to_owned(),
            Token::Div(',') => ",".to_owned(),
            Token::Div(c) => format!(" {c} "),
            Token::String(quote, value) => format!("{quote}{value}{quote}"),
            Token::Comment(value) => format!("/*{value}*/"),
            Token::Function(name, children) => format!("{name}({})", render(children)),
            Token::Url(name, raw) => format!("{name}({raw})"),
        })
        .collect()
}

fn content_keyword(value: &str) -> bool {
    matches!(
        value,
        "normal"
            | "none"
            | "open-quote"
            | "close-quote"
            | "no-open-quote"
            | "no-close-quote"
            | "inherit"
            | "initial"
            | "revert"
            | "revert-layer"
            | "unset"
    )
}
fn quote_content(raw: &str) -> String {
    let value = raw.trim();
    if content_keyword(value)
        || [
            "attr(",
            "counter(",
            "counters(",
            "url(",
            "linear-gradient(",
            "image-set(",
            "var(--",
        ]
        .iter()
        .any(|function| value.contains(function))
    {
        return value.to_owned();
    }
    if let Ok(tokens) = tokenize(value) {
        let list: Vec<_> = tokens
            .iter()
            .filter(|t| !matches!(t, Token::Space | Token::Div(_)))
            .collect();
        if list.iter().any(|t| matches!(t, Token::String(_, _)))
            && list.iter().all(|t| match t {
                Token::String(_, _) | Token::Function(_, _) | Token::Url(_, _) => true,
                Token::Word(value) => content_keyword(value),
                _ => false,
            })
        {
            return value.to_owned();
        }
    }
    let mut escaped = String::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                escaped.push('\\');
                if let Some(next) = chars.next() {
                    escaped.push(next);
                    if next == '\r' && chars.peek() == Some(&'\n') {
                        escaped.push(chars.next().unwrap());
                    }
                } else {
                    escaped.push('\\');
                }
            }
            '"' => escaped.push_str("\\\""),
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                escaped.push_str("\\A ");
            }
            '\n' | '\u{c}' => escaped.push_str("\\A "),
            _ => escaped.push(c),
        }
    }
    format!("\"{escaped}\"")
}
