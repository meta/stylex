/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';
import namedColors from '../reference/namedColors';
import type { Expression, Node, Pattern, Property } from 'estree';
import type { Rule } from 'eslint';
import isCSSVariable from '../rules/isCSSVariable';
import makeLiteralRule from '../rules/makeLiteralRule';
import makeRangeRule from '../rules/makeRangeRule';
import makeRegExRule from '../rules/makeRegExRule';
import isString from '../rules/isString';
import isHexColor from '../rules/isHexColor';
import makeUnionRule from '../rules/makeUnionRule';
import isNumber from '../rules/isNumber';
import isPercentage from '../rules/isPercentage';
import isAbsoluteLength from '../rules/isAbsoluteLength';
import isRelativeLength from '../rules/isRelativeLength';
import { borderSplitter } from '../utils/split-css-value';
import formatPropertiesWithNodeIndentation from '../utils/formatPropertiesWithNodeIndentation';
import getSourceCode from '../utils/getSourceCode';

export type RuleResponse = void | {
  message: string,
  distance?: number,
  fix?: Rule.ReportFixer,
  suggest?: {
    fix: Rule.ReportFixer,
    desc: string,
  },
};
export type RuleCheck = (
  node: $ReadOnly<Expression | Pattern>,
  variables?: Variables,
  prop?: $ReadOnly<Property>,
  context?: Rule.RuleContext,
) => RuleResponse;

export type Variables = $ReadOnlyMap<string, Expression | 'ARG'>;

const showError =
  (message: string): RuleCheck =>
  () => ({ message });

const isStringOrNumber: RuleCheck = makeUnionRule(isString, isNumber);

const isNamedColor: RuleCheck = makeUnionRule(
  ...Array.from(namedColors).map((color) => makeLiteralRule(color)),
);

const isLength: RuleCheck = makeUnionRule(isAbsoluteLength, isRelativeLength);

const isNonNumericString: RuleCheck = (
  node: Node,
  _variables?: Variables,
  _prop?: Property,
): RuleResponse => {
  if (node.type === 'Literal' && typeof node.value === 'string') {
    if (/^[-+]?(?:\d+|\d*\.\d+)$/.test(node.value)) {
      if (node.value === '0') {
        return undefined;
      }

      const response: $NonMaybeType<RuleResponse> = {
        message: 'a non-numeric string',
      };
      response.suggest = {
        desc: `Replace string '${node.value}' with number ${Number(node.value)}?`,
        fix: (fixer: Rule.RuleFixer): Rule.Fix | null => {
          return fixer.replaceText(node, String(Number(node.value)));
        },
      };
      return response;
    }
  }
  return undefined;
};

// NOTE: converted from Flow types to function calls using this
// https://astexplorer.net/#/gist/87e64b378349f13e885f9b6968c1e556/4b4ff0358de33cf86b8b21d29c17504d789babf9
export const all: RuleCheck = makeUnionRule(
  makeLiteralRule(null),
  makeLiteralRule('initial'),
  makeLiteralRule('inherit'),
  makeLiteralRule('unset'),
  makeLiteralRule('revert'),
);

const length: RuleCheck = makeUnionRule(isNumber, isNonNumericString);

const color: RuleCheck = makeUnionRule(isString, isNamedColor, isHexColor);
const width: RuleCheck = makeUnionRule(
  isNonNumericString,
  isNumber,
  makeLiteralRule('available'),
  makeLiteralRule('min-content'),
  makeLiteralRule('max-content'),
  makeLiteralRule('fit-content'),
  makeLiteralRule('auto'),
  isLength,
  isPercentage,
);
const borderWidth: RuleCheck = makeUnionRule(
  isNumber,
  makeLiteralRule('thin'),
  makeLiteralRule('medium'),
  makeLiteralRule('thick'),
  isNonNumericString,
  isLength,
);
const lengthPercentage: RuleCheck = isStringOrNumber;
const borderImageSource: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  isString,
);
const time: RuleCheck = makeUnionRule(isNumber, isString);
const animationDirection: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('reverse'),
  makeLiteralRule('alternate'),
  makeLiteralRule('alternate-reverse'),
);
const animationFillMode: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('forwards'),
  makeLiteralRule('backwards'),
  makeLiteralRule('both'),
);
const animationIterationCount: RuleCheck = makeUnionRule(
  makeLiteralRule('infinite'),
  isNumber,
);
const animationPlayState: RuleCheck = makeUnionRule(
  makeLiteralRule('running'),
  makeLiteralRule('paused'),
);
const animationTimingFunction: RuleCheck = makeUnionRule(
  makeLiteralRule('ease'),
  makeLiteralRule('linear'),
  makeLiteralRule('ease-in'),
  makeLiteralRule('ease-out'),
  makeLiteralRule('ease-in-out'),
  makeLiteralRule('step-start'),
  makeLiteralRule('step-end'),
  isString,
);
const attachment: RuleCheck = makeUnionRule(
  makeLiteralRule('scroll'),
  makeLiteralRule('fixed'),
  makeLiteralRule('local'),
);
const blendMode: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('multiply'),
  makeLiteralRule('screen'),
  makeLiteralRule('overlay'),
  makeLiteralRule('darken'),
  makeLiteralRule('lighten'),
  makeLiteralRule('color-dodge'),
  makeLiteralRule('color-burn'),
  makeLiteralRule('hard-light'),
  makeLiteralRule('soft-light'),
  makeLiteralRule('difference'),
  makeLiteralRule('exclusion'),
  makeLiteralRule('hue'),
  makeLiteralRule('saturation'),
  makeLiteralRule('color'),
  makeLiteralRule('luminosity'),
);
const bgSize: RuleCheck = makeUnionRule(
  isString,
  makeLiteralRule('cover'),
  makeLiteralRule('contain'),
);
const boxAlign: RuleCheck = makeUnionRule(
  makeLiteralRule('start'),
  makeLiteralRule('center'),
  makeLiteralRule('end'),
  makeLiteralRule('baseline'),
  makeLiteralRule('stretch'),
);
const repeatStyle: RuleCheck = makeUnionRule(
  makeLiteralRule('repeat-x'),
  makeLiteralRule('repeat-y'),
  isString,
);
const backgroundPosition: RuleCheck = makeUnionRule(
  isString,
  makeLiteralRule('top'),
  makeLiteralRule('bottom'),
  makeLiteralRule('left'),
  makeLiteralRule('right'),
  makeLiteralRule('center'),
);
const backgroundPositionX: RuleCheck = makeUnionRule(
  isNumber,
  isString,
  makeLiteralRule('left'),
  makeLiteralRule('right'),
  makeLiteralRule('center'),
);
const backgroundPositionY: RuleCheck = makeUnionRule(
  isNumber,
  isString,
  makeLiteralRule('top'),
  makeLiteralRule('bottom'),
  makeLiteralRule('center'),
);
const borderImageOutset: RuleCheck = isString;
const borderImageRepeat: RuleCheck = makeUnionRule(
  isString,
  makeLiteralRule('stretch'),
  makeLiteralRule('repeat'),
  makeLiteralRule('round'),
  makeLiteralRule('space'),
);
const borderImageWidth: RuleCheck = isString;
const borderImageSlice: RuleCheck = makeUnionRule(
  isStringOrNumber,
  makeLiteralRule('fill'),
);
const box: RuleCheck = makeUnionRule(
  makeLiteralRule('border-box'),
  makeLiteralRule('padding-box'),
  makeLiteralRule('content-box'),
);
const brStyle: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('hidden'),
  makeLiteralRule('dotted'),
  makeLiteralRule('dashed'),
  makeLiteralRule('solid'),
  makeLiteralRule('double'),
  makeLiteralRule('groove'),
  makeLiteralRule('ridge'),
  makeLiteralRule('inset'),
  makeLiteralRule('outset'),
);
const CSSCursor: RuleCheck = makeUnionRule(
  isCSSVariable,
  makeLiteralRule('auto'),
  makeLiteralRule('default'),
  makeLiteralRule('none'),
  makeLiteralRule('context-menu'),
  makeLiteralRule('help'),
  makeLiteralRule('pointer'),
  makeLiteralRule('progress'),
  makeLiteralRule('wait'),
  makeLiteralRule('cell'),
  makeLiteralRule('crosshair'),
  makeLiteralRule('text'),
  makeLiteralRule('vertical-text'),
  makeLiteralRule('alias'),
  makeLiteralRule('copy'),
  makeLiteralRule('move'),
  makeLiteralRule('no-drop'),
  makeLiteralRule('not-allowed'),
  makeLiteralRule('e-resize'),
  makeLiteralRule('n-resize'),
  makeLiteralRule('ne-resize'),
  makeLiteralRule('nw-resize'),
  makeLiteralRule('s-resize'),
  makeLiteralRule('se-resize'),
  makeLiteralRule('sw-resize'),
  makeLiteralRule('w-resize'),
  makeLiteralRule('ew-resize'),
  makeLiteralRule('ns-resize'),
  makeLiteralRule('nesw-resize'),
  makeLiteralRule('nwse-resize'),
  makeLiteralRule('col-resize'),
  makeLiteralRule('row-resize'),
  makeLiteralRule('all-scroll'),
  makeLiteralRule('zoom-in'),
  makeLiteralRule('zoom-out'),
  makeLiteralRule('grab'),
  makeLiteralRule('grabbing'),
  makeLiteralRule('-webkit-grab'),
  makeLiteralRule('-webkit-grabbing'),
);
const relativeSize: RuleCheck = makeUnionRule(
  makeLiteralRule('larger'),
  makeLiteralRule('smaller'),
);
const emptyCells: RuleCheck = makeUnionRule(
  makeLiteralRule('show'),
  makeLiteralRule('hide'),
);
const filter: RuleCheck = makeUnionRule(makeLiteralRule('none'), isString);
// const flex = makeUnionRule(makeLiteralRule('none'), isString, isNumber);
const flexBasis: RuleCheck = makeUnionRule(
  makeLiteralRule('content'),
  isNumber,
  isString,
);
const flexDirection: RuleCheck = makeUnionRule(
  makeLiteralRule('row'),
  makeLiteralRule('row-reverse'),
  makeLiteralRule('column'),
  makeLiteralRule('column-reverse'),
);
const flexWrap: RuleCheck = makeUnionRule(
  makeLiteralRule('nowrap'),
  makeLiteralRule('wrap'),
  makeLiteralRule('wrap-reverse'),
);
const flexGrow: RuleCheck = isStringOrNumber;
const flexShrink: RuleCheck = isStringOrNumber;
const flexFlow: RuleCheck = makeUnionRule(flexDirection, flexWrap);
const float: RuleCheck = makeUnionRule(
  makeLiteralRule('left'),
  makeLiteralRule('right'),
  makeLiteralRule('none'),
  makeLiteralRule('inline-start'),
  makeLiteralRule('inline-end'),
);
const absoluteSize: RuleCheck = makeUnionRule(
  makeLiteralRule('xx-small'),
  makeLiteralRule('x-small'),
  makeLiteralRule('small'),
  makeLiteralRule('medium'),
  makeLiteralRule('large'),
  makeLiteralRule('x-large'),
  makeLiteralRule('xx-large'),
);
const fontFamily: RuleCheck = isString;
const gridLine: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  isString,
  isNumber,
);
const gridTemplate: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('subgrid'),
  isString,
);
const gridTemplateAreas: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  isString,
);
const trackBreadth: RuleCheck = makeUnionRule(
  lengthPercentage,
  isString,
  makeLiteralRule('min-content'),
  makeLiteralRule('max-content'),
  makeLiteralRule('auto'),
);
const listStyleType: RuleCheck = makeUnionRule(
  isString,
  makeLiteralRule('none'),
);
const trackSize: RuleCheck = makeUnionRule(trackBreadth, isString);
const borderStyle: RuleCheck = brStyle;
const columnRuleColor: RuleCheck = color;
const columnRuleStyle: RuleCheck = brStyle;
const columnRuleWidth: RuleCheck = borderWidth;
const columnRule: RuleCheck = makeUnionRule(
  columnRuleWidth,
  columnRuleStyle,
  columnRuleColor,
);
const shapeBox: RuleCheck = makeUnionRule(box, makeLiteralRule('margin-box'));
const geometryBox: RuleCheck = makeUnionRule(
  shapeBox,
  makeLiteralRule('fill-box'),
  makeLiteralRule('stroke-box'),
  makeLiteralRule('view-box'),
);
const maskReference: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  isString,
);
const compositeOperator: RuleCheck = makeUnionRule(
  makeLiteralRule('add'),
  makeLiteralRule('subtract'),
  makeLiteralRule('intersect'),
  makeLiteralRule('exclude'),
);
const maskingMode: RuleCheck = makeUnionRule(
  makeLiteralRule('alpha'),
  makeLiteralRule('luminance'),
  makeLiteralRule('match-source'),
);
const maskLayer: RuleCheck = makeUnionRule(
  maskReference,
  maskingMode,
  isString,
  repeatStyle,
  geometryBox,
  compositeOperator,
);

const alignBase: RuleCheck = makeUnionRule(
  'normal',
  'stretch',
  'center',
  'start',
  'end',
  'flex-start',
  'flex-end',
  'baseline',
  'first baseline',
  'last baseline',
  'safe center',
  'unsafe center',
);

const alignContent: RuleCheck = makeUnionRule(
  alignBase,
  'space-between',
  'space-around',
  'space-evenly',
);

const alignItems: RuleCheck = makeUnionRule(
  alignBase,
  'self-start',
  'self-end',
);

const alignSelf: RuleCheck = makeUnionRule(
  alignBase,
  'auto',
  'self-start',
  'self-end',
);
const appearance: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('none'),
  makeLiteralRule('textfield'),
);
const backdropFilter: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  isString,
);
const backfaceVisibility: RuleCheck = makeUnionRule(
  makeLiteralRule('visible'),
  makeLiteralRule('hidden'),
);
// type background = string | finalBgLayer;
const backgroundAttachment: RuleCheck = attachment;
const backgroundBlendMode: RuleCheck = (
  node: Expression | Pattern,
  _variables?: Variables,
  prop?: Property,
) => {
  if (node.type !== 'Literal' || prop == null) {
    return blendMode(node, _variables, prop);
  }

  if (typeof node.value === 'string') {
    const value: string = node.value;
    const items = value.split(', ');
    if (value.split(',').length !== items.length) {
      return {
        message:
          "backgroundBlendMode values must be separated by a comma and a space (', ')",
        suggest: {
          desc: 'Replace comma with a comma and a space (", ")',
          fix: (fixer: Rule.RuleFixer): Rule.Fix | null => {
            return fixer.replaceText(
              prop,
              `backgroundBlendMode: '${value.replace(',', ', ')}'`,
            );
          },
        },
      };
    }
    for (const item of items) {
      const response = blendMode(
        { type: 'Literal', value: item, raw: `'${item}'` },
        _variables,
        prop,
      );
      if (response !== undefined) {
        return {
          message: response.message,
        };
      }
    }

    return undefined;
  }
};
const backgroundClip: RuleCheck = makeUnionRule(
  'border-box',
  'padding-box',
  'content-box',
  'text',
);
const backgroundColor: RuleCheck = color;
const backgroundImage: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  isString,
);
const backgroundOrigin: RuleCheck = box;
const backgroundRepeat: RuleCheck = repeatStyle;
const backgroundSize: RuleCheck = bgSize;
const blockSize: RuleCheck = width;
const NUMERIC_LITERAL_VALUE_REGEX = /^[-+]?(?:\d+|\d*\.\d+)$/;

const NUMERIC_LITERAL_PROPERTIES = new Set([
  'lineHeight',
  'fontWeight',
  'animationIterationCount',
  'borderWidth',
  'borderTopWidth',
  'borderRightWidth',
  'borderBottomWidth',
  'borderLeftWidth',
  'borderInlineStartWidth',
  'borderInlineEndWidth',
  'borderBlockStartWidth',
  'borderBlockEndWidth',
]);

const serializeValue = (propertyKey: string, val: number | string): string => {
  if (typeof val === 'number') {
    return String(val);
  }
  if (
    NUMERIC_LITERAL_PROPERTIES.has(propertyKey) &&
    NUMERIC_LITERAL_VALUE_REGEX.test(val)
  ) {
    return String(Number(val));
  }
  // Escape single quotes within the string
  const escaped = val.replace(/\\/g, '\\\\').replace(/'/g, "\\'");
  return `'${escaped}'`;
};

const formatReplacementProperties = (
  prop: $ReadOnly<Property>,
  properties: $ReadOnlyArray<string>,
  context?: Rule.RuleContext,
): string => {
  const sourceCode = context != null ? getSourceCode(context) : undefined;
  return formatPropertiesWithNodeIndentation(prop, properties, sourceCode);
};

const border =
  (suffix: string = ''): RuleCheck =>
  (
    node: Expression | Pattern,
    _variables?: Variables,
    prop?: Property,
    context?: Rule.RuleContext,
  ) => {
    const response: $NonMaybeType<RuleResponse> = {
      message: `The 'border${suffix}' property is not supported. Use the 'border${suffix}Width', 'border${suffix}Style' and 'border${suffix}Color' properties instead.`,
    };
    if (node.type !== 'Literal' || prop == null) {
      return response;
    }
    if (typeof node.value === 'number') {
      const fixFn = (fixer: Rule.RuleFixer): Rule.Fix | null => {
        return fixer.replaceText(
          prop,
          `border${suffix}Width: ${String(node.value)}`,
        );
      };
      response.fix = fixFn;
      response.suggest = {
        desc: `Replace 'border${suffix}' set to a number with 'border${suffix}Width' instead?`,
        fix: fixFn,
      };
    }
    if (typeof node.value === 'string') {
      const [width, style, color] = borderSplitter(node.value);
      if (width != null || style != null || color != null) {
        const fixFn = (fixer: Rule.RuleFixer): Rule.Fix | null => {
          const newRules = [];
          if (width != null) {
            newRules.push(
              `border${suffix}Width: ${serializeValue(`border${suffix}Width`, width)}`,
            );
          }
          if (style != null) {
            newRules.push(
              `border${suffix}Style: ${serializeValue(`border${suffix}Style`, style)}`,
            );
          }
          if (color != null) {
            newRules.push(
              `border${suffix}Color: ${serializeValue(`border${suffix}Color`, color)}`,
            );
          }
          return fixer.replaceText(
            prop,
            formatReplacementProperties(prop, newRules, context),
          );
        };
        response.fix = fixFn;
        response.suggest = {
          desc: `Replace 'border${suffix}' with 'border${suffix}Width', 'border${suffix}Style' and 'border${suffix}Color' instead?`,
          fix: fixFn,
        };
      }
    }
    return response;
  };
const borderBottomStyle: RuleCheck = brStyle;
const borderBottomWidth: RuleCheck = borderWidth;
const borderCollapse: RuleCheck = makeUnionRule(
  makeLiteralRule('collapse'),
  makeLiteralRule('separate'),
);
const borderColor: RuleCheck = color;
const borderBlockOrInline: RuleCheck = makeUnionRule(
  borderWidth,
  borderStyle,
  color,
);
const borderImage: RuleCheck = makeUnionRule(
  borderImageSource,
  borderImageSlice,
  isString,
  borderImageRepeat,
);
const borderLeftColor: RuleCheck = color;
const borderLeftStyle: RuleCheck = brStyle;
const borderLeftWidth: RuleCheck = borderWidth;
const borderSpacing: RuleCheck = isStringOrNumber;
const cornerShape: RuleCheck = isString;
const borderTopStyle: RuleCheck = brStyle;
const borderTopWidth: RuleCheck = borderWidth;
const boxDecorationBreak: RuleCheck = makeUnionRule(
  makeLiteralRule('slice'),
  makeLiteralRule('clone'),
);
const boxDirection: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('reverse'),
);
const boxFlex: RuleCheck = isStringOrNumber;
const boxFlexGroup: RuleCheck = isStringOrNumber;
const boxLines: RuleCheck = makeUnionRule(
  makeLiteralRule('single'),
  makeLiteralRule('multiple'),
);
const boxOrdinalGroup: RuleCheck = isStringOrNumber;
const boxOrient: RuleCheck = makeUnionRule(
  makeLiteralRule('horizontal'),
  makeLiteralRule('vertical'),
  makeLiteralRule('inline-axis'),
  makeLiteralRule('block-axis'),
);
const boxShadow: RuleCheck = makeUnionRule(makeLiteralRule('none'), isString);
const boxSizing: RuleCheck = makeUnionRule(
  makeLiteralRule('content-box'),
  makeLiteralRule('border-box'),
);
const boxSuppress: RuleCheck = makeUnionRule(
  makeLiteralRule('show'),
  makeLiteralRule('discard'),
  makeLiteralRule('hide'),
);
const breakBeforeOrAfter: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('avoid'),
  makeLiteralRule('avoid-page'),
  makeLiteralRule('page'),
  makeLiteralRule('left'),
  makeLiteralRule('right'),
  makeLiteralRule('recto'),
  makeLiteralRule('verso'),
  makeLiteralRule('avoid-column'),
  makeLiteralRule('column'),
  makeLiteralRule('avoid-region'),
  makeLiteralRule('region'),
);
const breakInside: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('avoid'),
  makeLiteralRule('avoid-page'),
  makeLiteralRule('avoid-column'),
  makeLiteralRule('avoid-region'),
);
const captionSide: RuleCheck = makeUnionRule(
  makeLiteralRule('top'),
  makeLiteralRule('bottom'),
  makeLiteralRule('block-start'),
  makeLiteralRule('block-end'),
  makeLiteralRule('inline-start'),
  makeLiteralRule('inline-end'),
);
const clear: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('left'),
  makeLiteralRule('right'),
  makeLiteralRule('both'),
  makeLiteralRule('inline-start'),
  makeLiteralRule('inline-end'),
);
const clip: RuleCheck = makeUnionRule(isString, makeLiteralRule('auto'));
const clipPath: RuleCheck = makeUnionRule(isString, makeLiteralRule('none'));
const columnCount: RuleCheck = makeUnionRule(
  isNumber,
  isString,
  makeLiteralRule('auto'),
);
const columnFill: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('balance'),
);
const columnGap: RuleCheck = makeUnionRule(
  isNumber,
  isNonNumericString,
  makeLiteralRule('normal'),
);
const columnSpan: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('all'),
);
const columnWidth: RuleCheck = makeUnionRule(
  isNumber,
  isNonNumericString,
  makeLiteralRule('auto'),
);
const columns: RuleCheck = makeUnionRule(columnWidth, columnCount);
const contain: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('strict'),
  makeLiteralRule('content'),
  isString,
);
const content: RuleCheck = isString;
const counterIncrement: RuleCheck = makeUnionRule(
  isString,
  makeLiteralRule('none'),
);
const counterReset: RuleCheck = makeUnionRule(
  isString,
  makeLiteralRule('none'),
);
const cursor: RuleCheck = CSSCursor;
const direction: RuleCheck = makeUnionRule(
  makeLiteralRule('ltr'),
  makeLiteralRule('rtl'),
);
const display: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('inline'),
  makeLiteralRule('block'),
  makeLiteralRule('flow-root'),
  makeLiteralRule('list-item'),
  makeLiteralRule('inline-list-item'),
  makeLiteralRule('inline-block'),
  makeLiteralRule('inline-table'),
  makeLiteralRule('table'),
  makeLiteralRule('table-cell'),
  makeLiteralRule('table-column'),
  makeLiteralRule('table-column-group'),
  makeLiteralRule('table-footer-group'),
  makeLiteralRule('table-header-group'),
  makeLiteralRule('table-row'),
  makeLiteralRule('table-row-group'),
  makeLiteralRule('flex'),
  makeLiteralRule('inline-flex'),
  makeLiteralRule('grid'),
  makeLiteralRule('inline-grid'),
  makeLiteralRule('-webkit-box'),
  makeLiteralRule('run-in'),
  makeLiteralRule('ruby'),
  makeLiteralRule('ruby-base'),
  makeLiteralRule('ruby-text'),
  makeLiteralRule('ruby-base-container'),
  makeLiteralRule('ruby-text-container'),
  makeLiteralRule('contents'),
);
const displayInside: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('block'),
  makeLiteralRule('table'),
  makeLiteralRule('flex'),
  makeLiteralRule('grid'),
  makeLiteralRule('ruby'),
);
const displayList: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('list-item'),
);
const displayOutside: RuleCheck = makeUnionRule(
  makeLiteralRule('block-level'),
  makeLiteralRule('inline-level'),
  makeLiteralRule('run-in'),
  makeLiteralRule('contents'),
  makeLiteralRule('none'),
  makeLiteralRule('table-row-group'),
  makeLiteralRule('table-header-group'),
  makeLiteralRule('table-footer-group'),
  makeLiteralRule('table-row'),
  makeLiteralRule('table-cell'),
  makeLiteralRule('table-column-group'),
  makeLiteralRule('table-column'),
  makeLiteralRule('table-caption'),
  makeLiteralRule('ruby-base'),
  makeLiteralRule('ruby-text'),
  makeLiteralRule('ruby-base-container'),
  makeLiteralRule('ruby-text-container'),
);
const fontFeatureSettings: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  isString,
);
const fontKerning: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('normal'),
  makeLiteralRule('none'),
);
const fontLanguageOverride: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  isString,
);
const fontSize: RuleCheck = makeUnionRule(
  absoluteSize,
  relativeSize,
  lengthPercentage,
);
const fontSizeAdjust: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  isNumber,
  isString,
);
const fontStretch: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('ultra-condensed'),
  makeLiteralRule('extra-condensed'),
  makeLiteralRule('condensed'),
  makeLiteralRule('semi-condensed'),
  makeLiteralRule('semi-expanded'),
  makeLiteralRule('expanded'),
  makeLiteralRule('extra-expanded'),
  makeLiteralRule('ultra-expanded'),
);
const fontStyle: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('italic'),
  makeLiteralRule('oblique'),
);
const fontSynthesis: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  isString,
);
const fontVariant: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('none'),
  isString,
);
const fontVariantAlternates: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  isString,
);
const fontVariantCaps: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('small-caps'),
  makeLiteralRule('all-small-caps'),
  makeLiteralRule('petite-caps'),
  makeLiteralRule('all-petite-caps'),
  makeLiteralRule('unicase'),
  makeLiteralRule('titling-caps'),
);
const fontVariantEastAsian: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  isString,
);
const fontVariantLigatures: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('none'),
  isString,
);
const fontVariantNumeric: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  isString,
);
const fontVariantPosition: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('sub'),
  makeLiteralRule('super'),
);
const fontWeight: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('bold'),
  makeLiteralRule('bolder'),
  makeLiteralRule('lighter'),
  makeRangeRule(1, 1000, 'a number between 1 and 1000'),
  isCSSVariable,
);
const gap: RuleCheck = length;
const grid: RuleCheck = makeUnionRule(gridTemplate, isString);
const gridArea: RuleCheck = makeUnionRule(gridLine, isString);
const gridAutoColumns: RuleCheck = trackSize;
const gridAutoFlow: RuleCheck = makeUnionRule(
  isString,
  makeLiteralRule('dense'),
);
const gridAutoRows: RuleCheck = trackSize;
const gridColumn: RuleCheck = makeUnionRule(gridLine, isString);
const gridColumnEnd: RuleCheck = gridLine;
const gridColumnStart: RuleCheck = gridLine;
const gridRow: RuleCheck = makeUnionRule(gridLine, isString);
const gridRowEnd: RuleCheck = gridLine;
const gridRowStart: RuleCheck = gridLine;
const hyphens: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('manual'),
  makeLiteralRule('auto'),
);
const imageOrientation: RuleCheck = makeUnionRule(
  makeLiteralRule('from-image'),
  isNumber,
  isString,
);
const imageRendering: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('crisp-edges'),
  makeLiteralRule('pixelated'),
  makeLiteralRule('optimizeSpeed'),
  makeLiteralRule('optimizeQuality'),
  isString,
);
const imageResolution: RuleCheck = makeUnionRule(
  isString,
  makeLiteralRule('snap'),
);
const imeMode: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('normal'),
  makeLiteralRule('active'),
  makeLiteralRule('inactive'),
  makeLiteralRule('disabled'),
);
const initialLetter: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  isString,
);
const initialLetterAlign: RuleCheck = isString;
const inlineSize: RuleCheck = width;
const interpolateSize: RuleCheck = makeUnionRule(
  'allow-keywords',
  'numeric-only',
);
const isolation: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('isolate'),
);

const justifyBase: string[] = [
  'normal',
  'stretch',
  'center',
  'start',
  'end',
  'flex-start',
  'flex-end',
  'left',
  'right',
  'baseline',
  'first baseline',
  'last baseline',
  'safe center',
  'unsafe center',
];

const justifyContent: RuleCheck = makeUnionRule(
  ...justifyBase,
  'space-between',
  'space-around',
  'space-evenly',
);

const justifyItems: RuleCheck = makeUnionRule(
  ...justifyBase,
  'self-start',
  'self-end',
  'legacy right',
  'legacy left',
  'legacy center',
);

// There's an optional overflowPosition (safe vs unsafe) prefix to
// [selfPosition | 'left' | 'right']. It's not used on www, so, it's not added
// here.
const justifySelf: RuleCheck = makeUnionRule(
  'auto',
  ...justifyBase,
  'self-start',
  'self-end',
);

const letterSpacing: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  lengthPercentage,
);
const lineBreak: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('loose'),
  makeLiteralRule('normal'),
  makeLiteralRule('strict'),
);
const lineHeight: RuleCheck = length;
const listStyleImage: RuleCheck = makeUnionRule(
  isString,
  makeLiteralRule('none'),
);
const listStylePosition: RuleCheck = makeUnionRule(
  makeLiteralRule('inside'),
  makeLiteralRule('outside'),
);
const listStyle: RuleCheck = makeUnionRule(
  listStyleType,
  listStylePosition,
  listStyleImage,
);
const margin: RuleCheck = length;
const marginLeft: RuleCheck = makeUnionRule(
  isNumber,
  isNonNumericString,
  makeLiteralRule('auto'),
);
const marginTop: RuleCheck = makeUnionRule(
  isNumber,
  isNonNumericString,
  makeLiteralRule('auto'),
);
const markerOffset: RuleCheck = makeUnionRule(
  isNumber,
  makeLiteralRule('auto'),
);
const mask: RuleCheck = maskLayer;
const maskClip: RuleCheck = isString;
const maskComposite: RuleCheck = compositeOperator;
const maskMode: RuleCheck = maskingMode;
const maskOrigin: RuleCheck = geometryBox;
const maskPosition: RuleCheck = isString;
const maskRepeat: RuleCheck = repeatStyle;
const maskSize: RuleCheck = bgSize;
const maskType: RuleCheck = makeUnionRule(
  makeLiteralRule('luminance'),
  makeLiteralRule('alpha'),
);
const minMaxLength: RuleCheck = makeUnionRule(
  isNumber,
  isNonNumericString,
  makeLiteralRule('none'),
  makeLiteralRule('max-content'),
  makeLiteralRule('min-content'),
  makeLiteralRule('fit-content'),
  makeLiteralRule('fill-available'),
);

const mixBlendMode: RuleCheck = blendMode;
const motionPath: RuleCheck = makeUnionRule(
  isString,
  geometryBox,
  makeLiteralRule('none'),
);
const motionRotation: RuleCheck = isStringOrNumber;
const motion: RuleCheck = makeUnionRule(
  motionPath,
  lengthPercentage,
  motionRotation,
);
const objectFit: RuleCheck = makeUnionRule(
  makeLiteralRule('fill'),
  makeLiteralRule('contain'),
  makeLiteralRule('cover'),
  makeLiteralRule('none'),
  makeLiteralRule('scale-down'),
);
const objectPosition: RuleCheck = isString;
const offsetBlockEnd: RuleCheck = isString;
const offsetBlockStart: RuleCheck = isString;
const offsetInlineEnd: RuleCheck = isString;
const offsetInlineStart: RuleCheck = isString;
const opacity: RuleCheck = isStringOrNumber;
const order: RuleCheck = isStringOrNumber;
const orphans: RuleCheck = isStringOrNumber;
const outline: RuleCheck = isString;
const overflow: RuleCheck = makeUnionRule(
  makeLiteralRule('visible'),
  makeLiteralRule('hidden'),
  makeLiteralRule('clip'),
  makeLiteralRule('scroll'),
  makeLiteralRule('auto'),
);
const overflowAnchor: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('none'),
);
const overflowClipBox: RuleCheck = makeUnionRule(
  makeLiteralRule('padding-box'),
  makeLiteralRule('content-box'),
);
const overflowWrap: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('break-word'),
  makeLiteralRule('anywhere'),
);
const overflowDir: RuleCheck = makeUnionRule(
  makeLiteralRule('visible'),
  makeLiteralRule('hidden'),
  makeLiteralRule('clip'),
  makeLiteralRule('scroll'),
  makeLiteralRule('auto'),
);
const overscrollBehavior: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('contain'),
  makeLiteralRule('auto'),
);
const scrollSpacing: RuleCheck = makeUnionRule(isNumber, isString);
const timelineAxis: RuleCheck = makeUnionRule('block', 'inline', 'x', 'y');

const pageBreak: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('always'),
  makeLiteralRule('avoid'),
  makeLiteralRule('left'),
  makeLiteralRule('right'),
);
const pageBreakInside: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('avoid'),
);
// `auto | <custom-ident>` — any string is a valid named page
const page: RuleCheck = makeUnionRule(all, isString);
const perspective: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  isNumber,
  isString,
);
const perspectiveOrigin: RuleCheck = isString;
const pointerEvents: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('none'),
  makeLiteralRule('visiblePainted'),
  makeLiteralRule('visibleFill'),
  makeLiteralRule('visibleStroke'),
  makeLiteralRule('visible'),
  makeLiteralRule('painted'),
  makeLiteralRule('fill'),
  makeLiteralRule('stroke'),
  makeLiteralRule('all'),
);
const position: RuleCheck = makeUnionRule(
  makeLiteralRule('static'),
  makeLiteralRule('relative'),
  makeLiteralRule('absolute'),
  makeLiteralRule('sticky'),
  makeLiteralRule('fixed'),
);
const quotes: RuleCheck = makeUnionRule(isString, makeLiteralRule('none'));
const resize: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('both'),
  makeLiteralRule('horizontal'),
  makeLiteralRule('vertical'),
);
const rowGap: RuleCheck = length;
const rubyAlign: RuleCheck = makeUnionRule(
  makeLiteralRule('start'),
  makeLiteralRule('center'),
  makeLiteralRule('space-between'),
  makeLiteralRule('space-around'),
);
const rubyMerge: RuleCheck = makeUnionRule(
  makeLiteralRule('separate'),
  makeLiteralRule('collapse'),
  makeLiteralRule('auto'),
);
const rubyPosition: RuleCheck = makeUnionRule(
  makeLiteralRule('over'),
  makeLiteralRule('under'),
  makeLiteralRule('inter-character'),
);
const scrollBehavior: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('smooth'),
);
const scrollSnapPaddingBottom: RuleCheck = isStringOrNumber;
const scrollSnapPaddingTop: RuleCheck = isStringOrNumber;
const scrollSnapAlign: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('start'),
  makeLiteralRule('end'),
  makeLiteralRule('center'),
);
const scrollSnapType: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('x mandatory'),
  makeLiteralRule('y mandatory'),
);
const shapeImageThreshold: RuleCheck = isStringOrNumber;
const shapeOutside: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  shapeBox,
  isString,
);
const tabSize: RuleCheck = isStringOrNumber;
const tableLayout: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('fixed'),
);
const textAlign: RuleCheck = makeUnionRule(
  makeLiteralRule('start'),
  makeLiteralRule('end'),
  makeLiteralRule('left'),
  makeLiteralRule('right'),
  makeLiteralRule('center'),
  makeLiteralRule('justify'),
  makeLiteralRule('match-parent'),
);
const textAlignLast: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('start'),
  makeLiteralRule('end'),
  makeLiteralRule('left'),
  makeLiteralRule('right'),
  makeLiteralRule('center'),
  makeLiteralRule('justify'),
);
const textCombineUpright: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('all'),
  isString,
);
const textDecorationColor: RuleCheck = color;
const textDecorationLine: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('underline'),
  makeLiteralRule('overline'),
  makeLiteralRule('line-through'),
  makeLiteralRule('blink'),
  isString,
);
// const textDecorationSkip = makeUnionRule(makeLiteralRule('none'), isString);
const textDecorationStyle: RuleCheck = makeUnionRule(
  makeLiteralRule('solid'),
  makeLiteralRule('double'),
  makeLiteralRule('dotted'),
  makeLiteralRule('dashed'),
  makeLiteralRule('wavy'),
);
const textDecoration: RuleCheck = makeUnionRule(
  textDecorationLine,
  textDecorationStyle,
  textDecorationColor,
);
const textEmphasisColor: RuleCheck = color;
const textEmphasisPosition: RuleCheck = isString;
const textEmphasisStyle: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('filled'),
  makeLiteralRule('open'),
  makeLiteralRule('dot'),
  makeLiteralRule('circle'),
  makeLiteralRule('double-circle'),
  makeLiteralRule('triangle'),
  makeLiteralRule('filled sesame'),
  makeLiteralRule('open sesame'),
  isString,
);
const textEmphasis: RuleCheck = makeUnionRule(
  textEmphasisStyle,
  textEmphasisColor,
);
const textIndent: RuleCheck = makeUnionRule(
  lengthPercentage,
  makeLiteralRule('hanging'),
  makeLiteralRule('each-line'),
);
const textJustify: RuleCheck = makeUnionRule(
  'none',
  'auto',
  'inter-word',
  'inter-character',
  'distribute',
);
const textOrientation: RuleCheck = makeUnionRule(
  makeLiteralRule('mixed'),
  makeLiteralRule('upright'),
  makeLiteralRule('sideways'),
);
const textOverflow: RuleCheck = makeUnionRule(
  makeLiteralRule('clip'),
  makeLiteralRule('ellipsis'),
  isString,
);
const textRendering: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('optimizeSpeed'),
  makeLiteralRule('optimizeLegibility'),
  makeLiteralRule('geometricPrecision'),
);
const textShadow: RuleCheck = makeUnionRule(makeLiteralRule('none'), isString);
const textSizeAdjust: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('auto'),
  isString,
);
const textTransform: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('capitalize'),
  makeLiteralRule('uppercase'),
  makeLiteralRule('lowercase'),
  makeLiteralRule('full-width'),
);
const textUnderlinePosition: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('under'),
  makeLiteralRule('left'),
  makeLiteralRule('right'),
  isString,
);
const textUnderlineOffset: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  isNumber,
  isLength,
  isPercentage,
);
const touchAction: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('none'),
  isString,
  makeLiteralRule('manipulation'),
);
const transform: RuleCheck = makeUnionRule(makeLiteralRule('none'), isString);
const transformBox: RuleCheck = makeUnionRule(
  makeLiteralRule('border-box'),
  makeLiteralRule('fill-box'),
  makeLiteralRule('view-box'),
  makeLiteralRule('content-box'),
  makeLiteralRule('stroke-box'),
);
const transformOrigin: RuleCheck = isStringOrNumber;
const transformStyle: RuleCheck = makeUnionRule(
  makeLiteralRule('flat'),
  makeLiteralRule('preserve-3d'),
);
const transitionProperty: RuleCheck = isString;
const transitionTimingFunction: RuleCheck = animationTimingFunction;
const unicodeBidi: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('embed'),
  makeLiteralRule('isolate'),
  makeLiteralRule('bidi-override'),
  makeLiteralRule('isolate-override'),
  makeLiteralRule('plaintext'),
);
const userSelect: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('text'),
  makeLiteralRule('none'),
  makeLiteralRule('contain'),
  makeLiteralRule('all'),
);
const verticalAlign: RuleCheck = makeUnionRule(
  makeLiteralRule('baseline'),
  makeLiteralRule('sub'),
  makeLiteralRule('super'),
  makeLiteralRule('text-top'),
  makeLiteralRule('text-bottom'),
  makeLiteralRule('middle'),
  makeLiteralRule('top'),
  makeLiteralRule('bottom'),
  isString,
  isNumber,
);
const visibility: RuleCheck = makeUnionRule(
  makeLiteralRule('visible'),
  makeLiteralRule('hidden'),
  makeLiteralRule('collapse'),
);
const whiteSpace: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('pre'),
  makeLiteralRule('nowrap'),
  makeLiteralRule('pre-wrap'),
  makeLiteralRule('pre-line'),
  makeLiteralRule('break-spaces'),
);
const widows: RuleCheck = isStringOrNumber;
const animatableFeature: RuleCheck = makeUnionRule(
  makeLiteralRule('scroll-position'),
  makeLiteralRule('contents'),
  isString,
);
const willChange: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  animatableFeature,
);
const nonStandardWordBreak: RuleCheck = makeLiteralRule('break-word');
const wordBreak: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('break-all'),
  makeLiteralRule('keep-all'),
  nonStandardWordBreak,
);
const wordSpacing: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  lengthPercentage,
);
const wordWrap: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('break-word'),
);
const svgWritingMode: RuleCheck = makeUnionRule(
  makeLiteralRule('lr-tb'),
  makeLiteralRule('rl-tb'),
  makeLiteralRule('tb-rl'),
  makeLiteralRule('lr'),
  makeLiteralRule('rl'),
  makeLiteralRule('tb'),
);
const writingMode: RuleCheck = makeUnionRule(
  makeLiteralRule('horizontal-tb'),
  makeLiteralRule('vertical-rl'),
  makeLiteralRule('vertical-lr'),
  makeLiteralRule('sideways-rl'),
  makeLiteralRule('sideways-lr'),
  svgWritingMode,
);
const zIndex: RuleCheck = makeUnionRule(makeLiteralRule('auto'), isNumber);
const alignmentBaseline: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('baseline'),
  makeLiteralRule('before-edge'),
  makeLiteralRule('text-before-edge'),
  makeLiteralRule('middle'),
  makeLiteralRule('central'),
  makeLiteralRule('after-edge'),
  makeLiteralRule('text-after-edge'),
  makeLiteralRule('ideographic'),
  makeLiteralRule('alphabetic'),
  makeLiteralRule('hanging'),
  makeLiteralRule('mathematical'),
);
const svgLength: RuleCheck = isStringOrNumber;
const baselineShift: RuleCheck = makeUnionRule(
  makeLiteralRule('baseline'),
  makeLiteralRule('sub'),
  makeLiteralRule('super'),
  svgLength,
);
const behavior: RuleCheck = isString;
const clipRule: RuleCheck = makeUnionRule(
  makeLiteralRule('nonzero'),
  makeLiteralRule('evenodd'),
);
const cueAfter: RuleCheck = makeUnionRule(
  isStringOrNumber,
  makeLiteralRule('none'),
);
const cueBefore: RuleCheck = makeUnionRule(
  isStringOrNumber,
  makeLiteralRule('none'),
);
const cue: RuleCheck = makeUnionRule(cueBefore, cueAfter);
const dominantBaseline: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('use-script'),
  makeLiteralRule('no-change'),
  makeLiteralRule('reset-size'),
  makeLiteralRule('ideographic'),
  makeLiteralRule('alphabetic'),
  makeLiteralRule('hanging'),
  makeLiteralRule('mathematical'),
  makeLiteralRule('central'),
  makeLiteralRule('middle'),
  makeLiteralRule('text-after-edge'),
  makeLiteralRule('text-before-edge'),
);
const paint: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  makeLiteralRule('currentColor'),
  color,
  isString,
);
const fill: RuleCheck = paint;
const fillOpacity: RuleCheck = isStringOrNumber;
const fillRule: RuleCheck = makeUnionRule(
  makeLiteralRule('nonzero'),
  makeLiteralRule('evenodd'),
);
const glyphOrientationHorizontal: RuleCheck = isStringOrNumber;
const glyphOrientationVertical: RuleCheck = isStringOrNumber;
const kerning: RuleCheck = makeUnionRule(makeLiteralRule('auto'), svgLength);
const marker: RuleCheck = makeUnionRule(makeLiteralRule('none'), isString);
const markerEnd: RuleCheck = makeUnionRule(makeLiteralRule('none'), isString);
const markerMid: RuleCheck = makeUnionRule(makeLiteralRule('none'), isString);
const markerStart: RuleCheck = makeUnionRule(makeLiteralRule('none'), isString);
const pauseOrRest: RuleCheck = makeUnionRule(
  isNumber,
  makeLiteralRule('none'),
  makeLiteralRule('x-weak'),
  makeLiteralRule('weak'),
  makeLiteralRule('medium'),
  makeLiteralRule('strong'),
  makeLiteralRule('x-strong'),
);
const shapeRendering: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('optimizeSpeed'),
  makeLiteralRule('crispEdges'),
  makeLiteralRule('geometricPrecision'),
);
const src: RuleCheck = isString;
const speak: RuleCheck = makeUnionRule(
  makeLiteralRule('auto'),
  makeLiteralRule('none'),
  makeLiteralRule('normal'),
);
const speakAs: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('spell-out'),
  makeLiteralRule('digits'),
  isString,
);
const stroke: RuleCheck = paint;
const strokeDasharray: RuleCheck = makeUnionRule(
  makeLiteralRule('none'),
  isNumber,
  isString,
);
const strokeDashoffset: RuleCheck = svgLength;
const strokeLinecap: RuleCheck = makeUnionRule(
  makeLiteralRule('butt'),
  makeLiteralRule('round'),
  makeLiteralRule('square'),
);
const strokeLinejoin: RuleCheck = makeUnionRule(
  makeLiteralRule('miter'),
  makeLiteralRule('round'),
  makeLiteralRule('bevel'),
);
const strokeMiterlimit: RuleCheck = isStringOrNumber;
const strokeOpacity: RuleCheck = isStringOrNumber;
const strokeWidth: RuleCheck = svgLength;
const textAnchor: RuleCheck = makeUnionRule(
  makeLiteralRule('start'),
  makeLiteralRule('middle'),
  makeLiteralRule('end'),
);
const unicodeRange: RuleCheck = isString;
const voiceBalance: RuleCheck = makeUnionRule(
  isNumber,
  makeLiteralRule('left'),
  makeLiteralRule('center'),
  makeLiteralRule('right'),
  makeLiteralRule('leftwards'),
  makeLiteralRule('rightwards'),
);
const voiceDuration: RuleCheck = makeUnionRule(makeLiteralRule('auto'), time);
const voiceFamily: RuleCheck = makeUnionRule(
  isString,
  makeLiteralRule('preserve'),
);
const voicePitch: RuleCheck = makeUnionRule(
  isNumber,
  makeLiteralRule('absolute'),
  isString,
);
const voiceRange: RuleCheck = makeUnionRule(
  isNumber,
  makeLiteralRule('absolute'),
  isString,
);
const voiceRate: RuleCheck = isString;
const voiceStress: RuleCheck = makeUnionRule(
  makeLiteralRule('normal'),
  makeLiteralRule('strong'),
  makeLiteralRule('moderate'),
  makeLiteralRule('none'),
  makeLiteralRule('reduced'),
);
const voiceVolume: RuleCheck = makeUnionRule(
  makeLiteralRule('silent'),
  isString,
);
const maskImage: RuleCheck = maskReference;

const SupportedVendorSpecificCSSProperties = {
  MozOsxFontSmoothing: makeLiteralRule('grayscale') as RuleCheck,
  WebkitFontSmoothing: makeLiteralRule('antialiased') as RuleCheck,
  WebkitAppearance: appearance,
  WebkitTapHighlightColor: color,
  WebkitOverflowScrolling: makeLiteralRule('touch') as RuleCheck,

  WebkitBoxOrient: makeUnionRule(
    'horizontal',
    'vertical',
    'inline-axis',
    'block-axis',
  ) as RuleCheck,
  WebkitLineClamp: isStringOrNumber,

  WebkitMaskImage: maskImage,

  WebkitTextFillColor: color,
  textFillColor: color,
  WebkitTextStrokeWidth: borderWidth,
  WebkitTextStrokeColor: color,
  WebkitBackgroundClip: makeUnionRule(
    'border-box',
    'padding-box',
    'content-box',
    'text',
  ) as RuleCheck,
  WebkitAppRegion: makeUnionRule('drag', 'no-drag') as RuleCheck,
};

export const convertToStandardProperties: $ReadOnly<{
  [key: string]: ?string,
}> = {
  marginStart: 'marginInlineStart',
  marginEnd: 'marginInlineEnd',
  marginHorizontal: 'marginInline',
  marginVertical: 'marginBlock',

  paddingStart: 'paddingInlineStart',
  paddingEnd: 'paddingInlineEnd',
  paddingHorizontal: 'paddingInline',
  paddingVertical: 'paddingBlock',

  borderVerticalWidth: 'borderBlockWidth',
  borderVerticalStyle: 'borderBlockStyle',
  borderVerticalColor: 'borderBlockColor',
  borderHorizontalWidth: 'borderInlineWidth',
  borderHorizontalStyle: 'borderInlineStyle',
  borderHorizontalColor: 'borderInlineColor',
  borderStartWidth: 'borderInlineStartWidth',
  borderStartStyle: 'borderInlineStartStyle',
  borderStartColor: 'borderInlineStartColor',
  borderEndWidth: 'borderInlineEndWidth',
  borderEndStyle: 'borderInlineEndStyle',
  borderEndColor: 'borderInlineEndColor',

  borderTopStartRadius: 'borderStartStartRadius',
  borderTopEndRadius: 'borderStartEndRadius',
  borderBottomStartRadius: 'borderEndStartRadius',
  borderBottomEndRadius: 'borderEndEndRadius',

  end: 'insetInlineEnd',
  start: 'insetInlineStart',
};

export const SVGProperties: { [string]: RuleCheck } = {
  colorInterpolation: makeUnionRule('auto', 'sRGB', 'linearRGB'),
  // colorRendering: color,
  fill: fill,
  fillOpacity: fillOpacity,
  fillRule: fillRule,
  floodColor: color,
  floodOpacity: opacity,
  stopColor: color,
  stopOpacity: opacity,
  stroke: stroke,
  strokeDasharray: strokeDasharray,
  strokeDashoffset: strokeDashoffset,
  strokeLinecap: strokeLinecap,
  strokeLinejoin: strokeLinejoin,
  strokeMiterlimit: strokeMiterlimit,
  strokeOpacity: strokeOpacity,
  strokeWidth: strokeWidth,
  vectorEffect: makeUnionRule(
    'none',
    'non-scaling-stroke',
    'non-scaling-size',
    'non-rotation',
    'fixed-position',
  ),
};

/* eslint-disable object-shorthand */
// $FlowFixMe[cannot-spread-indexer]
const CSSProperties = {
  ...SupportedVendorSpecificCSSProperties,
  ...SVGProperties,
  accentColor: color,
  alignTracks: isString,
  alignContent: alignContent,
  alignItems: alignItems,
  alignSelf: alignSelf,
  alignmentBaseline: alignmentBaseline,
  all: all,
  animation: showError(
    '`animation` is not recommended. Please use `animationName`, `animationDuration`, etc. instead',
  ) as RuleCheck,
  animationComposition: makeUnionRule(
    'replace',
    'add',
    'accumulate',
  ) as RuleCheck,
  animationDelay: time,
  animationDirection: animationDirection,
  animationDuration: time,
  animationFillMode: animationFillMode,
  animationIterationCount: animationIterationCount,
  animationPlayState: animationPlayState,
  animationTimingFunction: animationTimingFunction,
  animationTimeline: isString,
  appearance: appearance,
  aspectRatio: isStringOrNumber,
  backdropFilter: backdropFilter,
  backfaceVisibility: backfaceVisibility,
  background: isString,
  backgroundAttachment: backgroundAttachment,
  backgroundBlendMode: backgroundBlendMode,
  backgroundClip: backgroundClip,
  backgroundColor: backgroundColor,
  backgroundImage: backgroundImage,
  backgroundOrigin: backgroundOrigin,
  backgroundPosition: backgroundPosition,
  backgroundPositionX: backgroundPositionX,
  backgroundPositionY: backgroundPositionY,
  backgroundRepeat: backgroundRepeat,
  backgroundSize: backgroundSize,
  baselineShift: baselineShift,
  behavior: behavior,

  borderCollapse: borderCollapse,

  borderImage: borderImage,
  borderImageWidth: borderImageWidth,
  borderImageOutset: borderImageOutset,
  borderImageRepeat: borderImageRepeat,
  borderImageSlice: borderImageSlice,
  borderImageSource: borderImageSource,

  borderWidth: borderWidth,
  borderStyle: borderStyle,
  borderColor: borderColor,
  borderTopWidth: borderTopWidth,
  borderTopStyle: borderTopStyle,
  borderTopColor: color,
  borderBottomWidth: borderBottomWidth,
  borderBottomStyle: borderBottomStyle,
  borderBottomColor: color,
  borderLeftColor: borderLeftColor,
  borderLeftStyle: borderLeftStyle,
  borderLeftWidth: borderLeftWidth,
  borderRightColor: borderLeftColor,
  borderRightStyle: borderLeftStyle,
  borderRightWidth: borderLeftWidth,
  borderInline: borderBlockOrInline,
  borderInlineEnd: showError(
    [
      '`borderInlineEnd` is not supported. Please use',
      '  - `borderInlineEndWidth`,',
      '  - `borderInlineEndStyle` and',
      '  - `borderInlineEndColor` instead',
    ].join('\n'),
  ) as RuleCheck,
  borderInlineColor: borderLeftColor,
  borderInlineStyle: borderLeftStyle,
  borderInlineWidth: borderLeftWidth,
  borderInlineEndColor: borderLeftColor,
  borderInlineEndStyle: borderLeftStyle,
  borderInlineEndWidth: borderLeftWidth,
  borderInlineStart: showError(
    [
      '`borderInlineStart` is not supported. Please use',
      '  - `borderInlineStartWidth`,',
      '  - `borderInlineStartStyle` and',
      '  - `borderInlineStartColor` instead',
    ].join('\n'),
  ) as RuleCheck,
  borderInlineStartColor: borderLeftColor,
  borderInlineStartStyle: borderLeftStyle,
  borderInlineStartWidth: borderLeftWidth,
  borderBlock: borderBlockOrInline,
  borderBlockEnd: showError(
    [
      '`borderBlockEnd` is not supported. Please use',
      '  - `borderBlockEndWidth`,',
      '  - `borderBlockEndStyle` and',
      '  - `borderBlockEndColor` instead',
    ].join('\n'),
  ) as RuleCheck,
  borderBlockColor: borderLeftColor,
  borderBlockStyle: borderLeftStyle,
  borderBlockWidth: borderLeftWidth,
  borderBlockEndColor: borderLeftColor,
  borderBlockEndStyle: borderLeftStyle,
  borderBlockEndWidth: borderLeftWidth,
  borderBlockStart: showError(
    [
      '`borderBlockStart` is not supported. Please use',
      '  - `borderBlockStartWidth`,',
      '  - `borderBlockStartStyle` and',
      '  - `borderBlockStartColor` instead',
    ].join('\n'),
  ) as RuleCheck,
  borderBlockStartColor: borderLeftColor,
  borderBlockStartStyle: borderLeftStyle,
  borderBlockStartWidth: borderLeftWidth,

  borderSpacing: borderSpacing,

  borderRadius: lengthPercentage,
  borderStartStartRadius: lengthPercentage,
  borderStartEndRadius: lengthPercentage,
  borderEndStartRadius: lengthPercentage,
  borderEndEndRadius: lengthPercentage,

  borderTopLeftRadius: lengthPercentage,
  borderTopRightRadius: lengthPercentage,
  borderBottomLeftRadius: lengthPercentage,
  borderBottomRightRadius: lengthPercentage,

  cornerShape: cornerShape,
  cornerStartStartShape: cornerShape,
  cornerStartEndShape: cornerShape,
  cornerEndStartShape: cornerShape,
  cornerEndEndShape: cornerShape,
  cornerTopLeftShape: cornerShape,
  cornerTopRightShape: cornerShape,
  cornerBottomLeftShape: cornerShape,
  cornerBottomRightShape: cornerShape,

  boxAlign: boxAlign,
  boxDecorationBreak: boxDecorationBreak,
  boxDirection: boxDirection,
  boxFlex: boxFlex,
  boxFlexGroup: boxFlexGroup,
  boxLines: boxLines,
  boxOrdinalGroup: boxOrdinalGroup,
  boxOrient: boxOrient,
  boxShadow: boxShadow,
  boxSizing: boxSizing,
  boxSuppress: boxSuppress,
  breakAfter: breakBeforeOrAfter,
  breakBefore: breakBeforeOrAfter,
  breakInside: breakInside,
  captionSide: captionSide,
  caretColor: color,
  clear: clear,
  clip: clip,
  clipPath: clipPath,
  clipRule: clipRule,
  color: color,
  colorAdjust: makeUnionRule('economy', 'exact') as RuleCheck,
  colorScheme: makeUnionRule('light', 'dark', 'light dark') as RuleCheck,
  columnCount: columnCount,
  columnFill: columnFill,
  columnGap: columnGap,
  columnRule: columnRule,
  columnRuleColor: columnRuleColor,
  columnRuleStyle: columnRuleStyle,
  columnRuleWidth: columnRuleWidth,
  columnSpan: columnSpan,
  columnWidth: columnWidth,
  columns: columns,
  contain: contain,
  containIntrinsicSize: makeUnionRule(isNumber, isString) as RuleCheck,
  containIntrinsicBlockSize: makeUnionRule(isNumber, isString) as RuleCheck,
  containIntrinsicInlineSize: makeUnionRule(isNumber, isString) as RuleCheck,
  containIntrinsicHeight: makeUnionRule(isNumber, isString) as RuleCheck,
  containIntrinsicWidth: makeUnionRule(isNumber, isString) as RuleCheck,
  container: isString,
  containerType: makeUnionRule('normal', 'size', 'inline-size') as RuleCheck,
  containerName: isString,
  content: content,
  contentVisibility: makeUnionRule('visible', 'hidden', 'auto') as RuleCheck,
  counterIncrement: counterIncrement,
  counterReset: counterReset,
  counterSet: isString,
  cue: cue,
  cueAfter: cueAfter,
  cueBefore: cueBefore,
  cursor: cursor,
  direction: direction,
  display: display,
  displayInside: displayInside,
  displayList: displayList,
  displayOutside: displayOutside,
  dominantBaseline: dominantBaseline,
  emptyCells: emptyCells,

  filter: filter,
  flex: isString,
  flexBasis: flexBasis,
  flexDirection: flexDirection,
  flexFlow: flexFlow,
  flexGrow: flexGrow,
  flexShrink: flexShrink,
  flexWrap: flexWrap,
  float: float,
  font: showError(
    '`font` is not recommended. Please use `fontSize`, `fontFamily`, `fontStyle` etc. instead',
  ) as RuleCheck,
  fontFamily: fontFamily,
  fontFeatureSettings: fontFeatureSettings,
  fontKerning: fontKerning,
  fontLanguageOverride: fontLanguageOverride,
  fontOpticalSizing: makeUnionRule('auto', 'none') as RuleCheck,
  fontPalette: isString,
  fontSize: fontSize,
  fontSizeAdjust: fontSizeAdjust,
  fontSmooth: makeUnionRule(
    'auto',
    'never',
    'always',
    lengthPercentage,
  ) as RuleCheck,
  fontStretch: fontStretch,
  fontStyle: fontStyle,
  fontSynthesis: fontSynthesis,
  fontVariant: fontVariant,
  fontVariantAlternates: fontVariantAlternates,
  fontVariantCaps: fontVariantCaps,
  fontVariantEastAsian: fontVariantEastAsian,
  fontVariantEmoji: makeUnionRule(
    'auto',
    'text',
    'emoji',
    'unicode',
  ) as RuleCheck,
  fontVariantLigatures: fontVariantLigatures,
  fontVariantNumeric: fontVariantNumeric,
  fontVariantPosition: fontVariantPosition,
  fontVariationSettings: isString,
  fontWeight: fontWeight,
  forcedColorAdjust: makeUnionRule('auto', 'none') as RuleCheck,

  fieldSizing: makeUnionRule('content', 'fixed') as RuleCheck,

  gap: gap,
  glyphOrientationHorizontal: glyphOrientationHorizontal,
  glyphOrientationVertical: glyphOrientationVertical,

  grid: grid,
  gridArea: gridArea,
  gridAutoColumns: gridAutoColumns,
  gridAutoFlow: gridAutoFlow,
  gridAutoRows: gridAutoRows,
  gridColumn: gridColumn,
  gridColumnEnd: gridColumnEnd,
  gridColumnGap: lengthPercentage,
  gridColumnStart: gridColumnStart,

  gridRow: gridRow,
  gridRowEnd: gridRowEnd,
  gridGap: lengthPercentage,
  gridRowGap: lengthPercentage,

  gridRowStart: gridRowStart,
  gridTemplate: gridTemplate,
  gridTemplateAreas: gridTemplateAreas,
  gridTemplateColumns: gridTemplate,
  gridTemplateRows: gridTemplate,

  hangingPunctuation: makeUnionRule(
    'none',
    'first',
    'last',
    'allow-end',
    'force-end',
    isString,
  ) as RuleCheck,
  hyphenateCharacter: isString,
  hyphens: hyphens,
  imageOrientation: imageOrientation,
  imageRendering: imageRendering,
  imageResolution: imageResolution,
  imeMode: imeMode,
  initialLetter: initialLetter,
  initialLetterAlign: initialLetterAlign,

  inset: length,
  top: length,
  right: length,
  bottom: length,
  left: length,
  insetBlock: length,
  insetBlockStart: length,
  insetBlockEnd: length,
  insetInline: length,
  insetInlineStart: length,
  insetInlineEnd: length,

  height: width,
  width: width,
  blockSize: blockSize,
  inlineSize: inlineSize,

  interpolateSize: interpolateSize,

  maxHeight: minMaxLength,
  maxWidth: minMaxLength,
  maxBlockSize: minMaxLength,
  maxInlineSize: minMaxLength,
  minBlockSize: minMaxLength,
  minHeight: minMaxLength,
  minInlineSize: minMaxLength,
  minWidth: minMaxLength,

  isolation: isolation,
  justifyContent: justifyContent,
  justifyItems: justifyItems,
  justifySelf: justifySelf,
  // Not supported in any browser yet.
  // justifyTracks: makeUnionRule(
  //   'start',
  //   'end',
  //   'center',
  //   'normal',
  //   'space-between',
  //   'space-around',
  //   'space-evenly',
  //   'stretch',
  //   'left',
  //   'right',
  // ),
  kerning: kerning,
  letterSpacing: letterSpacing,
  lineBreak: lineBreak,
  lineHeight: lineHeight,
  listStyle: listStyle,
  listStyleImage: listStyleImage,
  listStylePosition: listStylePosition,
  listStyleType: listStyleType,

  margin: margin,
  marginBlock: marginLeft,
  marginBlockEnd: marginLeft,
  marginBlockStart: marginLeft,
  marginBottom: marginLeft,
  marginInline: marginLeft,
  marginInlineEnd: marginLeft,
  marginInlineStart: marginLeft,
  marginLeft: marginLeft,
  marginRight: marginLeft,
  marginTop: marginTop,

  marker: marker,
  markerEnd: markerEnd,
  markerMid: markerMid,
  markerOffset: markerOffset,
  markerStart: markerStart,
  mask: mask,
  maskBorderMode: makeUnionRule('alpha', 'luminance') as RuleCheck,
  maskBorderOutset: isString,
  maskBorderRepeat: makeUnionRule(
    'stretch',
    'repeat',
    'round',
    'space',
  ) as RuleCheck,
  maskBorderSlice: isString,
  maskBorderSource: isString,
  maskBorderWidth: isString,
  maskClip: maskClip,
  maskComposite: maskComposite,
  maskImage: maskImage,
  maskMode: maskMode,
  maskOrigin: maskOrigin,
  maskPosition: maskPosition,
  maskRepeat: maskRepeat,
  maskSize: maskSize,
  maskType: maskType,

  mixBlendMode: mixBlendMode,
  motion: motion,
  motionOffset: lengthPercentage,
  motionPath: motionPath,
  motionRotation: motionRotation,
  objectFit: objectFit,
  objectPosition: objectPosition,

  offsetAnchor: isString,
  offsetPath: isString,
  offsetDistance: width,
  offsetBlockEnd: offsetBlockEnd,
  offsetBlockStart: offsetBlockStart,
  offsetInlineEnd: offsetInlineEnd,
  offsetInlineStart: offsetInlineStart,
  offsetRotate: isString,
  opacity: opacity,
  order: order,
  orphans: orphans,
  outline: outline,
  outlineColor: color,
  outlineOffset: makeUnionRule(isNumber, isLength) as RuleCheck,
  outlineStyle: makeUnionRule(
    'auto',
    'none',
    'dotted',
    'dashed',
    'solid',
    'double',
    'groove',
    'ridge',
    'inset',
    'outset',
  ) as RuleCheck,
  outlineWidth: makeUnionRule(
    isNumber,
    isLength,
    isNonNumericString,
  ) as RuleCheck,
  blockOverflow: overflow, // TODO - Add support to Babel Plugin
  inlineOverflow: overflow, // TODO - Add support to Babel Plugin
  overflow: overflow,
  overflowAnchor: overflowAnchor,
  overflowClipBox: overflowClipBox,
  overflowWrap: overflowWrap,
  overflowBlock: overflowDir,
  overflowX: overflowDir,
  overflowY: overflowDir,
  overscrollBehavior: overscrollBehavior,
  overscrollBehaviorInline: overscrollBehavior,
  overscrollBehaviorX: overscrollBehavior,
  overscrollBehaviorY: overscrollBehavior,
  overscrollBehaviorBlock: overscrollBehavior,
  overflowClipMargin: makeUnionRule(isNumber, isString) as RuleCheck,

  paintOrder: makeUnionRule(
    'normal',
    'fill',
    'stroke',
    'markers',
    isString,
  ) as RuleCheck,

  padding: length,
  paddingBlock: length,
  paddingBlockEnd: length,
  paddingBlockStart: length,
  paddingBottom: length,
  paddingInline: length,
  paddingInlineEnd: length,
  paddingInlineStart: length,
  paddingLeft: length,
  paddingRight: length,
  paddingTop: length,

  page: page,
  pageBreakAfter: pageBreak,
  pageBreakBefore: pageBreak,
  pageBreakInside: pageBreakInside,
  pause: pauseOrRest,
  pauseAfter: pauseOrRest,
  pauseBefore: pauseOrRest,
  perspective: perspective,
  perspectiveOrigin: perspectiveOrigin,
  pointerEvents: pointerEvents,
  position: position,

  // Shorthand not yet supported
  placeContent: isString,
  placeItems: isString,
  placeSelf: isString,
  printColorAdjust: makeUnionRule('economy', 'exact') as RuleCheck,

  quotes: quotes,
  resize: resize,
  rest: pauseOrRest,
  restAfter: pauseOrRest,
  restBefore: pauseOrRest,

  rotate: isString, // angle
  scale: makeUnionRule(isString, isNumber) as RuleCheck,
  translate: makeUnionRule(isString, isNumber) as RuleCheck,

  rowGap: rowGap,
  rubyAlign: rubyAlign,
  rubyMerge: rubyMerge,
  rubyPosition: rubyPosition,

  scrollbarColor: color,
  scrollbarGutter: makeUnionRule(
    'auto',
    'stable',
    'stable both-edges',
  ) as RuleCheck,
  scrollbarWidth: makeUnionRule('auto', 'thin', 'none') as RuleCheck,

  scrollBehavior: scrollBehavior,
  scrollSnapPaddingBottom: scrollSnapPaddingBottom,
  scrollSnapPaddingTop: scrollSnapPaddingTop,
  scrollSnapAlign: scrollSnapAlign,
  scrollSnapType: scrollSnapType,
  scrollSnapStop: makeUnionRule('normal', 'always') as RuleCheck,

  scrollMargin: scrollSpacing,
  scrollMarginBlock: scrollSpacing,
  scrollMarginBlockEnd: scrollSpacing,
  scrollMarginBlockStart: scrollSpacing,
  scrollMarginBottom: scrollSpacing,
  scrollMarginInline: scrollSpacing,
  scrollMarginInlineEnd: scrollSpacing,
  scrollMarginInlineStart: scrollSpacing,
  scrollMarginLeft: scrollSpacing,
  scrollMarginRight: scrollSpacing,
  scrollMarginTop: scrollSpacing,
  scrollPadding: scrollSpacing,
  scrollPaddingBlock: scrollSpacing,
  scrollPaddingBlockEnd: scrollSpacing,
  scrollPaddingBlockStart: scrollSpacing,
  scrollPaddingBottom: scrollSpacing,
  scrollPaddingInline: scrollSpacing,
  scrollPaddingInlineEnd: scrollSpacing,
  scrollPaddingInlineStart: scrollSpacing,
  scrollPaddingLeft: scrollSpacing,
  scrollPaddingRight: scrollSpacing,
  scrollPaddingTop: scrollSpacing,
  scrollSnapMarginBottom: scrollSpacing,
  scrollSnapMarginLeft: scrollSpacing,
  scrollSnapMarginRight: scrollSpacing,
  scrollSnapMarginTop: scrollSpacing,

  scrollTimeline: isString,
  scrollTimelineAxis: timelineAxis,
  scrollTimelineName: isString,

  shapeImageThreshold: shapeImageThreshold,
  shapeMargin: lengthPercentage,
  shapeOutside: shapeOutside,
  shapeRendering: shapeRendering,
  speak: speak,
  speakAs: speakAs,
  src: src,

  tabSize: tabSize,
  tableLayout: tableLayout,
  textAlign: textAlign,
  textAlignLast: textAlignLast,
  textAnchor: textAnchor,
  textCombineUpright: textCombineUpright,

  textDecoration: textDecoration,
  textDecorationColor: color,
  textDecorationLine: isString, // TODO: Stricter support in the future
  textDecorationSkip: makeUnionRule(
    'none',
    'objects',
    'spaces',
    'leading-spaces',
    'trailing-spaces',
    'edges',
    'box-decoration',
  ) as RuleCheck,
  textDecorationSkipInk: makeUnionRule('none', 'auto', 'all') as RuleCheck,
  textDecorationStyle: makeUnionRule(
    'solid',
    'double',
    'dotted',
    'dashed',
    'wavy',
  ) as RuleCheck,
  textDecorationThickness: makeUnionRule(isNumber, isLength) as RuleCheck,

  textEmphasis: textEmphasis,
  textEmphasisColor: textEmphasisColor,
  textEmphasisPosition: textEmphasisPosition,
  textEmphasisStyle: textEmphasisStyle,
  textIndent: textIndent,
  textJustify: textJustify,
  textOrientation: textOrientation,
  textOverflow: textOverflow,
  textRendering: textRendering,
  textShadow: textShadow,
  textSizeAdjust: textSizeAdjust,
  textTransform: textTransform,
  textUnderlineOffset: textUnderlineOffset,
  textUnderlinePosition: textUnderlinePosition,
  textWrap: makeUnionRule(
    'wrap',
    'nowrap',
    'balance',
    'pretty',
    'stable',
  ) as RuleCheck,

  timelineScope: isString,
  touchAction: touchAction,
  transform: transform,
  transformBox: transformBox,
  transformOrigin: transformOrigin,
  transformStyle: transformStyle,
  transition: isString,
  transitionDelay: time,
  transitionDuration: time,
  transitionProperty: transitionProperty,
  transitionTimingFunction: transitionTimingFunction,
  unicodeBidi: unicodeBidi,
  unicodeRange: unicodeRange,
  userSelect: userSelect,
  viewTimeline: isString,
  viewTimelineAxis: timelineAxis,
  viewTimelineName: isString,
  viewTimelineInset: scrollSpacing,
  viewTransitionName: makeUnionRule(all, isString) as RuleCheck,
  verticalAlign: verticalAlign,
  visibility: visibility,
  voiceBalance: voiceBalance,
  voiceDuration: voiceDuration,
  voiceFamily: voiceFamily,
  voicePitch: voicePitch,
  voiceRange: voiceRange,
  voiceRate: voiceRate,
  voiceStress: voiceStress,
  voiceVolume: voiceVolume,
  whiteSpace: whiteSpace,
  widows: widows,
  willChange: willChange,
  wordBreak: wordBreak,
  wordSpacing: wordSpacing,
  wordWrap: wordWrap,
  writingMode: writingMode,
  zIndex: zIndex,

  // Purposely not supported because it is not supported in Firefox.
  zoom: makeUnionRule('normal', 'reset', isNumber, isPercentage) as RuleCheck,
};
export const CSSPropertyKeys: Array<string> = Object.keys(CSSProperties);
for (const key of CSSPropertyKeys) {
  CSSProperties[key] = makeUnionRule(CSSProperties[key], all);
}

export const CSSPropertyReplacements: { [key: string]: RuleCheck | void } = {
  border: border(),
  borderTop: border('Top'),
  borderBlockStart: border('Top'),
  borderEnd: border('InlineEnd'),
  borderInlineEnd: border('InlineEnd'),
  borderRight: border('Right'),
  borderBottom: border('Bottom'),
  borderBlockEnd: border('Bottom'),
  borderStart: border('InlineStart'),
  borderInlineStart: border('InlineStart'),
  borderLeft: border('Left'),
};

export const pseudoElements: RuleCheck = makeUnionRule(
  makeLiteralRule('::before'),
  makeLiteralRule('::after'),
  makeLiteralRule('::first-letter'),
  makeLiteralRule('::first-line'),
  makeLiteralRule('::selection'),
  makeLiteralRule('::backdrop'),
  makeLiteralRule('::marker'),
  makeLiteralRule('::placeholder'),
  makeLiteralRule('::spelling-error'),
  makeLiteralRule('::grammar-error'),
  makeLiteralRule('::cue'),
  makeLiteralRule('::cue-region'),
  makeLiteralRule('::file-selector-button'),
  makeLiteralRule('::target-text'),
  makeLiteralRule('::slotted'),
  makeLiteralRule('::part'),
  makeLiteralRule('::thumb'),
  // For styling input[type=number]
  makeLiteralRule('::-webkit-inner-spin-button'),
  makeLiteralRule('::-webkit-outer-spin-button'),
  // For styling input[type=search]
  makeLiteralRule('::-webkit-search-decoration'),
  makeLiteralRule('::-webkit-search-cancel-button'),
  makeLiteralRule('::-webkit-search-results-button'),
  makeLiteralRule('::-webkit-search-results-decoration'),
  // For Scrollbars in Webkit and Chromium
  makeLiteralRule('::-webkit-scrollbar'),
  makeLiteralRule('::-webkit-scrollbar-button'),
  makeLiteralRule('::-webkit-scrollbar-thumb'),
  makeLiteralRule('::-webkit-scrollbar-track'),
  makeLiteralRule('::-webkit-scrollbar-track-piece'),
  makeLiteralRule('::-webkit-scrollbar-corner'),
  makeLiteralRule('::-webkit-resizer'),
  // For input ranges in Chromium
  makeLiteralRule('::-webkit-slider-thumb'),
  makeLiteralRule('::-webkit-slider-runnable-track'),
  // For input ranges in Firefox
  makeLiteralRule('::-moz-range-thumb'),
  makeLiteralRule('::-moz-range-track'),
  makeLiteralRule('::-moz-range-progress'),
  // For progress elements
  makeLiteralRule('::-webkit-progress-bar'),
  makeLiteralRule('::-webkit-progress-value'),
  makeLiteralRule('::-moz-progress-bar'),
);

export const pseudoClassesAndAtRules: RuleCheck = makeUnionRule(
  makeLiteralRule(':first-child'),
  makeLiteralRule(':last-child'),
  makeLiteralRule(':only-child'),
  makeLiteralRule(':nth-child'),
  makeLiteralRule(':nth-of-type'),
  makeLiteralRule(':empty'),
  makeLiteralRule(':hover'),
  makeLiteralRule(':focus'),
  makeLiteralRule(':focus-visible'),
  makeLiteralRule(':focus-within'),
  makeLiteralRule(':active'),
  makeLiteralRule(':visited'),
  makeLiteralRule(':disabled'),
  makeRegExRule(/^@media/, 'a media query'),
  makeRegExRule(/^@container/, 'a media query'),
  makeRegExRule(/^@supports/, 'a supports query'),
  makeLiteralRule('@starting-style'),
);

export const allModifiers: RuleCheck = makeUnionRule(
  pseudoElements,
  pseudoClassesAndAtRules,
);

export { CSSProperties };
