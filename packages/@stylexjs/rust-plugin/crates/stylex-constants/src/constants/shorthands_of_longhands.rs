use phf::phf_set;

// Using MDN data as a source of truth to populate the above sets
// by group in alphabetical order:

pub static SHORTHANDS_OF_LONGHANDS: phf::Set<&'static str> = phf_set! {
  // CSS Animations
  "animation-range",

  "scroll-timeline",


  "view-timeline",

  // CSS Backgrounds and Borders
  "background-position",

  "border-color",
  "border-style",
  "border-width",
  "border-block-start", // Logical Properties
  "border-top",
  "border-block-end", // Logical Properties
  "border-bottom",
  "border-inline-color", // Logical Properties
  "border-inline-style", // Logical Properties
  "border-inline-width", // Logical Properties
  "border-inline-start", // Logical Properties
  "border-left",
  "border-inline-end", // Logical Properties
  "border-right",

  "border-image",

  "border-radius",

  "corner-shape",

  // CSS Basic User Interface
  "caret",


  "outline",


  // CSS Box Alignment
  "grid-gap", // alias for `gap`
  "gap",

  "place-content",
  "place-items",
  "place-self",

  // CSS Box Model
  "margin-block", // Logical Properties
  "margin-inline", // Logical Properties


  "overscroll-behavior",

  "padding-block", // Logical Properties
  "padding-inline", // Logical Properties

  // CSS Columns
  "columns",

  "column-rule",

  // CSS Containment

  "contain-intrinsic-size",

  "container",

  // CSS Flexible Box Layout
  "flex",
  "flex-flow",

  // CSS Fonts
  "font-variant",

  // CSS Grid Layout
  "grid-template-areas",
  "grid-row",
  "grid-column",


  // CSS Images

  // CSS Inline

  // CSS Lists and Counters
  "list-style",

  // CSS Masking
  "mask",
  "mask-border",

  // CSS Motion Path
  "offset",

  // CSS Overflow
  "overflow",

  // CSS Positioning
  "inset-block", // Logical Properties
  "inset-inline", // Logical Properties

  // CSS Scroll Snap
  "scroll-margin-block",
  "scroll-margin-inline",

  "scroll-padding-block",
  "scroll-padding-inline",

  "scroll-snap-type",

  // CSS Text Decoration
  "text-decoration",

  "text-emphasis",

  // CSS Transitions
  "transition",
};
