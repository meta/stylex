//! Media query parse trees, ported from style-value-parser's media-query tests. Each case is
//! an input, the `Debug` form of its parsed rule and its printed form.

use crate::at_queries::media_query::MediaQuery;

const PARSED: &[(&str, &str, &str)] = &[
  (
    "@media (color-gamut: p3)",
    r#"Pair(MediaRulePair { type: "pair", key: "color-gamut", value: String("p3") })"#,
    "@media (color-gamut: p3)",
  ),
  (
    "@media (color-gamut: rec2020)",
    r#"Pair(MediaRulePair { type: "pair", key: "color-gamut", value: String("rec2020") })"#,
    "@media (color-gamut: rec2020)",
  ),
  (
    "@media (color-gamut: srgb)",
    r#"Pair(MediaRulePair { type: "pair", key: "color-gamut", value: String("srgb") })"#,
    "@media (color-gamut: srgb)",
  ),
  (
    "@media (color-index: 256)",
    r#"Pair(MediaRulePair { type: "pair", key: "color-index", value: Number(256.0) })"#,
    "@media (color-index: 256)",
  ),
  (
    "@media (max-color-index: 65536)",
    r#"Pair(MediaRulePair { type: "pair", key: "max-color-index", value: Number(65536.0) })"#,
    "@media (max-color-index: 65536)",
  ),
  (
    "@media (min-color-index: 256)",
    r#"Pair(MediaRulePair { type: "pair", key: "min-color-index", value: Number(256.0) })"#,
    "@media (min-color-index: 256)",
  ),
  (
    "@media (resolution: 1.5dpcm)",
    r#"Pair(MediaRulePair { type: "pair", key: "resolution", value: Length(Length { value: 1.5, unit: "dpcm" }) })"#,
    "@media (resolution: 1.5dpcm)",
  ),
  (
    "@media (resolution: 2dppx)",
    r#"Pair(MediaRulePair { type: "pair", key: "resolution", value: Length(Length { value: 2.0, unit: "dppx" }) })"#,
    "@media (resolution: 2dppx)",
  ),
  (
    "@media (resolution: 300dpi)",
    r#"Pair(MediaRulePair { type: "pair", key: "resolution", value: Length(Length { value: 300.0, unit: "dpi" }) })"#,
    "@media (resolution: 300dpi)",
  ),
  (
    "@media (video-color-gamut: p3)",
    r#"Pair(MediaRulePair { type: "pair", key: "video-color-gamut", value: String("p3") })"#,
    "@media (video-color-gamut: p3)",
  ),
  (
    "@media (video-color-gamut: rec2020)",
    r#"Pair(MediaRulePair { type: "pair", key: "video-color-gamut", value: String("rec2020") })"#,
    "@media (video-color-gamut: rec2020)",
  ),
  (
    "@media (video-color-gamut: srgb)",
    r#"Pair(MediaRulePair { type: "pair", key: "video-color-gamut", value: String("srgb") })"#,
    "@media (video-color-gamut: srgb)",
  ),
  (
    "@media (-webkit-max-device-pixel-ratio: 1.5)",
    r#"Pair(MediaRulePair { type: "pair", key: "-webkit-max-device-pixel-ratio", value: Number(1.5) })"#,
    "@media (-webkit-max-device-pixel-ratio: 1.5)",
  ),
  (
    "@media (-webkit-min-device-pixel-ratio: 2)",
    r#"Pair(MediaRulePair { type: "pair", key: "-webkit-min-device-pixel-ratio", value: Number(2.0) })"#,
    "@media (-webkit-min-device-pixel-ratio: 2)",
  ),
  (
    "@media screen, print",
    r#"Or(MediaOrRules { type: "or", rules: [MediaKeyword(MediaKeyword { type: "media-keyword", key: "screen", not: false, only: false }), MediaKeyword(MediaKeyword { type: "media-keyword", key: "print", not: false, only: false })] })"#,
    "@media screen, print",
  ),
  (
    "@media screen and (min-width: 768px) and (max-width: 1024px)",
    r#"And(MediaAndRules { type: "and", rules: [MediaKeyword(MediaKeyword { type: "media-keyword", key: "screen", not: false, only: false }), Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 768.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 1024.0, unit: "px" }) })] })"#,
    "@media screen and (min-width: 768px) and (max-width: 1024px)",
  ),
  (
    "@media (min-width: 768px)",
    r#"Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 768.0, unit: "px" }) })"#,
    "@media (min-width: 768px)",
  ),
  (
    "@media screen",
    r#"MediaKeyword(MediaKeyword { type: "media-keyword", key: "screen", not: false, only: false })"#,
    "@media screen",
  ),
  (
    "@media (min-width: 768px) and (max-width: 1024px)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 768.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 1024.0, unit: "px" }) })] })"#,
    "@media (min-width: 768px) and (max-width: 1024px)",
  ),
  (
    "@media screen and (min-width: 768px)",
    r#"And(MediaAndRules { type: "and", rules: [MediaKeyword(MediaKeyword { type: "media-keyword", key: "screen", not: false, only: false }), Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 768.0, unit: "px" }) })] })"#,
    "@media screen and (min-width: 768px)",
  ),
  (
    "@media (color)",
    r#"WordRule(MediaWordRule { type: "word-rule", key_value: "color" })"#,
    "@media (color)",
  ),
  (
    "@media (max-width: 1024px)",
    r#"Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 1024.0, unit: "px" }) })"#,
    "@media (max-width: 1024px)",
  ),
  (
    "@media all",
    r#"MediaKeyword(MediaKeyword { type: "media-keyword", key: "all", not: false, only: false })"#,
    "@media all",
  ),
  (
    "@media print",
    r#"MediaKeyword(MediaKeyword { type: "media-keyword", key: "print", not: false, only: false })"#,
    "@media print",
  ),
  (
    "@media ((min-width: 400px) and ((max-width: 700px) and (orientation: landscape)))",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 400.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 700.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "orientation", value: String("landscape") })] })"#,
    "@media (min-width: 400px) and (max-width: 700px) and (orientation: landscape)",
  ),
  (
    "@media (((min-width: 400px) and (max-width: 700px)) and ((orientation: landscape) and (hover: hover)))",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 400.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 700.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "orientation", value: String("landscape") }), Pair(MediaRulePair { type: "pair", key: "hover", value: String("hover") })] })"#,
    "@media (min-width: 400px) and (max-width: 700px) and (orientation: landscape) and (hover: hover)",
  ),
  (
    "@media (min-width: 400px) and ((max-width: 700px) and (orientation: landscape))",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 400.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 700.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "orientation", value: String("landscape") })] })"#,
    "@media (min-width: 400px) and (max-width: 700px) and (orientation: landscape)",
  ),
  (
    "@media screen and ((min-width: 500px) and ((max-width: 800px) and (color)))",
    r#"And(MediaAndRules { type: "and", rules: [MediaKeyword(MediaKeyword { type: "media-keyword", key: "screen", not: false, only: false }), Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 500.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 800.0, unit: "px" }) }), WordRule(MediaWordRule { type: "word-rule", key_value: "color" })] })"#,
    "@media screen and (min-width: 500px) and (max-width: 800px) and (color)",
  ),
  (
    "@media not ((min-width: 600px) and (max-width: 900px))",
    r#"Not(MediaNotRule { type: "not", rule: And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 600.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 900.0, unit: "px" }) })] }) })"#,
    "@media (not ((min-width: 600px) and (max-width: 900px)))",
  ),
  (
    "@media not ((pointer: fine) and (hover: hover))",
    r#"Not(MediaNotRule { type: "not", rule: And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "pointer", value: String("fine") }), Pair(MediaRulePair { type: "pair", key: "hover", value: String("hover") })] }) })"#,
    "@media (not ((pointer: fine) and (hover: hover)))",
  ),
  (
    "@media not (not (min-width: 400px))",
    r#"Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 400.0, unit: "px" }) })"#,
    "@media (min-width: 400px)",
  ),
  (
    "@media not (not (not (not (update: fast))))",
    r#"Pair(MediaRulePair { type: "pair", key: "update", value: String("fast") })"#,
    "@media (update: fast)",
  ),
  (
    "@media not (not (not (hover: hover)))",
    r#"Not(MediaNotRule { type: "not", rule: Pair(MediaRulePair { type: "pair", key: "hover", value: String("hover") }) })"#,
    "@media (not (hover: hover))",
  ),
  (
    "@media not print",
    r#"MediaKeyword(MediaKeyword { type: "media-keyword", key: "print", not: true, only: false })"#,
    "@media not print",
  ),
  (
    "@media not screen",
    r#"MediaKeyword(MediaKeyword { type: "media-keyword", key: "screen", not: true, only: false })"#,
    "@media not screen",
  ),
  (
    "@media (min-aspect-ratio: 3/2) and (max-aspect-ratio: 16/9)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-aspect-ratio", value: Fraction(Fraction { numerator: 3.0, denominator: 2.0 }) }), Pair(MediaRulePair { type: "pair", key: "max-aspect-ratio", value: Fraction(Fraction { numerator: 16.0, denominator: 9.0 }) })] })"#,
    "@media (min-aspect-ratio: 3 / 2) and (max-aspect-ratio: 16 / 9)",
  ),
  (
    "@media (min-height: 600px) and (orientation: landscape)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-height", value: Length(Length { value: 600.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "orientation", value: String("landscape") })] })"#,
    "@media (min-height: 600px) and (orientation: landscape)",
  ),
  (
    "@media (min-resolution: 300dpi) and (max-resolution: 600dpi)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-resolution", value: Length(Length { value: 300.0, unit: "dpi" }) }), Pair(MediaRulePair { type: "pair", key: "max-resolution", value: Length(Length { value: 600.0, unit: "dpi" }) })] })"#,
    "@media (min-resolution: 300dpi) and (max-resolution: 600dpi)",
  ),
  (
    "@media (min-width: 1200px) and (orientation: landscape)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 1200.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "orientation", value: String("landscape") })] })"#,
    "@media (min-width: 1200px) and (orientation: landscape)",
  ),
  (
    "@media (min-width: 576px) and (max-width: 767px) and (hover: none)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 576.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 767.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "hover", value: String("none") })] })"#,
    "@media (min-width: 576px) and (max-width: 767px) and (hover: none)",
  ),
  (
    "@media (min-width: 768px) and (max-width: 991px)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 768.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 991.0, unit: "px" }) })] })"#,
    "@media (min-width: 768px) and (max-width: 991px)",
  ),
  (
    "@media (min-width: 992px) and (max-width: 1199px) and (pointer: fine)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 992.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 1199.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "pointer", value: String("fine") })] })"#,
    "@media (min-width: 992px) and (max-width: 1199px) and (pointer: fine)",
  ),
  (
    "@media not all and (monochrome)",
    r#"Not(MediaNotRule { type: "not", rule: And(MediaAndRules { type: "and", rules: [MediaKeyword(MediaKeyword { type: "media-keyword", key: "all", not: false, only: false }), WordRule(MediaWordRule { type: "word-rule", key_value: "monochrome" })] }) })"#,
    "@media not all and (monochrome)",
  ),
  (
    "@media (orientation: landscape) and (pointer: fine)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "orientation", value: String("landscape") }), Pair(MediaRulePair { type: "pair", key: "pointer", value: String("fine") })] })"#,
    "@media (orientation: landscape) and (pointer: fine)",
  ),
  (
    "@media (orientation: landscape) and (update: fast)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "orientation", value: String("landscape") }), Pair(MediaRulePair { type: "pair", key: "update", value: String("fast") })] })"#,
    "@media (orientation: landscape) and (update: fast)",
  ),
  (
    "@media (prefers-reduced-motion: reduce) and (update: slow)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "prefers-reduced-motion", value: String("reduce") }), Pair(MediaRulePair { type: "pair", key: "update", value: String("slow") })] })"#,
    "@media (prefers-reduced-motion: reduce) and (update: slow)",
  ),
  (
    "@media screen and (device-aspect-ratio: 16/9)",
    r#"And(MediaAndRules { type: "and", rules: [MediaKeyword(MediaKeyword { type: "media-keyword", key: "screen", not: false, only: false }), Pair(MediaRulePair { type: "pair", key: "device-aspect-ratio", value: Fraction(Fraction { numerator: 16.0, denominator: 9.0 }) })] })"#,
    "@media screen and (device-aspect-ratio: 16 / 9)",
  ),
  (
    "@media screen and (min-width: 400px)",
    r#"And(MediaAndRules { type: "and", rules: [MediaKeyword(MediaKeyword { type: "media-keyword", key: "screen", not: false, only: false }), Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 400.0, unit: "px" }) })] })"#,
    "@media screen and (min-width: 400px)",
  ),
  (
    "@media (max-width: 200px) and (not (max-width: 300px))",
    r#"MediaKeyword(MediaKeyword { type: "media-keyword", key: "all", not: true, only: false })"#,
    "@media not all",
  ),
  (
    "@media (min-width: 100px) and (max-width: 500px) and (not (max-width: 200px))",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 200.01, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 500.0, unit: "px" }) })] })"#,
    "@media (min-width: 200.01px) and (max-width: 500px)",
  ),
  (
    "@media (min-width: 100px) and (max-width: 500px) and (not (min-width: 600px))",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 100.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 500.0, unit: "px" }) })] })"#,
    "@media (min-width: 100px) and (max-width: 500px)",
  ),
  (
    "@media (min-width: 100px) and (not (max-width: 100px))",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 100.01, unit: "px" }) })] })"#,
    "@media (min-width: 100.01px)",
  ),
  (
    "@media (min-width: 100px) and (not (max-width: 50px))",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 100.0, unit: "px" }) })] })"#,
    "@media (min-width: 100px)",
  ),
  (
    "@media (min-width: 100px) and (not (max-width: 99.99px))",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 100.0, unit: "px" }) })] })"#,
    "@media (min-width: 100px)",
  ),
  (
    "@media (min-width: 100px) and (orientation: landscape)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 100.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "orientation", value: String("landscape") })] })"#,
    "@media (min-width: 100px) and (orientation: landscape)",
  ),
  (
    "@media (min-width: calc(100px + 2em)) and (max-width: 500px)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: String("calc(100px + 2em)") }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 500.0, unit: "px" }) })] })"#,
    "@media (min-width: calc(100px + 2em)) and (max-width: 500px)",
  ),
  (
    "@media (1000px >= width)",
    r#"Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 1000.0, unit: "px" }) })"#,
    "@media (max-width: 1000px)",
  ),
  (
    "@media (1000px >= width > 700px)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 700.01, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 1000.0, unit: "px" }) })] })"#,
    "@media (min-width: 700.01px) and (max-width: 1000px)",
  ),
  (
    "@media (1000px >= width >= 700px)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 700.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 1000.0, unit: "px" }) })] })"#,
    "@media (min-width: 700px) and (max-width: 1000px)",
  ),
  (
    "@media (1000px > width)",
    r#"Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 999.99, unit: "px" }) })"#,
    "@media (max-width: 999.99px)",
  ),
  (
    "@media (1000px > width > 700px)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 700.01, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 999.99, unit: "px" }) })] })"#,
    "@media (min-width: 700.01px) and (max-width: 999.99px)",
  ),
  (
    "@media (1000px > width >= 700px)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 700.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 999.99, unit: "px" }) })] })"#,
    "@media (min-width: 700px) and (max-width: 999.99px)",
  ),
  (
    "@media (1000px <= width)",
    r#"Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 1000.0, unit: "px" }) })"#,
    "@media (min-width: 1000px)",
  ),
  (
    "@media (1000px < width)",
    r#"Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 1000.01, unit: "px" }) })"#,
    "@media (min-width: 1000.01px)",
  ),
  (
    "@media (1000px <= width <= 700px)",
    r#"MediaKeyword(MediaKeyword { type: "media-keyword", key: "all", not: true, only: false })"#,
    "@media not all",
  ),
  (
    "@media (400px >= width)",
    r#"Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 400.0, unit: "px" }) })"#,
    "@media (max-width: 400px)",
  ),
  (
    "@media (400px > width)",
    r#"Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 399.99, unit: "px" }) })"#,
    "@media (max-width: 399.99px)",
  ),
  (
    "@media (400px <= width)",
    r#"Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 400.0, unit: "px" }) })"#,
    "@media (min-width: 400px)",
  ),
  (
    "@media (400px < width)",
    r#"Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 400.01, unit: "px" }) })"#,
    "@media (min-width: 400.01px)",
  ),
  (
    "@media (400px < width < 700px)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 400.01, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 699.99, unit: "px" }) })] })"#,
    "@media (min-width: 400.01px) and (max-width: 699.99px)",
  ),
  (
    "@media (400px <= width <= 700px)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 400.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 700.0, unit: "px" }) })] })"#,
    "@media (min-width: 400px) and (max-width: 700px)",
  ),
  (
    "@media (width > 400px)",
    r#"Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 400.01, unit: "px" }) })"#,
    "@media (min-width: 400.01px)",
  ),
  (
    "@media (width >= 400px)",
    r#"Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 400.0, unit: "px" }) })"#,
    "@media (min-width: 400px)",
  ),
  (
    "@media (width < 400px)",
    r#"Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 399.99, unit: "px" }) })"#,
    "@media (max-width: 399.99px)",
  ),
  (
    "@media (width <= 400px)",
    r#"Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 400.0, unit: "px" }) })"#,
    "@media (max-width: 400px)",
  ),
  (
    "@media not only screen",
    r#"MediaKeyword(MediaKeyword { type: "media-keyword", key: "screen", not: true, only: true })"#,
    "@media not screen",
  ),
  (
    "@media not only screen and (min-width: 100px)",
    r#"Not(MediaNotRule { type: "not", rule: And(MediaAndRules { type: "and", rules: [MediaKeyword(MediaKeyword { type: "media-keyword", key: "screen", not: false, only: true }), Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 100.0, unit: "px" }) })] }) })"#,
    "@media not only screen and (min-width: 100px)",
  ),
  (
    "@media only print and (color)",
    r#"And(MediaAndRules { type: "and", rules: [MediaKeyword(MediaKeyword { type: "media-keyword", key: "print", not: false, only: true }), WordRule(MediaWordRule { type: "word-rule", key_value: "color" })] })"#,
    "@media only print and (color)",
  ),
  (
    "@media only screen",
    r#"MediaKeyword(MediaKeyword { type: "media-keyword", key: "screen", not: false, only: true })"#,
    "@media only screen",
  ),
  (
    "@media only screen and (max-width: 38em)",
    r#"And(MediaAndRules { type: "and", rules: [MediaKeyword(MediaKeyword { type: "media-keyword", key: "screen", not: false, only: true }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 38.0, unit: "em" }) })] })"#,
    "@media only screen and (max-width: 38em)",
  ),
  (
    "@media (max-width: 1440px) and (not (max-width: 1024px)) and (not (max-width: 768px)) and (not (max-width: 458px))",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 1024.01, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 1440.0, unit: "px" }) })] })"#,
    "@media (min-width: 1024.01px) and (max-width: 1440px)",
  ),
  (
    "@media not all and (monochrome) and (min-width: 600px)",
    r#"Not(MediaNotRule { type: "not", rule: And(MediaAndRules { type: "and", rules: [MediaKeyword(MediaKeyword { type: "media-keyword", key: "all", not: false, only: false }), WordRule(MediaWordRule { type: "word-rule", key_value: "monochrome" }), Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 600.0, unit: "px" }) })] }) })"#,
    "@media not all and (monochrome) and (min-width: 600px)",
  ),
  (
    "@media not ((min-width: 500px) and (max-width: 600px) and (max-width: 400px))",
    r#"MediaKeyword(MediaKeyword { type: "media-keyword", key: "all", not: false, only: false })"#,
    "@media all",
  ),
  (
    "@media not (not (not (min-width: 400px)))",
    r#"Not(MediaNotRule { type: "not", rule: Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 400.0, unit: "px" }) }) })"#,
    "@media (not (min-width: 400px))",
  ),
  (
    "@media (color) and (min-width: 400px), screen and (max-width: 700px)",
    r#"Or(MediaOrRules { type: "or", rules: [And(MediaAndRules { type: "and", rules: [WordRule(MediaWordRule { type: "word-rule", key_value: "color" }), Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 400.0, unit: "px" }) })] }), And(MediaAndRules { type: "and", rules: [MediaKeyword(MediaKeyword { type: "media-keyword", key: "screen", not: false, only: false }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 700.0, unit: "px" }) })] })] })"#,
    "@media (color) and (min-width: 400px), screen and (max-width: 700px)",
  ),
  (
    "@media (min-width: 500px) or (max-width: 600px)",
    r#"Or(MediaOrRules { type: "or", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 500.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 600.0, unit: "px" }) })] })"#,
    "@media (min-width: 500px), (max-width: 600px)",
  ),
  (
    "@media (min-width: 576px) and (max-width: 767px), (hover: none) and (any-pointer: coarse)",
    r#"Or(MediaOrRules { type: "or", rules: [And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 576.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 767.0, unit: "px" }) })] }), And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "hover", value: String("none") }), Pair(MediaRulePair { type: "pair", key: "any-pointer", value: String("coarse") })] })] })"#,
    "@media (min-width: 576px) and (max-width: 767px), (hover: none) and (any-pointer: coarse)",
  ),
  (
    "@media (min-width: 576px) and (max-width: 767px), (hover: none) and (any-pointer: coarse), (prefers-reduced-transparency: reduce) and (forced-colors: active)",
    r#"Or(MediaOrRules { type: "or", rules: [And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 576.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 767.0, unit: "px" }) })] }), And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "hover", value: String("none") }), Pair(MediaRulePair { type: "pair", key: "any-pointer", value: String("coarse") })] }), And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "prefers-reduced-transparency", value: String("reduce") }), Pair(MediaRulePair { type: "pair", key: "forced-colors", value: String("active") })] })] })"#,
    "@media (min-width: 576px) and (max-width: 767px), (hover: none) and (any-pointer: coarse), (prefers-reduced-transparency: reduce) and (forced-colors: active)",
  ),
  (
    "@media (min-width: 576px), (orientation: portrait) and (max-width: 767px)",
    r#"Or(MediaOrRules { type: "or", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 576.0, unit: "px" }) }), And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "orientation", value: String("portrait") }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 767.0, unit: "px" }) })] })] })"#,
    "@media (min-width: 576px), (orientation: portrait) and (max-width: 767px)",
  ),
  (
    "@media (min-width: 576px), (orientation: portrait) and (max-width: 767px), (prefers-color-scheme: dark)",
    r#"Or(MediaOrRules { type: "or", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 576.0, unit: "px" }) }), And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "orientation", value: String("portrait") }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 767.0, unit: "px" }) })] }), Pair(MediaRulePair { type: "pair", key: "prefers-color-scheme", value: String("dark") })] })"#,
    "@media (min-width: 576px), (orientation: portrait) and (max-width: 767px), (prefers-color-scheme: dark)",
  ),
  (
    "@media (min-width: 768px) and (max-width: 991px), (orientation: landscape)",
    r#"Or(MediaOrRules { type: "or", rules: [And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 768.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 991.0, unit: "px" }) })] }), Pair(MediaRulePair { type: "pair", key: "orientation", value: String("landscape") })] })"#,
    "@media (min-width: 768px) and (max-width: 991px), (orientation: landscape)",
  ),
  (
    "@media (min-width: 768px) and (max-width: 991px), (orientation: landscape) and (update: fast), (prefers-reduced-motion: reduce)",
    r#"Or(MediaOrRules { type: "or", rules: [And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 768.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 991.0, unit: "px" }) })] }), And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "orientation", value: String("landscape") }), Pair(MediaRulePair { type: "pair", key: "update", value: String("fast") })] }), Pair(MediaRulePair { type: "pair", key: "prefers-reduced-motion", value: String("reduce") })] })"#,
    "@media (min-width: 768px) and (max-width: 991px), (orientation: landscape) and (update: fast), (prefers-reduced-motion: reduce)",
  ),
  (
    "@media (min-width: 992px) and (max-width: 1199px), (pointer: fine) and (hover: hover)",
    r#"Or(MediaOrRules { type: "or", rules: [And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 992.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 1199.0, unit: "px" }) })] }), And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "pointer", value: String("fine") }), Pair(MediaRulePair { type: "pair", key: "hover", value: String("hover") })] })] })"#,
    "@media (min-width: 992px) and (max-width: 1199px), (pointer: fine) and (hover: hover)",
  ),
  (
    "@media (min-width: 992px) and (max-width: 1199px), (pointer: fine) and (hover: hover), (any-pointer: coarse) and (any-hover: none)",
    r#"Or(MediaOrRules { type: "or", rules: [And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 992.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 1199.0, unit: "px" }) })] }), And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "pointer", value: String("fine") }), Pair(MediaRulePair { type: "pair", key: "hover", value: String("hover") })] }), And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "any-pointer", value: String("coarse") }), Pair(MediaRulePair { type: "pair", key: "any-hover", value: String("none") })] })] })"#,
    "@media (min-width: 992px) and (max-width: 1199px), (pointer: fine) and (hover: hover), (any-pointer: coarse) and (any-hover: none)",
  ),
  (
    "@media (orientation: portrait), (orientation: landscape)",
    r#"Or(MediaOrRules { type: "or", rules: [Pair(MediaRulePair { type: "pair", key: "orientation", value: String("portrait") }), Pair(MediaRulePair { type: "pair", key: "orientation", value: String("landscape") })] })"#,
    "@media (orientation: portrait), (orientation: landscape)",
  ),
  (
    "@media (width: 500px), (height: 400px)",
    r#"Or(MediaOrRules { type: "or", rules: [Pair(MediaRulePair { type: "pair", key: "width", value: Length(Length { value: 500.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "height", value: Length(Length { value: 400.0, unit: "px" }) })] })"#,
    "@media (width: 500px), (height: 400px)",
  ),
  (
    "@media (any-hover: hover)",
    r#"Pair(MediaRulePair { type: "pair", key: "any-hover", value: String("hover") })"#,
    "@media (any-hover: hover)",
  ),
  (
    "@media (any-hover: none)",
    r#"Pair(MediaRulePair { type: "pair", key: "any-hover", value: String("none") })"#,
    "@media (any-hover: none)",
  ),
  (
    "@media (any-pointer: fine)",
    r#"Pair(MediaRulePair { type: "pair", key: "any-pointer", value: String("fine") })"#,
    "@media (any-pointer: fine)",
  ),
  (
    "@media (any-pointer: none)",
    r#"Pair(MediaRulePair { type: "pair", key: "any-pointer", value: String("none") })"#,
    "@media (any-pointer: none)",
  ),
  (
    "@media (aspect-ratio: 16 / 9)",
    r#"Pair(MediaRulePair { type: "pair", key: "aspect-ratio", value: Fraction(Fraction { numerator: 16.0, denominator: 9.0 }) })"#,
    "@media (aspect-ratio: 16 / 9)",
  ),
  (
    "@media (device-aspect-ratio: 16 / 9)",
    r#"Pair(MediaRulePair { type: "pair", key: "device-aspect-ratio", value: Fraction(Fraction { numerator: 16.0, denominator: 9.0 }) })"#,
    "@media (device-aspect-ratio: 16 / 9)",
  ),
  (
    "@media (display-mode: fullscreen)",
    r#"Pair(MediaRulePair { type: "pair", key: "display-mode", value: String("fullscreen") })"#,
    "@media (display-mode: fullscreen)",
  ),
  (
    "@media (display-mode: minimal-ui)",
    r#"Pair(MediaRulePair { type: "pair", key: "display-mode", value: String("minimal-ui") })"#,
    "@media (display-mode: minimal-ui)",
  ),
  (
    "@media (display-mode: standalone)",
    r#"Pair(MediaRulePair { type: "pair", key: "display-mode", value: String("standalone") })"#,
    "@media (display-mode: standalone)",
  ),
  (
    "@media (forced-colors: active)",
    r#"Pair(MediaRulePair { type: "pair", key: "forced-colors", value: String("active") })"#,
    "@media (forced-colors: active)",
  ),
  (
    "@media (forced-colors: none)",
    r#"Pair(MediaRulePair { type: "pair", key: "forced-colors", value: String("none") })"#,
    "@media (forced-colors: none)",
  ),
  (
    "@media (hover: hover)",
    r#"Pair(MediaRulePair { type: "pair", key: "hover", value: String("hover") })"#,
    "@media (hover: hover)",
  ),
  (
    "@media (hover: none)",
    r#"Pair(MediaRulePair { type: "pair", key: "hover", value: String("none") })"#,
    "@media (hover: none)",
  ),
  (
    "@media (inverted-colors: inverted)",
    r#"Pair(MediaRulePair { type: "pair", key: "inverted-colors", value: String("inverted") })"#,
    "@media (inverted-colors: inverted)",
  ),
  (
    "@media (inverted-colors: none)",
    r#"Pair(MediaRulePair { type: "pair", key: "inverted-colors", value: String("none") })"#,
    "@media (inverted-colors: none)",
  ),
  (
    "@media (light-level: dim)",
    r#"Pair(MediaRulePair { type: "pair", key: "light-level", value: String("dim") })"#,
    "@media (light-level: dim)",
  ),
  (
    "@media (light-level: washed)",
    r#"Pair(MediaRulePair { type: "pair", key: "light-level", value: String("washed") })"#,
    "@media (light-level: washed)",
  ),
  (
    "@media (max-height: calc(100vh - 50px))",
    r#"Pair(MediaRulePair { type: "pair", key: "max-height", value: String("calc(100vh - 50px)") })"#,
    "@media (max-height: calc(100vh - 50px))",
  ),
  (
    "@media (max-resolution: 600dppx)",
    r#"Pair(MediaRulePair { type: "pair", key: "max-resolution", value: Length(Length { value: 600.0, unit: "dppx" }) })"#,
    "@media (max-resolution: 600dppx)",
  ),
  (
    "@media (max-width: 50em)",
    r#"Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 50.0, unit: "em" }) })"#,
    "@media (max-width: 50em)",
  ),
  (
    "@media (min-resolution: 150dpi)",
    r#"Pair(MediaRulePair { type: "pair", key: "min-resolution", value: Length(Length { value: 150.0, unit: "dpi" }) })"#,
    "@media (min-resolution: 150dpi)",
  ),
  (
    "@media (min-width: calc(300px + 5em))",
    r#"Pair(MediaRulePair { type: "pair", key: "min-width", value: String("calc(300px + 5em)") })"#,
    "@media (min-width: calc(300px + 5em))",
  ),
  (
    "@media (orientation: landscape)",
    r#"Pair(MediaRulePair { type: "pair", key: "orientation", value: String("landscape") })"#,
    "@media (orientation: landscape)",
  ),
  (
    "@media (overflow-block: scroll)",
    r#"Pair(MediaRulePair { type: "pair", key: "overflow-block", value: String("scroll") })"#,
    "@media (overflow-block: scroll)",
  ),
  (
    "@media (overflow-inline: none)",
    r#"Pair(MediaRulePair { type: "pair", key: "overflow-inline", value: String("none") })"#,
    "@media (overflow-inline: none)",
  ),
  (
    "@media (pointer: coarse)",
    r#"Pair(MediaRulePair { type: "pair", key: "pointer", value: String("coarse") })"#,
    "@media (pointer: coarse)",
  ),
  (
    "@media (pointer: none)",
    r#"Pair(MediaRulePair { type: "pair", key: "pointer", value: String("none") })"#,
    "@media (pointer: none)",
  ),
  (
    "@media (prefers-color-scheme: dark)",
    r#"Pair(MediaRulePair { type: "pair", key: "prefers-color-scheme", value: String("dark") })"#,
    "@media (prefers-color-scheme: dark)",
  ),
  (
    "@media (prefers-contrast: more)",
    r#"Pair(MediaRulePair { type: "pair", key: "prefers-contrast", value: String("more") })"#,
    "@media (prefers-contrast: more)",
  ),
  (
    "@media (prefers-contrast: no-preference)",
    r#"Pair(MediaRulePair { type: "pair", key: "prefers-contrast", value: String("no-preference") })"#,
    "@media (prefers-contrast: no-preference)",
  ),
  (
    "@media (prefers-reduced-motion: no-preference)",
    r#"Pair(MediaRulePair { type: "pair", key: "prefers-reduced-motion", value: String("no-preference") })"#,
    "@media (prefers-reduced-motion: no-preference)",
  ),
  (
    "@media (prefers-reduced-motion: reduce)",
    r#"Pair(MediaRulePair { type: "pair", key: "prefers-reduced-motion", value: String("reduce") })"#,
    "@media (prefers-reduced-motion: reduce)",
  ),
  (
    "@media (prefers-reduced-transparency: no-preference)",
    r#"Pair(MediaRulePair { type: "pair", key: "prefers-reduced-transparency", value: String("no-preference") })"#,
    "@media (prefers-reduced-transparency: no-preference)",
  ),
  (
    "@media (prefers-reduced-transparency: reduce)",
    r#"Pair(MediaRulePair { type: "pair", key: "prefers-reduced-transparency", value: String("reduce") })"#,
    "@media (prefers-reduced-transparency: reduce)",
  ),
  (
    "@media (scripting: enabled)",
    r#"Pair(MediaRulePair { type: "pair", key: "scripting", value: String("enabled") })"#,
    "@media (scripting: enabled)",
  ),
  (
    "@media (scripting: none)",
    r#"Pair(MediaRulePair { type: "pair", key: "scripting", value: String("none") })"#,
    "@media (scripting: none)",
  ),
  (
    "@media (update: fast)",
    r#"Pair(MediaRulePair { type: "pair", key: "update", value: String("fast") })"#,
    "@media (update: fast)",
  ),
  (
    "@media (update: slow)",
    r#"Pair(MediaRulePair { type: "pair", key: "update", value: String("slow") })"#,
    "@media (update: slow)",
  ),
  (
    "@media (width: 100px)",
    r#"Pair(MediaRulePair { type: "pair", key: "width", value: Length(Length { value: 100.0, unit: "px" }) })"#,
    "@media (width: 100px)",
  ),
  (
    "@media (color-index)",
    r#"WordRule(MediaWordRule { type: "word-rule", key_value: "color-index" })"#,
    "@media (color-index)",
  ),
  (
    "@media (grid)",
    r#"WordRule(MediaWordRule { type: "word-rule", key_value: "grid" })"#,
    "@media (grid)",
  ),
  (
    "@media (monochrome)",
    r#"WordRule(MediaWordRule { type: "word-rule", key_value: "monochrome" })"#,
    "@media (monochrome)",
  ),
  (
    "@media (min-width: 100px) and (max-width: 500px) and (not (max-width: 200px)) and (not (min-width: 400px))",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 200.01, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 399.98, unit: "px" }) })] })"#,
    "@media (min-width: 200.01px) and (max-width: 399.98px)",
  ),
  (
    "@media (min-width: 100px) and (min-width: 200px)",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 200.0, unit: "px" }) })] })"#,
    "@media (min-width: 200px)",
  ),
  (
    "@media (min-width: 100px) and (not (max-width: 200px))",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 200.01, unit: "px" }) })] })"#,
    "@media (min-width: 200.01px)",
  ),
  (
    "@media (not (max-width: 200px)) and (not (max-width: 300px))",
    r#"And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 300.01, unit: "px" }) })] })"#,
    "@media (min-width: 300.01px)",
  ),
  (
    "@media (min-width: 900px) and (max-width: 1000px) and (not ((min-width: 500px) and (max-width: 1500px)))",
    r#"And(MediaAndRules { type: "and", rules: [Or(MediaOrRules { type: "or", rules: [] })] })"#,
    "@media not all",
  ),
  (
    "@media (min-width: 900px) and (max-width: 1440px) and (not ((min-width: 600px) and (max-width: 800px))) and (not ((min-width: 400px) and (max-width: 500px)))",
    r#"And(MediaAndRules { type: "and", rules: [Or(MediaOrRules { type: "or", rules: [Or(MediaOrRules { type: "or", rules: [] }), Or(MediaOrRules { type: "or", rules: [And(MediaAndRules { type: "and", rules: [Pair(MediaRulePair { type: "pair", key: "min-width", value: Length(Length { value: 900.0, unit: "px" }) }), Pair(MediaRulePair { type: "pair", key: "max-width", value: Length(Length { value: 1440.0, unit: "px" }) })] })] })] })] })"#,
    "@media (min-width: 900px) and (max-width: 1440px)",
  ),
];

const REJECTED: &[&str] = &["@media (not only screen)", "@media only not screen"];

#[test]
fn parses_each_query_to_its_tree_and_prints_it_back() {
  for (input, tree, printed) in PARSED {
    let parsed = MediaQuery::parser()
      .parse_to_end(input)
      .unwrap_or_else(|error| panic!("{input}: {error}"));
    assert_eq!(format!("{:?}", parsed.queries), *tree, "{input}");
    assert_eq!(parsed.to_string(), *printed, "{input}");
  }
}

#[test]
fn rejects_the_modifier_spellings_upstream_rejects() {
  for input in REJECTED {
    assert!(MediaQuery::parser().parse_to_end(input).is_err(), "{input}");
  }
}
