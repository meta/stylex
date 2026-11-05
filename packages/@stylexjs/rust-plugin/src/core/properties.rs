//! Property contracts from current Babel preprocess-rules and shared priorities.
use super::{CreateOptions, StyleValue};

/// Ordered application-order cancellation slots. Alias wrappers that only
/// rename their value deliberately do not recursively acquire new slots.
pub fn shorthand_cancellations(key: &str) -> &'static [&'static str] {
    match key {
        "animation" => &[
            "animationComposition",
            "animationName",
            "animationDuration",
            "animationTimingFunction",
            "animationDelay",
            "animationIterationCount",
            "animationDirection",
            "animationFillMode",
            "animationPlayState",
            "animationRange",
            "animationRangeEnd",
            "animationRangeStart",
            "animationTimeline",
        ],
        "animationRange" => &["animationRangeEnd", "animationRangeStart"],
        "background" => &[
            "backgroundAttachment",
            "backgroundClip",
            "backgroundColor",
            "backgroundImage",
            "backgroundOrigin",
            "backgroundPosition",
            "backgroundPositionX",
            "backgroundPositionY",
            "backgroundRepeat",
            "backgroundSize",
        ],
        "backgroundPosition" => &["backgroundPositionX", "backgroundPositionY"],
        "border" => &[
            "borderWidth",
            "borderInlineWidth",
            "borderInlineStartWidth",
            "borderLeftWidth",
            "borderInlineEndWidth",
            "borderRightWidth",
            "borderBlockWidth",
            "borderTopWidth",
            "borderBottomWidth",
            "borderStyle",
            "borderInlineStyle",
            "borderInlineStartStyle",
            "borderLeftStyle",
            "borderInlineEndStyle",
            "borderRightStyle",
            "borderBlockStyle",
            "borderTopStyle",
            "borderBottomStyle",
            "borderColor",
            "borderInlineColor",
            "borderInlineStartColor",
            "borderLeftColor",
            "borderInlineEndColor",
            "borderRightColor",
            "borderBlockColor",
            "borderTopColor",
            "borderBottomColor",
        ],
        "borderInline" => &[
            "borderInlineWidth",
            "borderInlineStartWidth",
            "borderLeftWidth",
            "borderInlineEndWidth",
            "borderRightWidth",
            "borderInlineStyle",
            "borderInlineStartStyle",
            "borderLeftStyle",
            "borderInlineEndStyle",
            "borderRightStyle",
            "borderInlineColor",
            "borderInlineStartColor",
            "borderLeftColor",
            "borderInlineEndColor",
            "borderRightColor",
        ],
        "borderBlock" => &[
            "borderBlockWidth",
            "borderTopWidth",
            "borderBottomWidth",
            "borderBlockStyle",
            "borderTopStyle",
            "borderBottomStyle",
            "borderBlockColor",
            "borderTopColor",
            "borderBottomColor",
        ],
        "borderTop" => &["borderTopWidth", "borderTopStyle", "borderTopColor"],
        "borderBottom" => &[
            "borderBottomWidth",
            "borderBottomStyle",
            "borderBottomColor",
        ],
        "borderInlineStart" => &[
            "borderInlineStartWidth",
            "borderLeftWidth",
            "borderRightWidth",
            "borderInlineStartStyle",
            "borderLeftStyle",
            "borderRightStyle",
            "borderInlineStartColor",
            "borderLeftColor",
            "borderRightColor",
        ],
        "borderInlineEnd" => &[
            "borderInlineEndWidth",
            "borderLeftWidth",
            "borderRightWidth",
            "borderInlineEndStyle",
            "borderLeftStyle",
            "borderRightStyle",
            "borderInlineEndColor",
            "borderLeftColor",
            "borderRightColor",
        ],
        "borderLeft" => &[
            "borderLeftWidth",
            "borderInlineStartWidth",
            "borderInlineEndWidth",
            "borderLeftStyle",
            "borderInlineStartStyle",
            "borderInlineEndStyle",
            "borderLeftColor",
            "borderInlineStartColor",
            "borderInlineEndColor",
        ],
        "borderRight" => &[
            "borderRightWidth",
            "borderInlineStartWidth",
            "borderInlineEndWidth",
            "borderRightStyle",
            "borderInlineStartStyle",
            "borderInlineEndStyle",
            "borderRightColor",
            "borderInlineStartColor",
            "borderInlineEndColor",
        ],
        "borderInlineWidth" => &[
            "borderInlineStartWidth",
            "borderLeftWidth",
            "borderInlineEndWidth",
            "borderRightWidth",
        ],
        "borderInlineStyle" => &[
            "borderInlineStartStyle",
            "borderLeftStyle",
            "borderInlineEndStyle",
            "borderRightStyle",
        ],
        "borderInlineColor" => &[
            "borderInlineStartColor",
            "borderLeftColor",
            "borderInlineEndColor",
            "borderRightColor",
        ],
        "borderBlockWidth" => &["borderTopWidth", "borderBottomWidth"],
        "borderBlockStyle" => &["borderTopStyle", "borderBottomStyle"],
        "borderBlockColor" => &["borderTopColor", "borderBottomColor"],
        "borderColor" => &[
            "borderInlineColor",
            "borderInlineStartColor",
            "borderLeftColor",
            "borderInlineEndColor",
            "borderRightColor",
            "borderBlockColor",
            "borderTopColor",
            "borderBottomColor",
        ],
        "borderStyle" => &[
            "borderInlineStyle",
            "borderInlineStartStyle",
            "borderLeftStyle",
            "borderInlineEndStyle",
            "borderRightStyle",
            "borderBlockStyle",
            "borderTopStyle",
            "borderBottomStyle",
        ],
        "borderWidth" => &[
            "borderInlineWidth",
            "borderInlineStartWidth",
            "borderLeftWidth",
            "borderInlineEndWidth",
            "borderRightWidth",
            "borderBlockWidth",
            "borderTopWidth",
            "borderBottomWidth",
        ],
        "borderInlineStartColor" | "borderInlineEndColor" => {
            &["borderLeftColor", "borderRightColor"]
        }
        "borderInlineStartStyle" | "borderInlineEndStyle" => {
            &["borderLeftStyle", "borderRightStyle"]
        }
        "borderInlineStartWidth" | "borderInlineEndWidth" => {
            &["borderLeftWidth", "borderRightWidth"]
        }
        "borderLeftColor" | "borderRightColor" => {
            &["borderInlineStartColor", "borderInlineEndColor"]
        }
        "borderLeftStyle" | "borderRightStyle" => {
            &["borderInlineStartStyle", "borderInlineEndStyle"]
        }
        "borderLeftWidth" | "borderRightWidth" => {
            &["borderInlineStartWidth", "borderInlineEndWidth"]
        }
        "borderRadius" => &[
            "borderStartStartRadius",
            "borderStartEndRadius",
            "borderEndStartRadius",
            "borderEndEndRadius",
            "borderTopLeftRadius",
            "borderTopRightRadius",
            "borderBottomLeftRadius",
            "borderBottomRightRadius",
        ],
        "borderStartStartRadius" | "borderStartEndRadius" => {
            &["borderTopLeftRadius", "borderTopRightRadius"]
        }
        "borderEndStartRadius" | "borderEndEndRadius" => {
            &["borderBottomLeftRadius", "borderBottomRightRadius"]
        }
        "borderTopLeftRadius" | "borderTopRightRadius" => {
            &["borderStartStartRadius", "borderStartEndRadius"]
        }
        "borderBottomLeftRadius" | "borderBottomRightRadius" => {
            &["borderEndStartRadius", "borderEndEndRadius"]
        }
        "cornerShape" => &[
            "cornerStartStartShape",
            "cornerStartEndShape",
            "cornerEndStartShape",
            "cornerEndEndShape",
            "cornerTopLeftShape",
            "cornerTopRightShape",
            "cornerBottomLeftShape",
            "cornerBottomRightShape",
        ],
        "cornerStartStartShape" | "cornerStartEndShape" => {
            &["cornerTopLeftShape", "cornerTopRightShape"]
        }
        "cornerEndStartShape" | "cornerEndEndShape" => {
            &["cornerBottomLeftShape", "cornerBottomRightShape"]
        }
        "cornerTopLeftShape" | "cornerTopRightShape" => {
            &["cornerStartStartShape", "cornerStartEndShape"]
        }
        "cornerBottomLeftShape" | "cornerBottomRightShape" => {
            &["cornerEndStartShape", "cornerEndEndShape"]
        }
        "borderImage" => &[
            "borderImageOutset",
            "borderImageRepeat",
            "borderImageSlice",
            "borderImageSource",
            "borderImageWidth",
        ],
        "columnRule" => &["columnRuleColor", "columnRuleStyle", "columnRuleWidth"],
        "columns" => &["columnCount", "columnWidth"],
        "container" => &["containerName", "containerType"],
        "containIntrinsicSize" => &["containIntrinsicWidth", "containIntrinsicHeight"],
        "flex" => &["flexGrow", "flexShrink", "flexBasis"],
        "flexFlow" => &["flexDirection", "flexWrap"],
        "font" => &[
            "fontFamily",
            "fontSize",
            "fontStretch",
            "fontStyle",
            "fontVariant",
            "fontVariantAlternates",
            "fontVariantCaps",
            "fontVariantEastAsian",
            "fontVariantEmoji",
            "fontVariantLigatures",
            "fontVariantNumeric",
            "fontVariantPosition",
            "fontWeight",
            "lineHeight",
        ],
        "fontVariant" => &[
            "fontVariantAlternates",
            "fontVariantCaps",
            "fontVariantEastAsian",
            "fontVariantEmoji",
            "fontVariantLigatures",
            "fontVariantNumeric",
            "fontVariantPosition",
        ],
        "gap" => &["rowGap", "columnGap"],
        "grid" => &[
            "gridTemplate",
            "gridTemplateAreas",
            "gridTemplateColumns",
            "gridTemplateRows",
            "gridAutoRows",
            "gridAutoColumns",
            "gridAutoFlow",
        ],
        "gridArea" => &[
            "gridRow",
            "gridRowStart",
            "gridRowEnd",
            "gridColumn",
            "gridColumnStart",
            "gridColumnEnd",
        ],
        "gridRow" => &["gridRowStart", "gridRowEnd"],
        "gridColumn" => &["gridColumnStart", "gridColumnEnd"],
        "gridTemplate" => &[
            "gridTemplateAreas",
            "gridTemplateColumns",
            "gridTemplateRows",
        ],
        "inset" => &[
            "insetInline",
            "insetInlineStart",
            "insetInlineEnd",
            "left",
            "right",
            "insetBlock",
            "top",
            "bottom",
        ],
        "insetInline" => &["insetInlineStart", "insetInlineEnd", "left", "right"],
        "insetBlock" => &["top", "bottom"],
        "insetInlineStart" | "insetInlineEnd" => &["left", "right"],
        "left" | "right" => &["insetInlineStart", "insetInlineEnd"],
        "listStyle" => &["listStyleImage", "listStylePosition", "listStyleType"],
        "margin" => &[
            "marginInline",
            "marginInlineStart",
            "marginLeft",
            "marginInlineEnd",
            "marginRight",
            "marginBlock",
            "marginTop",
            "marginBottom",
        ],
        "marginInline" => &[
            "marginInlineStart",
            "marginLeft",
            "marginInlineEnd",
            "marginRight",
        ],
        "marginBlock" => &["marginTop", "marginBottom"],
        "marginInlineStart" | "marginInlineEnd" => &["marginLeft", "marginRight"],
        "marginLeft" | "marginRight" => &["marginInlineStart", "marginInlineEnd"],
        "mask" => &[
            "maskClip",
            "maskComposite",
            "maskImage",
            "maskMode",
            "maskOrigin",
            "maskPosition",
            "maskRepeat",
            "maskSize",
        ],
        "maskBorder" => &[
            "maskBorderMode",
            "maskBorderOutset",
            "maskBorderRepeat",
            "maskBorderSlice",
            "maskBorderSource",
            "maskBorderWidth",
        ],
        "offset" => &[
            "offsetAnchor",
            "offsetDistance",
            "offsetPath",
            "offsetPosition",
            "offsetRotate",
        ],
        "outline" => &[
            "outlineColor",
            "outlineOffset",
            "outlineStyle",
            "outlineWidth",
        ],
        "overflow" => &["overflowX", "overflowY"],
        "padding" => &[
            "paddingInline",
            "paddingStart",
            "paddingLeft",
            "paddingEnd",
            "paddingRight",
            "paddingBlock",
            "paddingTop",
            "paddingBottom",
        ],
        "paddingInline" => &["paddingStart", "paddingLeft", "paddingEnd", "paddingRight"],
        "paddingBlock" => &["paddingTop", "paddingBottom"],
        "paddingInlineStart" | "paddingInlineEnd" => &["paddingLeft", "paddingRight"],
        "paddingLeft" | "paddingRight" => &["paddingInlineStart", "paddingInlineEnd"],
        "placeContent" => &["alignContent", "justifyContent"],
        "placeItems" => &["alignItems", "justifyItems"],
        "placeSelf" => &["alignSelf", "justifySelf"],
        "scrollMargin" => &[
            "scrollMarginBlock",
            "scrollMarginTop",
            "scrollMarginBottom",
            "scrollMarginInline",
            "scrollMarginInlineStart",
            "scrollMarginInlineEnd",
            "scrollMarginLeft",
            "scrollMarginRight",
        ],
        "scrollMarginBlock" => &["scrollMarginTop", "scrollMarginBottom"],
        "scrollMarginInline" => &[
            "scrollMarginInlineStart",
            "scrollMarginInlineEnd",
            "scrollMarginLeft",
            "scrollMarginRight",
        ],
        "scrollMarginInlineStart" | "scrollMarginInlineEnd" => {
            &["scrollMarginLeft", "scrollMarginRight"]
        }
        "scrollMarginLeft" | "scrollMarginRight" => {
            &["scrollMarginInlineStart", "scrollMarginInlineEnd"]
        }
        "scrollPadding" => &[
            "scrollPaddingBlock",
            "scrollPaddingTop",
            "scrollPaddingBottom",
            "scrollPaddingInline",
            "scrollPaddingInlineStart",
            "scrollPaddingInlineEnd",
            "scrollPaddingLeft",
            "scrollPaddingRight",
        ],
        "scrollPaddingBlock" => &["scrollPaddingTop", "scrollPaddingBottom"],
        "scrollPaddingInline" => &[
            "scrollPaddingInlineStart",
            "scrollPaddingInlineEnd",
            "scrollPaddingLeft",
            "scrollPaddingRight",
        ],
        "scrollPaddingInlineStart" | "scrollPaddingInlineEnd" => {
            &["scrollPaddingLeft", "scrollPaddingRight"]
        }
        "scrollPaddingLeft" | "scrollPaddingRight" => {
            &["scrollPaddingInlineStart", "scrollPaddingInlineEnd"]
        }
        "scrollSnapType" => &["scrollSnapTypeX", "scrollSnapTypeY"],
        "scrollTimeline" => &["scrollTimelineName", "scrollTimelineAxis"],
        "textDecoration" => &[
            "textDecorationColor",
            "textDecorationLine",
            "textDecorationStyle",
            "textDecorationThickness",
        ],
        "textEmphasis" => &["textEmphasisColor", "textEmphasisStyle"],
        "transition" => &[
            "transitionBehavior",
            "transitionDelay",
            "transitionDuration",
            "transitionProperty",
            "transitionTimingFunction",
        ],
        "borderHorizontal"
        | "borderVertical"
        | "borderBlockStart"
        | "borderEnd"
        | "borderBlockEnd"
        | "borderStart"
        | "borderHorizontalWidth"
        | "borderHorizontalStyle"
        | "borderHorizontalColor"
        | "borderVerticalWidth"
        | "borderVerticalStyle"
        | "borderVerticalColor"
        | "borderStartColor"
        | "borderEndColor"
        | "borderStartStyle"
        | "borderEndStyle"
        | "borderStartWidth"
        | "borderEndWidth"
        | "gridGap"
        | "marginStart"
        | "marginEnd"
        | "marginHorizontal"
        | "marginVertical"
        | "paddingStart"
        | "paddingEnd"
        | "paddingHorizontal"
        | "paddingVertical"
        | "start"
        | "end" => shorthand_cancellations(&canonical_property_name(key)),
        _ => &[],
    }
}

pub fn canonical_property_name(key: &str) -> String {
    match key {
        "borderHorizontal" => "borderInline",
        "borderVertical" => "borderBlock",
        "borderBlockStart" => "borderTop",
        "borderBlockEnd" => "borderBottom",
        "borderStart" => "borderInlineStart",
        "borderEnd" => "borderInlineEnd",
        "blockSize" => "height",
        "inlineSize" => "width",
        "minBlockSize" => "minHeight",
        "minInlineSize" => "minWidth",
        "maxBlockSize" => "maxHeight",
        "maxInlineSize" => "maxWidth",
        "borderHorizontalWidth" => "borderInlineWidth",
        "borderHorizontalStyle" => "borderInlineStyle",
        "borderHorizontalColor" => "borderInlineColor",
        "borderVerticalWidth" => "borderBlockWidth",
        "borderVerticalStyle" => "borderBlockStyle",
        "borderVerticalColor" => "borderBlockColor",
        "borderBlockStartColor" => "borderTopColor",
        "borderBlockEndColor" => "borderBottomColor",
        "borderBlockStartStyle" => "borderTopStyle",
        "borderBlockEndStyle" => "borderBottomStyle",
        "borderBlockStartWidth" => "borderTopWidth",
        "borderBlockEndWidth" => "borderBottomWidth",
        "borderStartColor" => "borderInlineStartColor",
        "borderEndColor" => "borderInlineEndColor",
        "borderStartStyle" => "borderInlineStartStyle",
        "borderEndStyle" => "borderInlineEndStyle",
        "borderStartWidth" => "borderInlineStartWidth",
        "borderEndWidth" => "borderInlineEndWidth",
        "borderTopStartRadius" => "borderStartStartRadius",
        "borderTopEndRadius" => "borderStartEndRadius",
        "borderBottomStartRadius" => "borderEndStartRadius",
        "borderBottomEndRadius" => "borderEndEndRadius",
        "containIntrinsicBlockSize" => "containIntrinsicHeight",
        "containIntrinsicInlineSize" => "containIntrinsicWidth",
        "gridGap" => "gap",
        "gridRowGap" => "rowGap",
        "gridColumnGap" => "columnGap",
        "marginBlockStart" => "marginTop",
        "marginBlockEnd" => "marginBottom",
        "marginStart" => "marginInlineStart",
        "marginEnd" => "marginInlineEnd",
        "marginHorizontal" => "marginInline",
        "marginVertical" => "marginBlock",
        "overflowBlock" => "overflowY",
        "overflowInline" => "overflowX",
        "paddingBlockStart" => "paddingTop",
        "paddingBlockEnd" => "paddingBottom",
        "paddingStart" => "paddingInlineStart",
        "paddingEnd" => "paddingInlineEnd",
        "paddingHorizontal" => "paddingInline",
        "paddingVertical" => "paddingBlock",
        "scrollMarginBlockStart" => "scrollMarginTop",
        "scrollMarginBlockEnd" => "scrollMarginBottom",
        "insetBlockStart" => "top",
        "insetBlockEnd" => "bottom",
        "start" => "insetInlineStart",
        "end" => "insetInlineEnd",
        _ => key,
    }
    .to_owned()
}

pub(crate) fn expand_property(
    key: &str,
    value: &StyleValue,
    options: &CreateOptions,
) -> Result<Vec<(String, StyleValue)>, String> {
    let error = match (options.style_resolution.as_str(), key) {
        (_, "all") => Some("all is not supported"),
        ("property-specificity", "animation") => Some("animation is not supported"),
        ("property-specificity", "background") => Some("background is not supported. Use background-color, border-image etc. instead."),
        ("property-specificity", "border") => Some("border is not supported. Use border-width, border-style and border-color instead."),
        ("property-specificity", "borderInline" | "borderHorizontal") => Some("borderInline is not supported. Use borderInlineWidth, borderInlineStyle and borderInlineColor instead."),
        ("property-specificity", "borderBlock" | "borderVertical") => Some("borderBlock is not supported. Use borderBlockWidth, borderBlockStyle and borderBlockColor instead."),
        ("property-specificity", "borderTop" | "borderBlockStart") => Some("borderTop is not supported. Use borderTopWidth, borderTopStyle and borderTopColor instead."),
        ("property-specificity", "borderInlineEnd" | "borderEnd") => Some("borderInlineEnd is not supported. Use borderInlineEndWidth, borderInlineEndStyle and borderInlineEndColor instead."),
        ("property-specificity", "borderRight") => Some("borderRight is not supported. Use borderRightWidth, borderRightStyle and borderRightColor instead."),
        ("property-specificity", "borderBottom" | "borderBlockEnd") => Some("borderBottom is not supported. Use borderBottomWidth, borderBottomStyle and borderBottomColor instead."),
        ("property-specificity", "borderInlineStart" | "borderStart") => Some("borderInlineStart is not supported. Use borderInlineStartWidth, borderInlineStartStyle and borderInlineStartColor instead."),
        ("property-specificity", "borderLeft") => Some("`borderLeft` is not supported. You could use `borderLeftWidth`, `borderLeftStyle` and `borderLeftColor`, but it is preferable to use `borderInlineStartWidth`, `borderInlineStartStyle` and `borderInlineStartColor`."),
        _ => None,
    };
    if let Some(error) = error {
        return if options.property_validation_mode == "throw" {
            Err(error.to_owned())
        } else {
            Ok(Vec::new())
        };
    }
    // property-specificity has no grid gap aliases; the legacy names are kept.
    let name = match (options.style_resolution.as_str(), key) {
        ("property-specificity", "gridGap" | "gridRowGap" | "gridColumnGap") => key.to_owned(),
        _ => canonical_property_name(key),
    };
    let mut result = vec![(name, value.clone())];
    if options.style_resolution == "application-order" {
        result.extend(
            shorthand_cancellations(key)
                .iter()
                .map(|key| ((*key).to_owned(), StyleValue::Null)),
        );
    }
    Ok(result)
}

pub(crate) fn default_priority(key: &str) -> f64 {
    match key {
        "animation" | "background" | "border" | "border-block" | "border-inline" | "margin"
        | "padding" | "font" | "grid" | "grid-template" | "grid-area" | "all" | "inset"
        | "scroll-margin" | "scroll-padding" => 1000.0,
        "animation-range"
        | "scroll-timeline"
        | "view-timeline"
        | "background-position"
        | "border-color"
        | "border-style"
        | "border-width"
        | "border-block-start"
        | "border-top"
        | "border-block-end"
        | "border-bottom"
        | "border-inline-color"
        | "border-inline-style"
        | "border-inline-width"
        | "border-inline-start"
        | "border-left"
        | "border-inline-end"
        | "border-right"
        | "border-image"
        | "border-radius"
        | "corner-shape"
        | "caret"
        | "outline"
        | "grid-gap"
        | "gap"
        | "place-content"
        | "place-items"
        | "place-self"
        | "margin-block"
        | "margin-inline"
        | "overscroll-behavior"
        | "padding-block"
        | "padding-inline"
        | "columns"
        | "column-rule"
        | "contain-intrinsic-size"
        | "container"
        | "flex"
        | "flex-flow"
        | "font-variant"
        | "grid-template-areas"
        | "grid-row"
        | "grid-column"
        | "list-style"
        | "mask"
        | "mask-border"
        | "offset"
        | "overflow"
        | "inset-block"
        | "inset-inline"
        | "scroll-margin-block"
        | "scroll-margin-inline"
        | "scroll-padding-block"
        | "scroll-padding-inline"
        | "scroll-snap-type"
        | "text-decoration"
        | "text-emphasis"
        | "transition" => 2000.0,
        "border-top-color"
        | "border-top-style"
        | "border-top-width"
        | "border-bottom-color"
        | "border-bottom-style"
        | "border-bottom-width"
        | "border-left-color"
        | "border-left-style"
        | "border-left-width"
        | "border-right-color"
        | "border-right-style"
        | "border-right-width"
        | "border-top-left-radius"
        | "border-top-right-radius"
        | "border-bottom-left-radius"
        | "border-bottom-right-radius"
        | "corner-top-left-shape"
        | "corner-top-right-shape"
        | "corner-bottom-left-shape"
        | "corner-bottom-right-shape"
        | "height"
        | "width"
        | "max-height"
        | "max-width"
        | "min-height"
        | "min-width"
        | "margin-top"
        | "margin-bottom"
        | "margin-left"
        | "margin-right"
        | "overscroll-behavior-y"
        | "overscroll-behavior-x"
        | "padding-top"
        | "padding-bottom"
        | "padding-left"
        | "padding-right"
        | "overflow-y"
        | "overflow-x"
        | "top"
        | "bottom"
        | "left"
        | "right"
        | "scroll-margin-top"
        | "scroll-margin-bottom"
        | "scroll-margin-left"
        | "scroll-margin-right"
        | "scroll-padding-top"
        | "scroll-padding-bottom"
        | "scroll-padding-left"
        | "scroll-padding-right" => 4000.0,
        _ => 3000.0,
    }
}

pub(crate) fn pseudo_priority(key: &str) -> f64 {
    match key {
        ":is" | ":where" | ":not" => 40.0,
        ":has" => 45.0,
        ":dir" => 50.0,
        ":lang" => 51.0,
        ":first-child" => 52.0,
        ":first-of-type" => 53.0,
        ":last-child" => 54.0,
        ":last-of-type" => 55.0,
        ":only-child" => 56.0,
        ":only-of-type" => 57.0,
        ":nth-child" => 60.0,
        ":nth-last-child" => 61.0,
        ":nth-of-type" => 62.0,
        ":nth-last-of-type" => 63.0,
        ":empty" => 70.0,
        ":link" => 80.0,
        ":any-link" => 81.0,
        ":local-link" => 82.0,
        ":target-within" => 83.0,
        ":target" => 84.0,
        ":visited" => 85.0,
        ":enabled" => 91.0,
        ":disabled" => 92.0,
        ":required" => 93.0,
        ":optional" => 94.0,
        ":read-only" => 95.0,
        ":read-write" => 96.0,
        ":placeholder-shown" => 97.0,
        ":in-range" => 98.0,
        ":out-of-range" => 99.0,
        ":default" => 100.0,
        ":checked" | ":indeterminate" => 101.0,
        ":blank" => 102.0,
        ":valid" => 103.0,
        ":invalid" => 104.0,
        ":user-invalid" => 105.0,
        ":autofill" => 110.0,
        ":picture-in-picture" => 120.0,
        ":modal" => 121.0,
        ":fullscreen" => 122.0,
        ":paused" => 123.0,
        ":playing" => 124.0,
        ":current" => 125.0,
        ":past" => 126.0,
        ":future" => 127.0,
        ":hover" => 130.0,
        ":focusWithin" => 140.0,
        ":focus" => 150.0,
        ":focusVisible" => 160.0,
        ":active" => 170.0,
        _ => 40.0,
    }
}
