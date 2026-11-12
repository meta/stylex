/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { StyleXOptions } from '../common-types';

import normalizeValue from './normalize-value';
import parser from 'postcss-value-parser';

/**
 * Convert a CSS value in JS to the final CSS string value
 */
export default function transformValue(
  key: string,
  rawValue: string | number,
  options: StyleXOptions,
): string {
  const value =
    typeof rawValue === 'number'
      ? String(Math.round(rawValue * 10000) / 10000) + getNumberSuffix(key)
      : rawValue;

  if (
    (key === 'content' ||
      key === 'hyphenateCharacter' ||
      key === 'hyphenate-character') &&
    typeof value === 'string'
  ) {
    const val = value.trim();

    const cssContentFunctions = [
      'attr(',
      'counter(',
      'counters(',
      'url(',
      'linear-gradient(',
      'image-set(',
      'var(--',
    ];

    const cssContentKeywords = new Set([
      'normal',
      'none',
      'open-quote',
      'close-quote',
      'no-open-quote',
      'no-close-quote',
      'inherit',
      'initial',
      'revert',
      'revert-layer',
      'unset',
    ]);

    const isCssFunction = cssContentFunctions.some((func) =>
      val.includes(func),
    );
    const isKeyword = cssContentKeywords.has(val);

    // A value the author already wrote as CSS is a list of content components:
    // quoted strings, functions and quote keywords. Counting quote characters
    // is not enough to detect that, because ordinary text can contain a pair of
    // them, as in "Bob's and Jim's" or 'He said "hello"'. Such a value is not a
    // CSS string, so the browser drops the whole declaration.
    const contentNodes = parser(val).nodes.filter(
      (node) => node.type !== 'space' && node.type !== 'div',
    );
    const isQuotedContentList =
      contentNodes.some((node) => node.type === 'string' && !node.unclosed) &&
      contentNodes.every((node) => {
        if (node.type === 'string' || node.type === 'function') {
          return !node.unclosed;
        }
        return node.type === 'word' && cssContentKeywords.has(node.value);
      });

    if (isCssFunction || isKeyword || isQuotedContentList) {
      return val;
    }

    // Inside a CSS string a backslash starts an escape sequence, so an escape
    // the author wrote, such as `\2014` for an em dash, is left as it is. Only
    // what would end the string early is escaped: a double quote that is not
    // already escaped, a trailing backslash that would otherwise escape the
    // closing quote, and a line break, which a CSS string writes as `\A`.
    const escaped = val.replace(
      /\\(?:\r\n|[\s\S])|\r\n|[\n\r\f]|["\\]/g,
      (match) => {
        if (match.length > 1 && match[0] === '\\') {
          return match;
        }
        if (match === '"' || match === '\\') {
          return `\\${match}`;
        }
        return '\\A ';
      },
    );
    return `"${escaped}"`;
  }

  return normalizeValue(value, key, options);
}

export function getNumberSuffix(key: string): string {
  if (unitlessNumberProperties.has(key) || key.startsWith('--')) {
    return '';
  }
  if (!(key in numberPropertySuffixes)) {
    return 'px';
  }

  const suffix = numberPropertySuffixes[key];
  if (suffix == null) {
    return 'px';
  } else {
    return suffix;
  }
}

const unitlessNumberProperties = new Set([
  'WebkitLineClamp',
  'animationIterationCount',
  'aspectRatio',
  'borderImageOutset',
  'borderImageSlice',
  'borderImageWidth',
  'counterSet',
  'counterReset',
  'columnCount',
  'flex',
  'flexGrow',
  'flexShrink',
  'flexOrder',
  'gridRow',
  'gridRowStart',
  'gridRowEnd',
  'gridColumn',
  'gridColumnStart',
  'gridColumnEnd',
  'gridArea',
  'fontSizeAdjust',
  'fontWeight',
  'hyphenateLimitChars',
  'lineClamp',
  'lineHeight',
  'maskBorderOutset',
  'maskBorderSlice',
  'maskBorderWidth',
  'opacity',
  'order',
  'orphans',
  'tabSize',
  'widows',
  'zIndex',
  'fillOpacity',
  'floodOpacity',
  'rotate',
  'scale',
  'shapeImageThreshold',
  'stopOpacity',
  'strokeDasharray',
  'strokeDashoffset',
  'strokeMiterlimit',
  'strokeOpacity',
  'strokeWidth',
  'scale',
  'mathDepth',
  'zoom',
]);

// List of properties that have custom suffixes for numbers
const numberPropertySuffixes: { +[key: string]: string } = {
  animationDelay: 'ms',
  animationDuration: 'ms',
  transitionDelay: 'ms',
  transitionDuration: 'ms',
  voiceDuration: 'ms',
};

export const timeUnits: Set<string> = new Set(
  Object.keys(numberPropertySuffixes),
);

export const lengthUnits: Set<string> = new Set([
  'backgroundPositionX',
  'backgroundPositionY',
  'blockSize',
  'borderBlockEndWidth',
  'borderBlockStartWidth',
  'borderBlockWidth',
  'borderVerticalWidth',
  'borderVerticalWidth',
  'borderBottomLeftRadius',
  'borderBottomRightRadius',
  'borderBottomWidth',
  'borderEndEndRadius',
  'borderEndStartRadius',
  'borderInlineEndWidth',
  'borderEndWidth',
  'borderInlineStartWidth',
  'borderStartWidth',
  'borderInlineWidth',
  'borderHorizontalWidth',
  'borderLeftWidth',
  'borderRightWidth',
  'borderSpacing',
  'borderStartEndRadius',
  'borderStartStartRadius',
  'borderTopLeftRadius',
  'borderTopRightRadius',
  'borderTopWidth',
  'bottom',
  'columnGap',
  'columnRuleWidth',
  'columnWidth',
  'containIntrinsicBlockSize',
  'containIntrinsicHeight',
  'containIntrinsicInlineSize',
  'containIntrinsicWidth',
  'flexBasis',
  'fontSize',
  'fontSmooth',
  'height',
  'inlineSize',
  'insetBlockEnd',
  'insetBlockStart',
  'insetInlineEnd',
  'insetInlineStart',
  'left',
  'letterSpacing',
  'marginBlockEnd',
  'marginBlockStart',
  'marginBottom',
  'marginInlineEnd',
  'marginEnd',
  'marginInlineStart',
  'marginStart',
  'marginLeft',
  'marginRight',
  'marginTop',
  'maxBlockSize',
  'maxHeight',
  'maxInlineSize',
  'maxWidth',
  'minBlockSize',
  'minHeight',
  'minInlineSize',
  'minWidth',
  'offsetDistance',
  'outlineOffset',
  'outlineWidth',
  'overflowClipMargin',
  'paddingBlockEnd',
  'paddingBlockStart',
  'paddingBottom',
  'paddingInlineEnd',
  'paddingEnd',
  'paddingInlineStart',
  'paddingStart',
  'paddingLeft',
  'paddingRight',
  'paddingTop',
  'perspective',
  'right',
  'rowGap',
  'scrollMarginBlockEnd',
  'scrollMarginBlockStart',
  'scrollMarginBottom',
  'scrollMarginInlineEnd',
  'scrollMarginInlineStart',
  'scrollMarginLeft',
  'scrollMarginRight',
  'scrollMarginTop',
  'scrollPaddingBlockEnd',
  'scrollPaddingBlockStart',
  'scrollPaddingBottom',
  'scrollPaddingInlineEnd',
  'scrollPaddingInlineStart',
  'scrollPaddingLeft',
  'scrollPaddingRight',
  'scrollPaddingTop',
  'scrollSnapMarginBottom',
  'scrollSnapMarginLeft',
  'scrollSnapMarginRight',
  'scrollSnapMarginTop',
  'shapeMargin',
  'tabSize',
  'textDecorationThickness',
  'textIndent',
  'textUnderlineOffset',
  'top',
  'transformOrigin',
  'translate',
  'verticalAlign',
  'width',
  'wordSpacing',
  'border',
  'borderBlock',
  'borderBlockEnd',
  'borderBlockStart',
  'borderBottom',
  'borderLeft',
  'borderRadius',
  'borderRight',
  'borderTop',
  'borderWidth',
  'columnRule',
  'containIntrinsicSize',
  'gap',
  'inset',
  'insetBlock',
  'insetInline',
  'margin',
  'marginBlock',
  'marginVertical',
  'marginInline',
  'marginHorizontal',
  'offset',
  'outline',
  'padding',
  'paddingBlock',
  'paddingVertical',
  'paddingInline',
  'paddingHorizontal',
  'scrollMargin',
  'scrollMarginBlock',
  'scrollMarginInline',
  'scrollPadding',
  'scrollPaddingBlock',
  'scrollPaddingInline',
  'scrollSnapMargin',
]);
