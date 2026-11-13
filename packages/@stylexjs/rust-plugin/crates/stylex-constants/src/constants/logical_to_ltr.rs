use phf::phf_map;

// These properties are kept for a polyfill that is only used with
// `legacy-expand-shorthands`
pub static PROPERTY_TO_LTR: phf::Map<&'static str, &'static str> = phf_map! {
    "margin-start" => "margin-left",
    "margin-end" => "margin-right",
    "padding-start" => "padding-left",
    "padding-end" => "padding-right",
    "border-start" => "border-left",
    "border-end" => "border-right",
    "border-start-width" => "border-left-width",
    "border-end-width" => "border-right-width",
    "border-start-color" => "border-left-color",
    "border-end-color" => "border-right-color",
    "border-start-style" => "border-left-style",
    "border-end-style" => "border-right-style",
    "border-top-start-radius" => "border-top-left-radius",
    "border-bottom-start-radius" => "border-bottom-left-radius",
    "border-top-end-radius" => "border-top-right-radius",
    "border-bottom-end-radius" => "border-bottom-right-radius",
    "start" => "left",
    "end" => "right",

};

pub static INLINE_PROPERTY_TO_LTR: phf::Map<&'static str, &'static str> = phf_map! {
    "margin-inline-start" => "margin-left",
    "margin-inline-end" => "margin-right",
    "padding-inline-start" => "padding-left",
    "padding-inline-end" => "padding-right",
    "border-inline-start" => "border-left",
    "border-inline-end" => "border-right",
    "border-inline-start-width" => "border-left-width",
    "border-inline-end-width" => "border-right-width",
    "border-inline-start-color" => "border-left-color",
    "border-inline-end-color" => "border-right-color",
    "border-inline-start-style" => "border-left-style",
    "border-inline-end-style" => "border-right-style",
    "border-start-start-radius" => "border-top-left-radius",
    "border-end-start-radius" => "border-bottom-left-radius",
    "border-start-end-radius" => "border-top-right-radius",
    "border-end-end-radius" => "border-bottom-right-radius",
    "inset-inline-start" => "left",
    "inset-inline-end" => "right",
};
