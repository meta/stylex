use phf::phf_set;

// Using MDN data as a source of truth to populate the above sets
// by group in alphabetical order:

pub static LONG_HAND_PHYSICAL: phf::Set<&'static str> = phf_set! {
  "border-bottom-color",
  "border-bottom-style",
  "border-bottom-width",
  "border-left-color",
  "border-left-style",
  "border-left-width",
  "border-right-color",
  "border-right-style",
  "border-right-width",

  "border-top-color",
  "border-top-style",
  "border-top-width",

  "border-top-left-radius",
  "border-top-right-radius",
  "border-bottom-left-radius",
  "border-bottom-right-radius",

  "corner-top-left-shape",
  "corner-top-right-shape",
  "corner-bottom-left-shape",
  "corner-bottom-right-shape",

  "height",
  "width",

  "max-height",
  "max-width",
  "min-height",
  "min-width",

  "margin-top",
  "margin-bottom",
  "margin-left",
  "margin-right",

  "overscroll-behavior-y",
  "overscroll-behavior-x",

  "padding-top",
  "padding-bottom",
  "padding-left",
  "padding-right",

  "line-clamp",
  "max-lines",

  "overflow-y",
  "overflow-x",


  // CSS Positioning
  "top",
  "bottom",
  "left",
  "right",

  // CSS Scroll Snap
  "scroll-margin-top",
  "scroll-margin-bottom",
  "scroll-margin-left",
  "scroll-margin-right",

  "scroll-padding-top",
  "scroll-padding-bottom",
  "scroll-padding-left",
  "scroll-padding-right",
};
