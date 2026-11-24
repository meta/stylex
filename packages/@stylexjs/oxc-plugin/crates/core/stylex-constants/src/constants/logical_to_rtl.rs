use phf::phf_map;

// These properties are kept for a polyfill that is only used with
// `legacy-expand-shorthands`
pub static LOGICAL_TO_RTL: phf::Map<&'static str, &'static str> = phf_map! {
    "margin-start" => "margin-right",
    "margin-end" => "margin-left",
    "padding-start" => "padding-right",
    "padding-end" => "padding-left",
    "border-start" => "border-right",
    "border-end" => "border-left",
    "border-start-width" => "border-right-width",
    "border-end-width" => "border-left-width",
    "border-start-color" => "border-right-color",
    "border-end-color" => "border-left-color",
    "border-start-style" => "border-right-style",
    "border-end-style" => "border-left-style",
    "border-top-start-radius" => "border-top-right-radius",
    "border-bottom-start-radius" => "border-bottom-right-radius",
    "border-top-end-radius" => "border-top-left-radius",
    "border-bottom-end-radius" => "border-bottom-left-radius",
    "start" => "right",
    "end" => "left",
};

pub static INLINE_TO_RTL: phf::Map<&'static str, &'static str> = phf_map! {
    "margin-inline-start" => "margin-right",
    "margin-inline-end" => "margin-left",
    "padding-inline-start" => "padding-right",
    "padding-inline-end" => "padding-left",
    "border-inline-start" => "border-right",
    "border-inline-end" => "border-left",
    "border-inline-start-width" => "border-right-width",
    "border-inline-end-width" => "border-left-width",
    "border-inline-start-color" => "border-right-color",
    "border-inline-end-color" => "border-left-color",
    "border-inline-start-style" => "border-right-style",
    "border-inline-end-style" => "border-left-style",
    "border-start-start-radius" => "border-top-right-radius",
    "border-end-start-radius" => "border-bottom-right-radius",
    "border-start-end-radius" => "border-top-left-radius",
    "border-end-end-radius" => "border-bottom-left-radius",
    "inset-inline-start" => "right",
    "inset-inline-end" => "left",
};

pub static LOGICAL_VALUE_TO_RTL: phf::Map<&'static str, &'static str> = phf_map! {
    "start" => "right",
    "inline-start" => "right",
    "end" => "left",
    "inline-end" => "left",
};
