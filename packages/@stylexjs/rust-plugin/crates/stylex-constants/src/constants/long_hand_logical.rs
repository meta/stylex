use phf::phf_set;

// Using MDN data as a source of truth to populate the above sets
// by group in alphabetical order:

pub static LONG_HAND_LOGICAL: phf::Set<&'static str> = phf_set! {
    // Composition and Blending
  "background-blend-mode",
  "isolation",
  "mix-blend-mode",

  // CSS Animations
  "animation-composition",
  "animation-delay",
  "animation-direction",
  "animation-duration",
  "animation-fill-mode",
  "animation-iteration-count",
  "animation-name",
  "animation-play-state",
  "animation-range-end",
  "animation-range-start",
  "animation-timing-function",
  "animation-timeline",

  "scroll-timeline-axis",
  "scroll-timeline-name",

  "timeline-scope",

  "view-timeline-axis",
  "view-timeline-inset",
  "view-timeline-name",

  // CSS Backgrounds and Borders
  "background-attachment",
  "background-clip",
  "background-color",
  "background-image",
  "background-origin",
  "background-repeat",
  "background-size",
  "background-position-x",
  "background-position-y",

  "border-block-color", // Logical Properties
  "border-block-stylex", // Logical Properties
  "border-block-width", // Logical Properties
  "border-block-start-color", // Logical Properties
  "border-block-start-style", // Logical Properties
  "border-block-start-width", // Logical Properties
  "border-block-end-color", // Logical Properties
  "border-block-end-style", // Logical Properties
  "border-block-end-width", // Logical Properties
  "border-inline-start-color", // Logical Properties
  "border-inline-start-style", // Logical Properties
  "border-inline-start-width", // Logical Properties
  "border-inline-end-color", // Logical Properties
  "border-inline-end-style", // Logical Properties
  "border-inline-end-width", // Logical Properties

  "border-image-outset",
  "border-image-repeat",
  "border-image-slice",
  "border-image-source",
  "border-image-width",

  "border-start-end-radius", // Logical Properties
  "border-start-start-radius", // Logical Properties
  "border-end-end-radius", // Logical Properties
  "border-end-start-radius", // Logical Properties

  "corner-start-start-shape", // Logical Properties
  "corner-start-end-shape", // Logical Properties
  "corner-end-start-shape", // Logical Properties
  "corner-end-end-shape", // Logical Properties

  "box-shadow",

  // CSS Basic User Interface
  "accent-color",
  "appearance",
  "aspect-ratio",

  "caret-color",
  "caret-shape",

  "cursor",
  "ime-mode",
  "input-security",

  "outline-color",
  "outline-offset",
  "outline-style",
  "outline-width",

  "pointer-events",
  "resize", // horizontal, vertical, block, inline, both
  "text-overflow",
  "user-select",

  // CSS Box Alignment
  "grid-row-gap", // alias for `row-gap`
  "row-gap",
  "grid-column-gap", // alias for `column-gap`
  "column-gap",

  "align-content",
  "justify-content",

  "align-items",
  "justify-items",

  "align-self",
  "justify-self",

  // CSS Box Model
  "box-sizing",

  "block-size", // Logical Properties
  "inline-size", // Logical Properties

  "max-block-size", // Logical Properties
  "max-inline-size", // Logical Properties
  "min-block-size", // Logical Properties
  "min-inline-size", // Logical Properties

  "margin-block-start", // Logical Properties
  "margin-block-end", // Logical Properties
  "margin-inline-start", // Logical Properties
  "margin-inline-end", // Logical Properties

  "margin-trim",

  "overscroll-behavior-block",
  "overscroll-behavior-inline",

  "padding-block-start", // Logical Properties
  "padding-block-end", // Logical Properties
  "padding-inline-start", // Logical Properties
  "padding-inline-end", // Logical Properties

  "visibility",

  // CSS Color
  "color",
  "color-scheme",
  "forced-color-adjust",
  "opacity",
  "print-color-adjust",

  // CSS Columns
  "column-count",
  "column-width",

  "column-fill",
  "column-span",

  "column-rule-color",
  "column-rule-style",
  "column-rule-width",

  // CSS Containment
  "contain",

  "contain-intrinsic-block-size",
  "contain-intrinsic-width",
  "contain-intrinsic-height",
  "contain-intrinsic-inline-size",

  "container-name",
  "container-type",

  "content-visibility",

  // CSS Counter Styles
  "counter-increment",
  "counter-reset",
  "counter-set",

  // CSS Display
  "display",

  // CSS Flexible Box Layout
  "flex-basis",
  "flex-grow",
  "flex-shrink",

  "flex-direction",
  "flex-wrap",

  "order",

  // CSS Fonts
  "font-family",
  "font-size",
  "font-stretch",
  "font-style",
  "font-weight",
  "line-height",
  "font-variant-alternates",
  "font-variant-caps",
  "font-variant-east-asian",
  "font-variant-emoji",
  "font-variant-ligatures",
  "font-variant-numeric",
  "font-variant-position",

  "font-feature-settings",
  "font-kerning",
  "font-language-override",
  "font-optical-sizing",
  "font-palette",
  "font-variation-settings",
  "font-size-adjust",
  "font-smooth", // Non-standard
  "font-synthesis-position",
  "font-synthesis-small-caps",
  "font-synthesis-style",
  "font-synthesis-weight",

  "line-height-step",

  // CSS Fragmentation
  "box-decoration-break",
  "break-after",
  "break-before",
  "break-inside",
  "orphans",
  "widows",

  // CSS Generated Content
  "content",
  "quotes",

  // CSS Grid Layout
  "grid-auto-flow",
  "grid-auto-rows",
  "grid-auto-columns",
  "grid-template-columns",
  "grid-template-rows",

  "grid-row-start",
  "grid-row-end",
  "grid-column-start",
  "grid-column-end",

  "align-tracks",
  "justify-tracks",
  "masonry-auto-flow",

  // CSS Images
  "image-orientation",
  "image-rendering",
  "image-resolution",
  "object-fit",
  "object-position",

  // CSS Inline
  "initial-letter",
  "initial-letter-align",

  // CSS Lists and Counters
  "list-style-image",
  "list-style-position",
  "list-style-type",

  // CSS Masking
  "clip", // @deprecated
  "clip-path",

  "mask-clip",
  "mask-composite",
  "mask-image",
  "mask-mode",
  "mask-origin",
  "mask-position",
  "mask-repeat",
  "mask-size",

  "mask-type",

  "mask-border-mode",
  "mask-border-outset",
  "mask-border-repeat",
  "mask-border-slice",
  "mask-border-source",
  "mask-border-width",

  // CSS Miscellaneous
  "text-rendering",

  // CSS Motion Path
  "offset-anchor",
  "offset-distance",
  "offset-path",
  "offset-position",
  "offset-rotate",

  // CSS Overflow
  "-webkit-box-orient",
  "-webkit-line-clamp",
  // "block-overflow",

  "overflow-block",
  "overflow-inline",

  "overflow-clip-margin", // partial support

  "scroll-gutter",
  "scroll-behavior",

  // CSS Pages
  "page",
  "page-break-after",
  "page-break-before",
  "page-break-inside",

  // CSS Positioning
  "inset-block-start", // Logical Properties
  "inset-block-end", // Logical Properties
  "inset-inline-start", // Logical Properties
  "inset-inline-end", // Logical Properties

  "clear",
  "float",
  // "overlay",
  "position",
  "z-index",

  // CSS Ruby
  "ruby-align",
  "ruby-merge",
  "ruby-position",

  // CSS Scroll Anchoring
  "overflow-anchor",

  // CSS Scroll Snap
  "scroll-margin-block-start",
  "scroll-margin-block-end",
  "scroll-margin-inline-start",
  "scroll-margin-inline-end",

  "scroll-padding-block-start",
  "scroll-padding-block-end",
  "scroll-padding-inline-start",
  "scroll-padding-inline-end",

  "scroll-snap-align",
  // "scroll-snap-coordinate",
  // "scroll-snap-destination",
  // "scroll-snap-points-x",
  // "scroll-snap-points-y",
  "scroll-snap-stop",
  // "scroll-snap-type-x",
  // "scroll-snap-type-y",

  // CSS Scrollbars
  "scrollbar-color",
  "scrollbar-width",

  // CSS Shapes
  "shape-image-threshold",
  "shape-margin",
  "shape-outside",

  // CSS Speech
  "azimuth",

  // CSS Table
  "border-collapse",
  "border-spacing",
  "caption-side",
  "empty-cells",
  "table-layout",
  "vertical-align",

  // CSS Text Decoration
  "text-decoration-color",
  "text-decoration-line",
  "text-decoration-skip",
  "text-decoration-skip-ink",
  "text-decoration-style",
  "text-decoration-thickness",

  "text-emphasis-color",
  "text-emphasis-position",
  "text-emphasis-style",
  "text-shadow",
  "text-underline-offset",
  "text-underline-position",

  // CSS Text
  "hanging-punctuation",
  "hyphenate-character",
  "hyphenate-limit-chars",
  "hyphens",
  "letter-spacing",
  "line-break",
  "overflow-wrap",
  "paint-order",
  "tab-size",
  "text-align",
  "text-align-last",
  "text-indent",
  "text-justify",
  "text-size-adjust",
  "text-transform",
  "text-wrap",
  "white-space",
  "white-space-collapse",
  // "white-space-trim",
  "word-break",
  "word-spacing",
  "word-wrap",

  // CSS Transforms
  "backface-visibility",
  "perspective",
  "perspective-origin",
  "rotate",
  "scale",
  "transform",
  "transform-box",
  "transform-origin",
  "transform-style",
  "translate",

  // CSS Transitions
  // "transition-behavior",
  "transition-delay",
  "transition-duration",
  "transition-property",
  "transition-timing-function",

  // CSS View Transitions
  "view-transition-name",

  // CSS Will Change
  "will-change",

  // CSS Writing Modes
  "direction",
  "text-combine-upright",
  "text-orientation",
  "unicode-bidi",
  "writing-mode",

  // CSS Filter Effects
  "backdrop-filter",
  "filter",

  // MathML
  "math-depth",
  "math-shift",
  "math-style",

  // CSS Pointer Events
  "touch-action"
};
