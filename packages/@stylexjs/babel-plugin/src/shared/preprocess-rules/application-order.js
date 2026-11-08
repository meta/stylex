/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { TStyleValue } from '../common-types';

/**
 * Shorthand properties:
 * - [x] all - Should be banned
 * - [x] animation
 * - [x] background
 * - [x] border
 * - [x] border-block-end
 * - [x] border-block-start
 * - [x] border-bottom
 * - [x] border-color
 * - [x] border-image
 * - [x] border-inline-end
 * - [x] border-inline-start
 * - [x] border-left
 * - [x] border-radius
 * - [x] corner-shape
 * - [x] border-right
 * - [x] border-style
 * - [x] border-top
 * - [x] border-width
 * - [x] column-rule
 * - [x] columns
 * - [x] container
 * - [x] flex
 * - [x] flex-flow
 * - [x] font
 * - [x] gap
 * - [x] grid
 * - [x] grid-area
 * - [x] grid-column
 * - [x] grid-row
 * - [x] grid-template
 * - [x] inset
 * - [x] inset-block
 * - [x] inset-inline
 * - [x] list-style
 * - [x] margin
 * - [x] mask
 * - [x] offset
 * - [x] outline
 * - [x] overflow
 * - [x] padding
 * - [x] place-content
 * - [x] place-items
 * - [x] place-self
 * - [x] scroll-margin
 * - [x] scroll-padding
 * - [x] text-decoration
 * - [x] text-emphasis
 * - [x] transition
 */

type TReturn = $ReadOnlyArray<[string, TStyleValue]>;

const shorthands: $ReadOnly<{ [key: string]: (TStyleValue) => TReturn }> = {
  all: (_: TStyleValue): TReturn => {
    throw new Error('all is not supported');
  },
  animation: (value: TStyleValue): Array<[string, TStyleValue]> => [
    ['animation', value],
    ['animationComposition', null],
    ['animationName', null],
    ['animationDuration', null],
    ['animationTimingFunction', null],
    ['animationDelay', null],
    ['animationIterationCount', null],
    ['animationDirection', null],
    ['animationFillMode', null],
    ['animationPlayState', null],
    ...shorthands.animationRange(null),
    ['animationTimeline', null],
  ],

  animationRange: (value: TStyleValue): TReturn => [
    ['animationRange', value],
    ['animationRangeEnd', null],
    ['animationRangeStart', null],
  ],

  background: (value: TStyleValue): TReturn => [
    ['background', value],
    ['backgroundAttachment', null],
    ['backgroundClip', null],
    ['backgroundColor', null],
    ['backgroundImage', null],
    ['backgroundOrigin', null],
    ...shorthands.backgroundPosition(null),
    ['backgroundRepeat', null],
    ['backgroundSize', null],
  ],

  backgroundPosition: (value: TStyleValue): TReturn => [
    ['backgroundPosition', value],
    ['backgroundPositionX', null],
    ['backgroundPositionY', null],
  ],

  // These will be removed later, matching the properties with React Native.
  // For now, we're compiling them to the React Native properties.
  border: (rawValue: TStyleValue): TReturn => [
    ['border', rawValue],
    ...shorthands.borderWidth(null),
    ...shorthands.borderStyle(null),
    ...shorthands.borderColor(null),
  ],
  borderInline: (rawValue: TStyleValue): TReturn => [
    ['borderInline', rawValue],
    ...shorthands.borderInlineWidth(null),
    ...shorthands.borderInlineStyle(null),
    ...shorthands.borderInlineColor(null),
  ],
  borderBlock: (rawValue: TStyleValue): TReturn => [
    ['borderBlock', rawValue],
    ...shorthands.borderBlockWidth(null),
    ...shorthands.borderBlockStyle(null),
    ...shorthands.borderBlockColor(null),
  ],

  borderTop: (rawValue: TStyleValue): TReturn => [
    ['borderTop', rawValue],
    ['borderTopWidth', null],
    ['borderTopStyle', null],
    ['borderTopColor', null],
  ],
  // @Deprecated
  borderInlineEnd: (rawValue: TStyleValue): TReturn => [
    ['borderInlineEnd', rawValue],
    ...shorthands.borderInlineEndWidth(null),
    ...shorthands.borderInlineEndStyle(null),
    ...shorthands.borderInlineEndColor(null),
  ],
  // @Deprecated
  borderRight: (rawValue: TStyleValue): TReturn => [
    ['borderRight', rawValue],
    ...shorthands.borderRightWidth(null),
    ...shorthands.borderRightStyle(null),
    ...shorthands.borderRightColor(null),
  ],
  // @Deprecated
  borderBottom: (rawValue: TStyleValue): TReturn => [
    ['borderBottom', rawValue],
    ['borderBottomWidth', null],
    ['borderBottomStyle', null],
    ['borderBottomColor', null],
  ],
  // @Deprecated
  borderInlineStart: (rawValue: TStyleValue): TReturn => [
    ['borderInlineStart', rawValue],
    ...shorthands.borderInlineStartWidth(null),
    ...shorthands.borderInlineStartStyle(null),
    ...shorthands.borderInlineStartColor(null),
  ],
  // @Deprecated
  borderLeft: (rawValue: TStyleValue): TReturn => [
    ['borderLeft', rawValue],
    ...shorthands.borderLeftWidth(null),
    ...shorthands.borderLeftStyle(null),
    ...shorthands.borderLeftColor(null),
  ],
  borderInlineWidth: (rawValue: TStyleValue): TReturn => [
    ['borderInlineWidth', rawValue],
    ['borderInlineStartWidth', null],
    ['borderLeftWidth', null],
    ['borderInlineEndWidth', null],
    ['borderRightWidth', null],
  ],
  borderInlineStyle: (rawValue: TStyleValue): TReturn => [
    ['borderInlineStyle', rawValue],
    ['borderInlineStartStyle', null],
    ['borderLeftStyle', null],
    ['borderInlineEndStyle', null],
    ['borderRightStyle', null],
  ],
  borderInlineColor: (rawValue: TStyleValue): TReturn => [
    ['borderInlineColor', rawValue],
    ['borderInlineStartColor', null],
    ['borderLeftColor', null],
    ['borderInlineEndColor', null],
    ['borderRightColor', null],
  ],
  borderBlockWidth: (rawValue: TStyleValue): TReturn => [
    ['borderBlockWidth', rawValue],
    ['borderTopWidth', null],
    ['borderBottomWidth', null],
  ],
  borderBlockStyle: (rawValue: TStyleValue): TReturn => [
    ['borderBlockStyle', rawValue],
    ['borderTopStyle', null],
    ['borderBottomStyle', null],
  ],
  borderBlockColor: (rawValue: TStyleValue): TReturn => [
    ['borderBlockColor', rawValue],
    ['borderTopColor', null],
    ['borderBottomColor', null],
  ],
  borderColor: (value: TStyleValue): TReturn => [
    ['borderColor', value],
    ...shorthands.borderInlineColor(null),
    ...shorthands.borderBlockColor(null),
  ],
  borderStyle: (value: TStyleValue): TReturn => [
    ['borderStyle', value],
    ...shorthands.borderInlineStyle(null),
    ...shorthands.borderBlockStyle(null),
  ],
  borderWidth: (value: TStyleValue): TReturn => [
    ['borderWidth', value],
    ...shorthands.borderInlineWidth(null),
    ...shorthands.borderBlockWidth(null),
  ],
  borderInlineStartColor: (value: TStyleValue): TReturn => [
    ['borderInlineStartColor', value],
    ['borderLeftColor', null],
    ['borderRightColor', null],
  ],
  borderInlineEndColor: (value: TStyleValue): TReturn => [
    ['borderInlineEndColor', value],
    ['borderLeftColor', null],
    ['borderRightColor', null],
  ],
  borderInlineStartStyle: (value: TStyleValue): TReturn => [
    ['borderInlineStartStyle', value],
    ['borderLeftStyle', null],
    ['borderRightStyle', null],
  ],
  borderInlineEndStyle: (value: TStyleValue): TReturn => [
    ['borderInlineEndStyle', value],
    ['borderLeftStyle', null],
    ['borderRightStyle', null],
  ],
  borderInlineStartWidth: (value: TStyleValue): TReturn => [
    ['borderInlineStartWidth', value],
    ['borderLeftWidth', null],
    ['borderRightWidth', null],
  ],
  borderInlineEndWidth: (value: TStyleValue): TReturn => [
    ['borderInlineEndWidth', value],
    ['borderLeftWidth', null],
    ['borderRightWidth', null],
  ],
  borderLeftColor: (value: TStyleValue): TReturn => [
    ['borderLeftColor', value],
    ['borderInlineStartColor', null],
    ['borderInlineEndColor', null],
  ],
  borderRightColor: (value: TStyleValue): TReturn => [
    ['borderRightColor', value],
    ['borderInlineStartColor', null],
    ['borderInlineEndColor', null],
  ],
  borderLeftStyle: (value: TStyleValue): TReturn => [
    ['borderLeftStyle', value],
    ['borderInlineStartStyle', null],
    ['borderInlineEndStyle', null],
  ],
  borderRightStyle: (value: TStyleValue): TReturn => [
    ['borderRightStyle', value],
    ['borderInlineStartStyle', null],
    ['borderInlineEndStyle', null],
  ],
  borderLeftWidth: (value: TStyleValue): TReturn => [
    ['borderLeftWidth', value],
    ['borderInlineStartWidth', null],
    ['borderInlineEndWidth', null],
  ],
  borderRightWidth: (value: TStyleValue): TReturn => [
    ['borderRightWidth', value],
    ['borderInlineStartWidth', null],
    ['borderInlineEndWidth', null],
  ],

  borderRadius: (value: TStyleValue): TReturn => [
    ['borderRadius', value],
    // // logical constituents
    ['borderStartStartRadius', null],
    ['borderStartEndRadius', null],
    ['borderEndStartRadius', null],
    ['borderEndEndRadius', null],
    // physical constituents
    ['borderTopLeftRadius', null],
    ['borderTopRightRadius', null],
    ['borderBottomLeftRadius', null],
    ['borderBottomRightRadius', null],
  ],

  borderStartStartRadius: (value: TStyleValue): TReturn => [
    ['borderStartStartRadius', value],
    ['borderTopLeftRadius', null],
    ['borderTopRightRadius', null],
  ],
  borderStartEndRadius: (value: TStyleValue): TReturn => [
    ['borderStartEndRadius', value],
    ['borderTopLeftRadius', null],
    ['borderTopRightRadius', null],
  ],
  borderEndStartRadius: (value: TStyleValue): TReturn => [
    ['borderEndStartRadius', value],
    ['borderBottomLeftRadius', null],
    ['borderBottomRightRadius', null],
  ],
  borderEndEndRadius: (value: TStyleValue): TReturn => [
    ['borderEndEndRadius', value],
    ['borderBottomLeftRadius', null],
    ['borderBottomRightRadius', null],
  ],
  borderTopLeftRadius: (value: TStyleValue): TReturn => [
    ['borderTopLeftRadius', value],
    ['borderStartStartRadius', null],
    ['borderStartEndRadius', null],
  ],
  borderTopRightRadius: (value: TStyleValue): TReturn => [
    ['borderTopRightRadius', value],
    ['borderStartStartRadius', null],
    ['borderStartEndRadius', null],
  ],
  borderBottomLeftRadius: (value: TStyleValue): TReturn => [
    ['borderBottomLeftRadius', value],
    ['borderEndStartRadius', null],
    ['borderEndEndRadius', null],
  ],
  borderBottomRightRadius: (value: TStyleValue): TReturn => [
    ['borderBottomRightRadius', value],
    ['borderEndStartRadius', null],
    ['borderEndEndRadius', null],
  ],

  cornerShape: (value: TStyleValue): TReturn => [
    ['cornerShape', value],
    ['cornerStartStartShape', null],
    ['cornerStartEndShape', null],
    ['cornerEndStartShape', null],
    ['cornerEndEndShape', null],
    ['cornerTopLeftShape', null],
    ['cornerTopRightShape', null],
    ['cornerBottomLeftShape', null],
    ['cornerBottomRightShape', null],
  ],
  cornerStartStartShape: (value: TStyleValue): TReturn => [
    ['cornerStartStartShape', value],
    ['cornerTopLeftShape', null],
    ['cornerTopRightShape', null],
  ],
  cornerStartEndShape: (value: TStyleValue): TReturn => [
    ['cornerStartEndShape', value],
    ['cornerTopLeftShape', null],
    ['cornerTopRightShape', null],
  ],
  cornerEndStartShape: (value: TStyleValue): TReturn => [
    ['cornerEndStartShape', value],
    ['cornerBottomLeftShape', null],
    ['cornerBottomRightShape', null],
  ],
  cornerEndEndShape: (value: TStyleValue): TReturn => [
    ['cornerEndEndShape', value],
    ['cornerBottomLeftShape', null],
    ['cornerBottomRightShape', null],
  ],
  cornerTopLeftShape: (value: TStyleValue): TReturn => [
    ['cornerTopLeftShape', value],
    ['cornerStartStartShape', null],
    ['cornerStartEndShape', null],
  ],
  cornerTopRightShape: (value: TStyleValue): TReturn => [
    ['cornerTopRightShape', value],
    ['cornerStartStartShape', null],
    ['cornerStartEndShape', null],
  ],
  cornerBottomLeftShape: (value: TStyleValue): TReturn => [
    ['cornerBottomLeftShape', value],
    ['cornerEndStartShape', null],
    ['cornerEndEndShape', null],
  ],
  cornerBottomRightShape: (value: TStyleValue): TReturn => [
    ['cornerBottomRightShape', value],
    ['cornerEndStartShape', null],
    ['cornerEndEndShape', null],
  ],

  borderImage: (value: TStyleValue): TReturn => [
    ['borderImage', value],
    ['borderImageOutset', null],
    ['borderImageRepeat', null],
    ['borderImageSlice', null],
    ['borderImageSource', null],
    ['borderImageWidth', null],
  ],

  columnRule: (value: TStyleValue): TReturn => [
    ['columnRule', value],
    ['columnRuleColor', null],
    ['columnRuleStyle', null],
    ['columnRuleWidth', null],
  ],
  columns: (value: TStyleValue): TReturn => [
    ['columns', value],
    ['columnCount', null],
    ['columnWidth', null],
  ],

  container: (value: TStyleValue): TReturn => [
    ['container', value],
    ['containerName', null],
    ['containerType', null],
  ],

  containIntrinsicSize: (value: TStyleValue): TReturn => [
    ['containIntrinsicSize', value],
    ['containIntrinsicWidth', null],
    ['containIntrinsicHeight', null],
  ],

  flex: (value: TStyleValue): TReturn => [
    ['flex', value],
    ['flexGrow', null],
    ['flexShrink', null],
    ['flexBasis', null],
  ],
  flexFlow: (value: TStyleValue): TReturn => [
    ['flexFlow', value],
    ['flexDirection', null],
    ['flexWrap', null],
  ],
  // @Deprecated ?
  font: (value: TStyleValue): TReturn => [
    ['font', value],
    ['fontFamily', null],
    ['fontSize', null],
    ['fontStretch', null],
    ['fontStyle', null],
    ...shorthands.fontVariant(null),
    ['fontWeight', null],
    ['lineHeight', null],
  ],
  fontVariant: (value: TStyleValue): TReturn => [
    ['fontVariant', value],
    ['fontVariantAlternates', null],
    ['fontVariantCaps', null],
    ['fontVariantEastAsian', null],
    ['fontVariantEmoji', null],
    ['fontVariantLigatures', null],
    ['fontVariantNumeric', null],
    ['fontVariantPosition', null],
  ],
  gap: (value: TStyleValue): TReturn => [
    ['gap', value],
    ['rowGap', null],
    ['columnGap', null],
  ],
  grid: (value: TStyleValue): TReturn => [
    ['grid', value],
    ...shorthands.gridTemplate(null),
    ['gridAutoRows', null],
    ['gridAutoColumns', null],
    ['gridAutoFlow', null],
  ],
  gridArea: (value: TStyleValue): TReturn => [
    ['gridArea', value],
    ['gridRow', null],
    ['gridRowStart', null],
    ['gridRowEnd', null],
    ['gridColumn', null],
    ['gridColumnStart', null],
    ['gridColumnEnd', null],
  ],
  gridRow: (value: TStyleValue): TReturn => [
    ['gridRow', value],
    ['gridRowStart', null],
    ['gridRowEnd', null],
  ],
  gridColumn: (value: TStyleValue): TReturn => [
    ['gridColumn', value],
    ['gridColumnStart', null],
    ['gridColumnEnd', null],
  ],
  gridTemplate: (value: TStyleValue): TReturn => [
    ['gridTemplate', value],
    ['gridTemplateAreas', null],
    ['gridTemplateColumns', null],
    ['gridTemplateRows', null],
  ],
  inset: (value: TStyleValue): TReturn => [
    ['inset', value],
    ...shorthands.insetInline(null),
    ...shorthands.insetBlock(null),
  ],
  insetInline: (value: TStyleValue): TReturn => [
    ['insetInline', value],
    ['insetInlineStart', null],
    ['insetInlineEnd', null],
    ['left', null],
    ['right', null],
  ],
  insetBlock: (value: TStyleValue): TReturn => [
    ['insetBlock', value],
    ['top', null],
    ['bottom', null],
  ],
  insetInlineStart: (value: TStyleValue): TReturn => [
    ['insetInlineStart', value],
    ['left', null],
    ['right', null],
  ],
  insetInlineEnd: (value: TStyleValue): TReturn => [
    ['insetInlineEnd', value],
    ['left', null],
    ['right', null],
  ],
  left: (value: TStyleValue): TReturn => [
    ['left', value],
    ['insetInlineStart', null],
    ['insetInlineEnd', null],
  ],
  right: (value: TStyleValue): TReturn => [
    ['right', value],
    ['insetInlineStart', null],
    ['insetInlineEnd', null],
  ],

  listStyle: (value: TStyleValue): TReturn => [
    ['listStyle', value],
    ['listStyleImage', null],
    ['listStylePosition', null],
    ['listStyleType', null],
  ],

  margin: (value: TStyleValue): TReturn => [
    ['margin', value],
    ...shorthands.marginInline(null),
    ...shorthands.marginBlock(null),
  ],
  marginInline: (value: TStyleValue): TReturn => [
    ['marginInline', value],
    ['marginInlineStart', null],
    ['marginLeft', null],
    ['marginInlineEnd', null],
    ['marginRight', null],
  ],
  marginBlock: (value: TStyleValue): TReturn => [
    ['marginBlock', value],
    ['marginTop', null],
    ['marginBottom', null],
  ],
  marginInlineStart: (value: TStyleValue): TReturn => [
    ['marginInlineStart', value],
    ['marginLeft', null],
    ['marginRight', null],
  ],
  marginInlineEnd: (value: TStyleValue): TReturn => [
    ['marginInlineEnd', value],
    ['marginLeft', null],
    ['marginRight', null],
  ],
  marginLeft: (value: TStyleValue): TReturn => [
    ['marginLeft', value],
    ['marginInlineStart', null],
    ['marginInlineEnd', null],
  ],
  marginRight: (value: TStyleValue): TReturn => [
    ['marginRight', value],
    ['marginInlineStart', null],
    ['marginInlineEnd', null],
  ],

  mask: (value: TStyleValue): TReturn => [
    ['mask', value],
    ['maskClip', null],
    ['maskComposite', null],
    ['maskImage', null],
    ['maskMode', null],
    ['maskOrigin', null],
    ['maskPosition', null],
    ['maskRepeat', null],
    ['maskSize', null],
  ],

  maskBorder: (value: TStyleValue): TReturn => [
    ['maskBorder', value],
    ['maskBorderMode', null],
    ['maskBorderOutset', null],
    ['maskBorderRepeat', null],
    ['maskBorderSlice', null],
    ['maskBorderSource', null],
    ['maskBorderWidth', null],
  ],

  offset: (value: TStyleValue): TReturn => [
    ['offset', value],
    ['offsetAnchor', null],
    ['offsetDistance', null],
    ['offsetPath', null],
    ['offsetPosition', null],
    ['offsetRotate', null],
  ],

  outline: (value: TStyleValue): TReturn => [
    ['outline', value],
    ['outlineColor', null],
    ['outlineOffset', null],
    ['outlineStyle', null],
    ['outlineWidth', null],
  ],

  overflow: (value: TStyleValue): TReturn => [
    ['overflow', value],
    ['overflowX', null],
    ['overflowY', null],
  ],

  padding: (rawValue: TStyleValue): TReturn => {
    return [
      ['padding', rawValue],
      ...shorthands.paddingInline(null),
      ...shorthands.paddingBlock(null),
    ];
  },
  paddingInline: (rawValue: TStyleValue): TReturn => [
    ['paddingInline', rawValue],
    ['paddingInlineStart', null],
    ['paddingLeft', null],
    ['paddingInlineEnd', null],
    ['paddingRight', null],
  ],
  paddingBlock: (rawValue: TStyleValue): TReturn => [
    ['paddingBlock', rawValue],
    ['paddingTop', null],
    ['paddingBottom', null],
  ],
  paddingInlineStart: (value: TStyleValue): TReturn => [
    ['paddingInlineStart', value],
    ['paddingLeft', null],
    ['paddingRight', null],
  ],
  paddingInlineEnd: (value: TStyleValue): TReturn => [
    ['paddingInlineEnd', value],
    ['paddingLeft', null],
    ['paddingRight', null],
  ],
  paddingLeft: (value: TStyleValue): TReturn => [
    ['paddingLeft', value],
    ['paddingInlineStart', null],
    ['paddingInlineEnd', null],
  ],
  paddingRight: (value: TStyleValue): TReturn => [
    ['paddingRight', value],
    ['paddingInlineStart', null],
    ['paddingInlineEnd', null],
  ],
  placeContent: (value: TStyleValue): TReturn => [
    ['placeContent', value],
    ['alignContent', null],
    ['justifyContent', null],
  ],
  placeItems: (value: TStyleValue): TReturn => [
    ['placeItems', value],
    ['alignItems', null],
    ['justifyItems', null],
  ],
  placeSelf: (value: TStyleValue): TReturn => [
    ['placeSelf', value],
    ['alignSelf', null],
    ['justifySelf', null],
  ],
  scrollMargin: (value: TStyleValue): TReturn => [
    ['scrollMargin', value],
    ...shorthands.scrollMarginBlock(null),
    ...shorthands.scrollMarginInline(null),
  ],
  scrollMarginBlock: (value: TStyleValue): TReturn => [
    ['scrollMarginBlock', value],
    ['scrollMarginTop', null],
    ['scrollMarginBottom', null],
  ],
  scrollMarginInline: (value: TStyleValue): TReturn => [
    ['scrollMarginInline', value],
    ['scrollMarginInlineStart', null],
    ['scrollMarginInlineEnd', null],
    ['scrollMarginLeft', null],
    ['scrollMarginRight', null],
  ],
  scrollMarginInlineStart: (value: TStyleValue): TReturn => [
    ['scrollMarginInlineStart', value],
    ['scrollMarginLeft', null],
    ['scrollMarginRight', null],
  ],
  scrollMarginInlineEnd: (value: TStyleValue): TReturn => [
    ['scrollMarginInlineEnd', value],
    ['scrollMarginLeft', null],
    ['scrollMarginRight', null],
  ],
  scrollMarginLeft: (value: TStyleValue): TReturn => [
    ['scrollMarginLeft', value],
    ['scrollMarginInlineStart', null],
    ['scrollMarginInlineEnd', null],
  ],
  scrollMarginRight: (value: TStyleValue): TReturn => [
    ['scrollMarginRight', value],
    ['scrollMarginInlineStart', null],
    ['scrollMarginInlineEnd', null],
  ],
  scrollPadding: (value: TStyleValue): TReturn => [
    ['scrollPadding', value],
    ...shorthands.scrollPaddingBlock(null),
    ...shorthands.scrollPaddingInline(null),
  ],
  scrollPaddingBlock: (value: TStyleValue): TReturn => [
    ['scrollPaddingBlock', value],
    ['scrollPaddingTop', null],
    ['scrollPaddingBottom', null],
  ],
  scrollPaddingInline: (value: TStyleValue): TReturn => [
    ['scrollPaddingInline', value],
    ['scrollPaddingInlineStart', null],
    ['scrollPaddingInlineEnd', null],
    ['scrollPaddingLeft', null],
    ['scrollPaddingRight', null],
  ],
  scrollPaddingInlineStart: (value: TStyleValue): TReturn => [
    ['scrollPaddingInlineStart', value],
    ['scrollPaddingLeft', null],
    ['scrollPaddingRight', null],
  ],
  scrollPaddingInlineEnd: (value: TStyleValue): TReturn => [
    ['scrollPaddingInlineEnd', value],
    ['scrollPaddingLeft', null],
    ['scrollPaddingRight', null],
  ],
  scrollPaddingLeft: (value: TStyleValue): TReturn => [
    ['scrollPaddingLeft', value],
    ['scrollPaddingInlineStart', null],
    ['scrollPaddingInlineEnd', null],
  ],
  scrollPaddingRight: (value: TStyleValue): TReturn => [
    ['scrollPaddingRight', value],
    ['scrollPaddingInlineStart', null],
    ['scrollPaddingInlineEnd', null],
  ],
  scrollSnapType: (value: TStyleValue): TReturn => [
    ['scrollSnapType', value],
    ['scrollSnapTypeX', null],
    ['scrollSnapTypeY', null],
  ],
  scrollTimeline: (value: TStyleValue): TReturn => [
    ['scrollTimeline', value],
    ['scrollTimelineName', null],
    ['scrollTimelineAxis', null],
  ],
  textDecoration: (value: TStyleValue): TReturn => [
    ['textDecoration', value],
    ['textDecorationColor', null],
    ['textDecorationLine', null],
    ['textDecorationStyle', null],
    ['textDecorationThickness', null],
  ],
  textEmphasis: (value: TStyleValue): TReturn => [
    ['textEmphasis', value],
    ['textEmphasisColor', null],
    ['textEmphasisStyle', null],
  ],
  transition: (value: TStyleValue): TReturn => [
    ['transition', value],
    ['transitionBehavior', null],
    ['transitionDelay', null],
    ['transitionDuration', null],
    ['transitionProperty', null],
    ['transitionTimingFunction', null],
  ],
};

const aliases = {
  // @Deprecated
  borderHorizontal: shorthands.borderInline,
  // @Deprecated
  borderVertical: shorthands.borderBlock,
  // @Deprecated
  borderBlockStart: shorthands.borderTop,
  // @Deprecated
  borderEnd: shorthands.borderInlineEnd,
  // @Deprecated
  borderBlockEnd: shorthands.borderBottom,
  // @Deprecated
  borderStart: shorthands.borderInlineStart,

  blockSize: (val: TStyleValue): TReturn => [['height', val]],
  inlineSize: (val: TStyleValue): TReturn => [['width', val]],
  minBlockSize: (val: TStyleValue): TReturn => [['minHeight', val]],
  minInlineSize: (val: TStyleValue): TReturn => [['minWidth', val]],
  maxBlockSize: (val: TStyleValue): TReturn => [['maxHeight', val]],
  maxInlineSize: (val: TStyleValue): TReturn => [['maxWidth', val]],

  borderHorizontalWidth: shorthands.borderInlineWidth,
  borderHorizontalStyle: shorthands.borderInlineStyle,
  borderHorizontalColor: shorthands.borderInlineColor,
  borderVerticalWidth: shorthands.borderBlockWidth,
  borderVerticalStyle: shorthands.borderBlockStyle,
  borderVerticalColor: shorthands.borderBlockColor,

  borderBlockStartColor: (value: TStyleValue): TReturn => [
    ['borderTopColor', value],
  ],
  borderBlockEndColor: (value: TStyleValue): TReturn => [
    ['borderBottomColor', value],
  ],
  borderBlockStartStyle: (value: TStyleValue): TReturn => [
    ['borderTopStyle', value],
  ],
  borderBlockEndStyle: (value: TStyleValue): TReturn => [
    ['borderBottomStyle', value],
  ],
  borderBlockStartWidth: (value: TStyleValue): TReturn => [
    ['borderTopWidth', value],
  ],
  borderBlockEndWidth: (value: TStyleValue): TReturn => [
    ['borderBottomWidth', value],
  ],

  borderStartColor: shorthands.borderInlineStartColor,
  borderEndColor: shorthands.borderInlineEndColor,
  borderStartStyle: shorthands.borderInlineStartStyle,
  borderEndStyle: shorthands.borderInlineEndStyle,
  borderStartWidth: shorthands.borderInlineStartWidth,
  borderEndWidth: shorthands.borderInlineEndWidth,

  borderTopStartRadius: (value: TStyleValue): TReturn => [
    ['borderStartStartRadius', value],
  ],
  borderTopEndRadius: (value: TStyleValue): TReturn => [
    ['borderStartEndRadius', value],
  ],
  borderBottomStartRadius: (value: TStyleValue): TReturn => [
    ['borderEndStartRadius', value],
  ],
  borderBottomEndRadius: (value: TStyleValue): TReturn => [
    ['borderEndEndRadius', value],
  ],

  containIntrinsicBlockSize: (value: TStyleValue): TReturn => [
    ['containIntrinsicHeight', value],
  ],
  containIntrinsicInlineSize: (value: TStyleValue): TReturn => [
    ['containIntrinsicWidth', value],
  ],

  gridGap: shorthands.gap,
  gridRowGap: (value: TStyleValue): TReturn => [['rowGap', value]],
  gridColumnGap: (value: TStyleValue): TReturn => [['columnGap', value]],

  marginBlockStart: (value: TStyleValue): TReturn => [['marginTop', value]],
  marginBlockEnd: (value: TStyleValue): TReturn => [['marginBottom', value]],
  marginStart: shorthands.marginInlineStart,
  marginEnd: shorthands.marginInlineEnd,
  marginHorizontal: shorthands.marginInline,
  marginVertical: shorthands.marginBlock,

  overflowBlock: (value: TStyleValue): TReturn => [['overflowY', value]],
  overflowInline: (value: TStyleValue): TReturn => [['overflowX', value]],

  paddingBlockStart: (rawValue: TStyleValue): TReturn => [
    ['paddingTop', rawValue],
  ],
  paddingBlockEnd: (rawValue: TStyleValue): TReturn => [
    ['paddingBottom', rawValue],
  ],
  paddingStart: shorthands.paddingInlineStart,
  paddingEnd: shorthands.paddingInlineEnd,
  paddingHorizontal: shorthands.paddingInline,
  paddingVertical: shorthands.paddingBlock,

  scrollMarginBlockStart: (value: TStyleValue): TReturn => [
    ['scrollMarginTop', value],
  ],
  scrollMarginBlockEnd: (value: TStyleValue): TReturn => [
    ['scrollMarginBottom', value],
  ],

  insetBlockStart: (value: TStyleValue): TReturn => [['top', value]],
  insetBlockEnd: (value: TStyleValue): TReturn => [['bottom', value]],
  start: shorthands.insetInlineStart,
  end: shorthands.insetInlineEnd,
};

const expansions = {
  ...shorthands,
  ...aliases,
};

export default expansions as $ReadOnly<{
  ...typeof shorthands,
  ...typeof aliases,
}>;
