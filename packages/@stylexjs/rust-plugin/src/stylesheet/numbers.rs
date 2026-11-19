pub(crate) fn js_number(value: f64) -> String {
    ryu_js::Buffer::new().format(value).to_owned()
}
