use phf::phf_set;

// Using MDN data as a source of truth to populate the above sets
// by group in alphabetical order:

pub static SHORTHANDS_OF_SHORTHANDS: phf::Set<&'static str> = phf_set! {
// CSS Animations
"animation",

// CSS Backgrounds and Borders
"background",

"border", // OF SHORTHANDS!
"border-block", // Logical Properties
"border-inline", // Logical Properties

// CSS Box Model
"margin",

"padding",

// CSS Fonts
"font",

// CSS Grid Layout
"grid",
"grid-template",
"grid-area",

// CSS Miscellaneous
"all", // avoid!

// CSS Positioning
"inset", // Logical Properties

// CSS Scroll Snap
"scroll-margin",

"scroll-padding",
};
