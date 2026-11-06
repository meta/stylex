use regex::Regex;
use std::sync::OnceLock;

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
