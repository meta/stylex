/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import getDistance from './utils/getDistance';
import isWhiteSpaceOrEmpty from './utils/isWhiteSpaceOrEmpty';
import type {
  CallExpression,
  Directive,
  Expression,
  Identifier,
  ImportDeclaration,
  ModuleDeclaration,
  Node,
  ObjectExpression,
  Pattern,
  Program,
  Property,
  Literal,
  Statement,
  VariableDeclaration,
  VariableDeclarator,
  PrivateIdentifier,
} from 'estree';
import micromatch from 'micromatch';
/*:: import { Rule } from 'eslint'; */
import makeLiteralRule from './rules/makeLiteralRule';
import isString from './rules/isString';
import makeUnionRule from './rules/makeUnionRule';
import isNumber from './rules/isNumber';
import isAnimationName from './rules/isAnimationName';
import isPositionTryFallbacks from './rules/isPositionTryFallbacks';
import isStylexResolvedVarsToken from './rules/isStylexResolvedVarsToken';
import isStylexEnvMember from './rules/isStylexEnvMember';
import isCSSVariable from './rules/isCSSVariable';
import evaluate from './utils/evaluate';
import resolveKey from './utils/resolveKey';
import formatPropertiesWithNodeIndentation from './utils/formatPropertiesWithNodeIndentation';
import getSourceCode from './utils/getSourceCode';
import {
  CSSPropertyKeys,
  CSSProperties,
  CSSPropertyReplacements,
  pseudoElements,
  pseudoClassesAndAtRules,
  allModifiers,
  all,
} from './reference/cssProperties';
import { splitSpecificShorthands, CANNOT_FIX } from './utils/splitShorthands';

export type Variables = $ReadOnlyMap<string, Expression | 'ARG'>;
export type RuleCheck = (
  node: $ReadOnly<Expression | Pattern>,
  variables?: Variables,
  prop?: $ReadOnly<Property>,
  context?: Rule.RuleContext,
) => RuleResponse;
export type RuleResponse = void | {
  message: string,
  distance?: number,
  fix?: Rule.ReportFixer,
  suggest?: {
    fix: Rule.ReportFixer,
    desc: string,
  },
};
type ValidationResult =
  | RuleResponse
  | {
      ...Rule.ReportDescriptor,
      isSpecialCase: true,
    };

const showError =
  (message: string): RuleCheck =>
  () => ({ message });

// Maps camelCase CSS shorthand property names to the hyphenated names
// used by splitSpecificShorthands
const shorthandExpansionMap: { [string]: string } = {
  animation: 'animation',
  font: 'font',
  gap: 'gap',
  gridArea: 'grid-area',
  gridColumn: 'grid-column',
  gridRow: 'grid-row',
  gridTemplate: 'grid-template',
  gridGap: 'gap',
};

const LEGACY_CONDITIONAL_SHORTHAND_FIXERS = new Set(['animation', 'font']);

const LEGACY_CONDITIONAL_REPLACEMENT_FIXERS = new Set([
  'border',
  'borderTop',
  'borderBlockStart',
  'borderEnd',
  'borderInlineEnd',
  'borderRight',
  'borderBottom',
  'borderBlockEnd',
  'borderStart',
  'borderInlineStart',
  'borderLeft',
]);

const NUMERIC_LITERAL_VALUE_REGEX = /^[-+]?(?:\d+|\d*\.\d+)$/;

const NUMERIC_LITERAL_PROPERTIES = new Set([
  'rowGap',
  'columnGap',
  'lineHeight',
  'fontWeight',
  'animationIterationCount',
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

const formatExpandedProperties = (
  prop: $ReadOnly<Property>,
  expanded: $ReadOnlyArray<$ReadOnly<[string, number | string]>>,
  context?: Rule.RuleContext,
): string => {
  const sourceCode = context != null ? getSourceCode(context) : undefined;
  const properties = expanded.map(
    ([key, value]) => `${key}: ${serializeValue(key, value)}`,
  );
  return formatPropertiesWithNodeIndentation(prop, properties, sourceCode);
};

const showErrorWithFix =
  (message: string, propertyKey: string): RuleCheck =>
  (
    node: $ReadOnly<Expression | Pattern>,
    _variables?: Variables,
    prop?: $ReadOnly<Property>,
    context?: Rule.RuleContext,
  ): RuleResponse => {
    const response: $NonMaybeType<RuleResponse> = { message };
    const shorthandProp = shorthandExpansionMap[propertyKey];
    if (shorthandProp == null || node.type !== 'Literal' || prop == null) {
      return response;
    }
    const val = node.value;
    if (typeof val !== 'string' && typeof val !== 'number') {
      return response;
    }
    const expanded = splitSpecificShorthands(shorthandProp, String(val));
    if (expanded.length <= 1 && expanded[0]?.[1] !== CANNOT_FIX) {
      // Single value that's unchanged — no expansion available
      return response;
    }
    if (expanded.length === 1 && expanded[0]?.[1] === CANNOT_FIX) {
      // Cannot be auto-fixed
      return response;
    }
    const newPropertiesText = formatExpandedProperties(prop, expanded, context);
    const fixFn = (fixer: Rule.RuleFixer) => {
      return fixer.replaceText(prop, newPropertiesText);
    };
    // animation is suggest-only since animationName needs a keyframes() reference
    if (propertyKey !== 'animation') {
      response.fix = fixFn;
    }
    response.suggest = {
      desc: `Split '${propertyKey}' shorthand into individual longhand properties?`,
      fix: fixFn,
    };
    return response;
  };

const stylexValidStyles = {
  meta: {
    type: 'problem',
    hasSuggestions: true,
    fixable: 'code',
    docs: {
      descriptions: 'Enforce that you create valid stylex styles',
      category: 'Possible Errors',
      recommended: true,
    },
    schema: [
      {
        type: 'object',
        properties: {
          validImports: {
            type: 'array',
            items: {
              oneOf: [
                { type: 'string' },
                {
                  type: 'object',
                  properties: {
                    from: { type: 'string' },
                    as: { type: 'string' },
                  },
                },
              ],
            },
            default: ['stylex', '@stylexjs/stylex'],
          },
          allowRawCSSVars: {
            type: 'boolean',
            default: true,
          },
          allowOuterPseudoAndMedia: {
            type: 'boolean',
            default: false,
          },
          banPropsForLegacy: {
            type: 'boolean',
            default: false,
          },
          styleResolution: {
            type: 'string',
            enum: [
              'application-order',
              'property-specificity',
              'legacy-expand-shorthands',
            ],
          },
          propLimits: {
            type: 'object',
            additionalProperties: {
              type: 'object',
              properties: {
                limit: {
                  oneOf: [
                    { type: 'null' },
                    { type: 'string' },
                    { type: 'number' },
                    {
                      type: 'array',
                      items: {
                        oneOf: [
                          { type: 'null' },
                          { type: 'string' },
                          { type: 'number' },
                        ],
                      },
                    },
                  ],
                },
                reason: { type: 'string' },
              },
            },
          },
        },
      },
    ],
  },
  create(context: Rule.RuleContext): { ... } {
    const variables = new Map<string, Expression | 'ARG'>();
    const dynamicStyleVariables = new Set<string>();
    const options = context.options[0] || {};
    const themeFileExtension = options.themeFileExtension || '.stylex';

    const legacyReason =
      'This property is not supported in legacy StyleX resolution.';

    type PropLimits = {
      [string]: {
        limit: null | string | number | Array<string | number>,
        reason: string,
      },
    };

    type Schema = {
      validImports: Array<
        | string
        | {
            from: string,
            as: string,
          },
      >,
      allowRawCSSVars: boolean,
      allowOuterPseudoAndMedia: boolean,
      banPropsForLegacy: boolean,
      styleResolution?:
        | 'application-order'
        | 'property-specificity'
        | 'legacy-expand-shorthands',
      propLimits?: PropLimits,
    };

    const legacyProps: PropLimits = {
      grid: { limit: null, reason: legacyReason },
      gridArea: { limit: null, reason: legacyReason },
      gridColumn: { limit: null, reason: legacyReason },
      gridRow: { limit: null, reason: legacyReason },
      gridTemplate: { limit: null, reason: legacyReason },
      gridGap: { limit: null, reason: legacyReason },
      gridColumnGap: { limit: null, reason: legacyReason },
      gridRowGap: { limit: null, reason: legacyReason },
      'mask+([a-zA-Z])': { limit: null, reason: legacyReason },
      blockOverflow: { limit: null, reason: legacyReason },
      inlineOverflow: { limit: null, reason: legacyReason },
      transitionProperty: {
        limit: ['opacity', 'transform', 'opacity, transform', 'none'],
        reason: legacyReason,
      },
    };

    const {
      validImports: importsToLookFor = ['stylex', '@stylexjs/stylex'],
      allowRawCSSVars = true,
      allowOuterPseudoAndMedia,
      banPropsForLegacy = false,
      styleResolution,
      propLimits = {},
    }: Schema = context.options[0] || {};

    const isLegacyExpandShorthands =
      styleResolution === 'legacy-expand-shorthands' || banPropsForLegacy;

    const shouldEnableLegacyConditionalShorthandFixer = (
      propertyKey: string,
    ): boolean =>
      isLegacyExpandShorthands ||
      !LEGACY_CONDITIONAL_SHORTHAND_FIXERS.has(propertyKey);

    /**
     * Check if a file has a valid extension for StyleX variable imports.
     *
     * `.stylex`: used when importing `defineVars` or `defineConsts` variables. This prevents
     *   the linter/compiler from marking imports as unresolved and allows computed
     *   keys in those cases.
     *
     *  `.stylex.const`: used when importing `defineConsts` constants. This prevents
     *   the linter/compiler from marking imports as unresolved and allows computed
     *   keys in those cases.
     *
     * `.transformed`: used for files that have already been processed by a custom
     *   transform that pre-resolve StyleX variables to silence ESLint/compiler errors.
     *
     */
    function isValidStylexResolvedVarsFileExtension(
      filename: string,
      themeFileExtension: string,
    ) {
      const baseExtensions = [
        themeFileExtension,
        `${themeFileExtension}.const`,
        '.transformed',
      ];
      const extensions = ['.js', '.ts', '.tsx', '.jsx', '.mjs', '.cjs'];
      return ['', ...extensions].some((ext) =>
        baseExtensions.some((base) => filename.endsWith(`${base}${ext}`)),
      );
    }
    const stylexResolvedVarsTokenImports = new Set<string>();

    // Track same-file defineVars/defineVarsNested/defineConstsNested declarations.
    const currentFilename = context.filename;
    const isStylexFile = isValidStylexResolvedVarsFileExtension(
      currentFilename,
      themeFileExtension,
    );

    const styleXDefaultImports = new Set<string>();
    const styleXCreateImports = new Set<string>();
    const styleXKeyframesImports = new Set<string>();
    const styleXPositionTryImports = new Set<string>();
    const styleXFirstThatWorksImports = new Set<string>();
    const styleXWhenImports = new Set<string>();
    const styleXEnvImports = new Set<string>();

    const overrides: PropLimits = {
      ...(isLegacyExpandShorthands ? legacyProps : {}),
      ...propLimits,
    };

    const CSSPropertiesWithOverrides: { [string]: RuleCheck } = {
      ...CSSProperties,
      // TODO change this to a special function that looks for stylex.keyframes call
      animationName: makeUnionRule(
        makeLiteralRule('none'),
        isAnimationName(styleXDefaultImports, styleXKeyframesImports),
        all,
      ),
      positionTryFallbacks: makeUnionRule(
        makeLiteralRule('none'),
        isCSSVariable,
        isPositionTryFallbacks(styleXDefaultImports, styleXPositionTryImports),
        all,
      ),
    };
    const getOverrideErrorRule = (reason: string, propertyKey: string) =>
      shorthandExpansionMap[propertyKey] != null &&
      shouldEnableLegacyConditionalShorthandFixer(propertyKey)
        ? showErrorWithFix(reason, propertyKey)
        : showError(reason);

    for (const overrideKey in overrides) {
      const { limit, reason } = overrides[overrideKey];
      if (limit === null) {
        // For properties with known shorthand expansions, provide auto-fixers
        if (overrideKey.includes('*') || overrideKey.includes('+')) {
          for (const key in CSSPropertiesWithOverrides) {
            if (micromatch.isMatch(key, overrideKey)) {
              CSSPropertiesWithOverrides[key] = getOverrideErrorRule(
                reason,
                key,
              );
            }
          }
        } else {
          CSSPropertiesWithOverrides[overrideKey] = getOverrideErrorRule(
            reason,
            overrideKey,
          );
        }
        continue;
      }
      const overrideValue =
        limit === '*'
          ? makeUnionRule(isString, isNumber, all)
          : limit === 'string'
            ? makeUnionRule(isString, all)
            : limit === 'number'
              ? makeUnionRule(isNumber, all)
              : typeof limit === 'string' || typeof limit === 'number'
                ? makeUnionRule(limit, all)
                : Array.isArray(limit)
                  ? makeUnionRule(
                      ...limit.map((l) => {
                        if (l === '*') {
                          return makeUnionRule(isString, isNumber);
                        }
                        if (l === 'string') {
                          return isString;
                        }
                        if (l === 'number') {
                          return isNumber;
                        }
                        return l;
                      }),
                      all,
                    )
                  : undefined;
      if (overrideValue === undefined) {
        // skip
        continue;
      }
      if (overrideKey.includes('*') || overrideKey.includes('+')) {
        for (const key in CSSPropertiesWithOverrides) {
          if (micromatch.isMatch(key, overrideKey)) {
            CSSPropertiesWithOverrides[key] = overrideValue;
          }
        }
      } else {
        CSSPropertiesWithOverrides[overrideKey] = overrideValue;
      }
    }

    function isStylexCallee(node: Node) {
      return (
        (node.type === 'MemberExpression' &&
          node.object.type === 'Identifier' &&
          styleXDefaultImports.has(node.object.name) &&
          node.property.type === 'Identifier' &&
          node.property.name === 'create') ||
        (node.type === 'Identifier' && styleXCreateImports.has(node.name))
      );
    }

    function isStylexDeclaration(node: $ReadOnly<{ ...Node, ... }>) {
      return (
        node &&
        node.type === 'CallExpression' &&
        isStylexCallee(node.callee) &&
        node.arguments.length === 1
      );
    }

    function validateStyleValue(
      valueNode: Expression | Pattern,
      varsWithFnArgs: Variables,
      style: Property,
      styleKey: Expression | PrivateIdentifier,
      propertyKey: string,
      ruleChecker: RuleCheck,
    ): ValidationResult | null {
      // For: condition ? <style-value> : <style-value>
      if (valueNode.type === 'ConditionalExpression') {
        const trueCheck = validateStyleValue(
          valueNode.consequent,
          varsWithFnArgs,
          style,
          styleKey,
          propertyKey,
          ruleChecker,
        );
        if (trueCheck != null) {
          return trueCheck;
        }
        const falseCheck = validateStyleValue(
          valueNode.alternate,
          varsWithFnArgs,
          style,
          styleKey,
          propertyKey,
          ruleChecker,
        );
        if (falseCheck != null) {
          return falseCheck;
        }
        return null;
      }

      // For: color: "blue" || "green" or zIndex: var ?? 10
      if (
        valueNode.type === 'LogicalExpression' &&
        ['||', '??'].includes(valueNode.operator)
      ) {
        const leftCheck = validateStyleValue(
          valueNode.left,
          varsWithFnArgs,
          style,
          styleKey,
          propertyKey,
          ruleChecker,
        );
        if (leftCheck != null) {
          return leftCheck;
        }
        const rightCheck = validateStyleValue(
          valueNode.right,
          varsWithFnArgs,
          style,
          styleKey,
          propertyKey,
          ruleChecker,
        );
        if (rightCheck != null) {
          return rightCheck;
        }
        return null;
      }

      if (valueNode.type === 'CallExpression') {
        const callee = valueNode.callee;
        if (
          (callee.type === 'MemberExpression' &&
            callee.object.type === 'Identifier' &&
            styleXDefaultImports.has(callee.object.name) &&
            callee.property.type === 'Identifier' &&
            callee.property.name === 'firstThatWorks') ||
          (callee.type === 'Identifier' &&
            styleXFirstThatWorksImports.has(callee.name))
        ) {
          for (const arg of valueNode.arguments) {
            if (arg.type === 'SpreadElement') {
              continue;
            }
            const argCheck = validateStyleValue(
              arg,
              varsWithFnArgs,
              style,
              styleKey,
              propertyKey,
              ruleChecker,
            );
            if (argCheck != null) {
              return argCheck;
            }
          }
          return null;
        }
      }

      if (
        (propertyKey === 'float' || propertyKey === 'clear') &&
        valueNode.type === 'Literal' &&
        typeof valueNode.value === 'string' &&
        (valueNode.value === 'start' || valueNode.value === 'end')
      ) {
        const replacement =
          valueNode.value === 'start' ? 'inline-start' : 'inline-end';
        return {
          node: valueNode,
          loc: valueNode.loc,
          message: `The value "${valueNode.value}" is not a standard CSS value for "${propertyKey}". Did you mean "${replacement}"?`,
          fix: (fixer) => fixer.replaceText(valueNode, `'${replacement}'`),
          suggest: [
            {
              desc: `Replace "${valueNode.value}" with "${replacement}"?`,
              fix: (fixer) => fixer.replaceText(valueNode, `'${replacement}'`),
            },
          ],
          isSpecialCase: true,
        };
      }

      const check = ruleChecker(valueNode, varsWithFnArgs, style, context);
      if (check != null) {
        return check;
      }
      if (
        valueNode.type === 'Literal' &&
        typeof valueNode.value === 'string' &&
        isWhiteSpaceOrEmpty(valueNode.value) &&
        styleKey.name !== 'content'
      ) {
        return {
          node: valueNode,
          loc: valueNode.loc,
          message:
            'The empty string is not allowed by Stylex. Use `null` to reset a style.',
          suggest: [
            {
              desc: 'Replace empty string with `null`?',
              fix: (fixer) => fixer.replaceText(valueNode, 'null'),
            },
          ],
          isSpecialCase: true,
        };
      }
    }
    function checkStyleProperty(
      style: Node,
      level: number,
      propName: null | string,
      outerIsPseudoElement: boolean,
    ): void {
      // currently ignoring preset spreads.
      if (style.type === 'Property') {
        // const styleAsProperty: Property = style;
        if (style.value.type === 'ObjectExpression') {
          const styleValue: ObjectExpression = style.value;
          // TODO: Remove this soon
          // But we want to make sure that the same "condition" isn't repeated
          if (
            level > 0 &&
            propName == null &&
            // Allow exactly one inner level when the outer/top nested layer is a pseudo-element
            !(outerIsPseudoElement && level === 1)
          ) {
            return context.report({
              node: style.value as Node,
              loc: style.value.loc,
              message: 'You cannot nest styles more than one level deep',
            } as Rule.ReportDescriptor);
          }
          const key = style.key;
          if (key.type === 'PrivateIdentifier') {
            return context.report({
              node: key,
              loc: key.loc,
              message: 'Private properties are not allowed in stylex',
            } as Rule.ReportDescriptor);
          }
          const keyName =
            key.type === 'Literal'
              ? key.value
              : key.type === 'Identifier'
                ? !style.computed
                  ? key.name
                  : resolveKey(key, variables)
                : null;
          if (isStylexResolvedVarsToken(key, stylexResolvedVarsTokenImports)) {
            return undefined;
          }
          if (isStylexEnvMember(key, styleXDefaultImports, styleXEnvImports)) {
            return styleValue.properties.forEach((prop) =>
              checkStyleProperty(
                prop,
                level + 1,
                propName,
                outerIsPseudoElement,
              ),
            );
          }
          if (
            typeof keyName !== 'string' ||
            (key.type !== 'Literal' && key.type !== 'Identifier')
          ) {
            return context.report({
              node: key,
              loc: key.loc,
              message: 'Keys must be strings',
            } as Rule.ReportDescriptor);
          }
          if (keyName.startsWith('@') || keyName.startsWith(':')) {
            if (level === 0) {
              const ruleCheck = (
                allowOuterPseudoAndMedia ? allModifiers : pseudoElements
              )(key, variables);

              if (ruleCheck !== undefined) {
                if (keyName.startsWith('::')) {
                  return context.report({
                    node: style.value,
                    loc: style.value.loc,
                    message: `Unknown pseudo element "${keyName}"`,
                  } as $ReadOnly<Rule.ReportDescriptor>);
                }
                return context.report({
                  node: style.value,
                  loc: style.value.loc,
                  message: allowOuterPseudoAndMedia
                    ? 'Nested styles can only be used for the pseudo selectors in the stylex allowlist and for @media queries'
                    : 'Pseudo Classes, Media Queries and other At Rules should be nested as conditions within style properties. Only Pseudo Elements (::after) are allowed at the top-level',
                } as $ReadOnly<Rule.ReportDescriptor>);
              }
            } else {
              const ruleCheck = pseudoClassesAndAtRules(key, variables);

              if (ruleCheck !== undefined) {
                return context.report({
                  node: style.value,
                  loc: style.value.loc,
                  message:
                    'Invalid Pseudo class or At Rule used for conditional style value',
                } as $ReadOnly<Rule.ReportDescriptor>);
              }
            }
          }

          return styleValue.properties.forEach((prop) =>
            checkStyleProperty(
              prop,
              level + 1,
              propName ??
                (keyName.startsWith('@') ||
                keyName.startsWith(':') ||
                keyName === 'default'
                  ? null
                  : keyName),
              outerIsPseudoElement || keyName.startsWith('::'),
            ),
          );
        }
        let styleKey: Expression | PrivateIdentifier = style.key;
        if (styleKey.type === 'PrivateIdentifier') {
          return context.report({
            node: styleKey,
            loc: styleKey.loc,
            message: 'Private properties are not allowed in stylex',
          } as Rule.ReportDescriptor);
        }
        if (
          isStylexResolvedVarsToken(styleKey, stylexResolvedVarsTokenImports)
        ) {
          return undefined;
        }

        const isStylexEnvKey = isStylexEnvMember(
          styleKey,
          styleXDefaultImports,
          styleXEnvImports,
        );
        let isStylexWhenCall = false;
        if (style.computed && styleKey.type !== 'Literal') {
          if (
            styleKey.type === 'CallExpression' &&
            styleKey.callee.type === 'MemberExpression'
          ) {
            const calleeObject = styleKey.callee.object;
            const calleeProperty = styleKey.callee.property;

            isStylexWhenCall =
              (calleeObject.type === 'MemberExpression' &&
                calleeObject.object.type === 'Identifier' &&
                styleXDefaultImports.has(calleeObject.object.name) &&
                calleeObject.property.type === 'Identifier' &&
                calleeObject.property.name === 'when' &&
                calleeProperty.type === 'Identifier') ||
              (calleeObject.type === 'Identifier' &&
                styleXWhenImports.has(calleeObject.name) &&
                calleeProperty.type === 'Identifier');

            if (!isStylexWhenCall) {
              const val = evaluate(styleKey, variables);
              if (val == null) {
                return context.report({
                  node: style.key,
                  loc: style.key.loc,
                  message: 'Computed key cannot be resolved.',
                } as Rule.ReportDescriptor);
              } else if (val === 'ARG') {
                return context.report({
                  node: style.key,
                  loc: style.key.loc,
                  message: 'Computed key cannot depend on function argument',
                } as Rule.ReportDescriptor);
              } else {
                styleKey = val;
              }
            }
          }
        }
        if (
          styleKey.type !== 'Literal' &&
          styleKey.type !== 'Identifier' &&
          !isStylexWhenCall &&
          !isStylexEnvKey
        ) {
          return context.report({
            node: styleKey,
            loc: styleKey.loc,
            message:
              'All keys in a stylex object must be static literal values.',
          } as Rule.ReportDescriptor);
        }

        const key =
          propName ??
          (styleKey.type === 'Identifier' ? styleKey.name : styleKey.value);
        if (typeof key !== 'string') {
          return context.report({
            node: styleKey,
            loc: styleKey.loc,
            message:
              'All keys in a stylex object must be static literal string values.',
          } as Rule.ReportDescriptor);
        }
        if (CSSPropertyReplacements[key] != null) {
          const propCheck: RuleCheck = CSSPropertyReplacements[key];
          // eslint-disable-next-line no-unused-vars
          const val: Property = style;
          const check = propCheck(style.value, variables, style, context);
          if (check != null) {
            const { message } = check;
            const { suggest } = check;
            let { fix } = check;
            if (
              !isLegacyExpandShorthands &&
              LEGACY_CONDITIONAL_REPLACEMENT_FIXERS.has(key)
            ) {
              fix = undefined;
            }
            const diagnostic: Rule.ReportDescriptor = {
              node: style,
              loc: style.loc,
              message,
              fix: fix != null ? fix : undefined,
              suggest: suggest != null ? [suggest] : undefined,
            };
            return context.report(diagnostic);
          }
        }
        const ruleChecker = CSSPropertiesWithOverrides[key];
        if (ruleChecker == null) {
          if (allowRawCSSVars && micromatch.isMatch(key, '--*')) {
            return;
          }

          const closestKey = CSSPropertyKeys.find((cssProp) => {
            const distance = getDistance(key, cssProp, 2);
            return distance <= 2;
          });

          return context.report({
            node: style.key,
            loc: style.key.loc,
            message: 'This is not a key that is allowed by stylex',
            suggest:
              closestKey != null
                ? [
                    {
                      desc: `Did you mean "${closestKey}"?`,
                      fix: (fixer) => {
                        if (style.key.type === 'Identifier') {
                          return fixer.replaceText(style.key, closestKey);
                        } else if (
                          style.key.type === 'Literal' &&
                          (typeof style.key.value === 'string' ||
                            typeof style.key.value === 'number' ||
                            typeof style.key.value === 'boolean' ||
                            style.key.value == null)
                        ) {
                          const styleKey: Literal = style.key;
                          const raw = style.key.raw;
                          if (raw != null) {
                            const quoteType = raw.substr(0, 1);
                            return fixer.replaceText(
                              styleKey,
                              `${quoteType}${closestKey}${quoteType}`,
                            );
                          }
                        }
                        return null;
                      },
                    },
                  ]
                : undefined,
          } as Rule.ReportDescriptor);
        }
        if (typeof ruleChecker !== 'function') {
          throw new TypeError(`CSSProperties[${key}] is not a function`);
        }

        const isReferencingStylexDefineVarsTokens =
          stylexResolvedVarsTokenImports.size > 0 &&
          isStylexResolvedVarsToken(
            style.value,
            stylexResolvedVarsTokenImports,
          );
        if (!isReferencingStylexDefineVarsTokens) {
          let varsWithFnArgs: Map<string, Expression | 'ARG'> = variables;
          if (dynamicStyleVariables.size > 0) {
            varsWithFnArgs = new Map();
            for (const [key, value] of variables) {
              varsWithFnArgs.set(key, value);
            }
            for (const key of dynamicStyleVariables) {
              varsWithFnArgs.set(key, 'ARG');
            }
          }

          const check = validateStyleValue(
            style.value,
            varsWithFnArgs,
            style,
            styleKey,
            key,
            ruleChecker,
          );
          if (check != null) {
            if (check.isSpecialCase) {
              return context.report({
                node: check.node,
                loc: check.loc,
                message: check.message,
                fix: check.fix,
                suggest: check.suggest,
              });
            }

            const { message } = check;
            let { fix, suggest } = check;

            // If the property has a known shorthand expansion and no fix yet,
            // try to attach one
            if (
              fix == null &&
              suggest == null &&
              shorthandExpansionMap[key] != null &&
              shouldEnableLegacyConditionalShorthandFixer(key) &&
              style.value.type === 'Literal'
            ) {
              const val = style.value.value;
              if (typeof val === 'string' || typeof val === 'number') {
                const shorthandProp = shorthandExpansionMap[key];
                const expanded = splitSpecificShorthands(
                  shorthandProp,
                  String(val),
                );
                const canFix =
                  expanded.length > 1 ||
                  (expanded.length === 1 && expanded[0]?.[1] === CANNOT_FIX);
                const isFixable =
                  canFix &&
                  !(expanded.length === 1 && expanded[0]?.[1] === CANNOT_FIX);
                if (isFixable) {
                  const newPropertiesText = formatExpandedProperties(
                    style,
                    expanded,
                    context,
                  );
                  const fixFn = (fixer: Rule.RuleFixer) =>
                    fixer.replaceText(style, newPropertiesText);
                  // animation is suggest-only since animationName needs keyframes()
                  if (key !== 'animation') {
                    fix = fixFn;
                  }
                  suggest = {
                    desc: `Split '${key}' shorthand into individual longhand properties?`,
                    fix: fixFn,
                  };
                }
              }
            }

            const isBackgroundBlendModeFormatError =
              key === 'backgroundBlendMode' &&
              typeof message === 'string' &&
              message.indexOf(
                'backgroundBlendMode values must be separated by a comma and a space',
              ) !== -1;

            const finalMessage = isBackgroundBlendModeFormatError
              ? message.split('\n')[0]
              : `${key} value must be one of:\n${message}${
                  key === 'lineHeight'
                    ? '\nBe careful when fixing: lineHeight: 10px is not the same as lineHeight: 10'
                    : ''
                }`;

            return context.report({
              node: style.value,
              loc: style.value.loc,
              message: finalMessage,
              fix: fix != null ? fix : undefined,
              suggest: suggest != null ? [suggest] : undefined,
            } as Rule.ReportDescriptor);
          }
        }
      }
    }

    return {
      Program(node: Program) {
        // Keep track of all the top-level local variable declarations
        // This is because stylex allows you to use local constants in your styles

        // const body = node.body;
        // for (let statement of body) {

        // }

        const vars = node.body
          .reduce(
            (
              collection: Array<VariableDeclaration>,
              node: Statement | ModuleDeclaration | Directive,
            ) => {
              if (node.type === 'VariableDeclaration') {
                collection.push(node);
              }
              return collection;
            },
            [],
          )
          .map(
            (
              constDecl: VariableDeclaration,
            ): $ReadOnlyArray<VariableDeclarator> => constDecl.declarations,
          )
          .reduce(
            (
              arr: $ReadOnlyArray<VariableDeclarator>,
              curr: $ReadOnlyArray<VariableDeclarator>,
            ) => arr.concat(curr),
            [],
          );

        const [requires, others] = vars.reduce(
          (acc, decl) => {
            if (
              decl.init != null &&
              decl.init.type === 'CallExpression' &&
              decl.init.callee.type === 'Identifier' &&
              decl.init.callee.name === 'require'
            ) {
              acc[0].push(decl);
            } else {
              acc[1].push(decl);
            }
            return acc;
          },
          [[] as Array<VariableDeclarator>, [] as Array<VariableDeclarator>],
        );

        requires.forEach((decl: VariableDeclarator) => {
          // detect requires of "stylex" and "@stylexjs/stylex"
          if (
            decl.init != null &&
            decl.init.type === 'CallExpression' &&
            decl.init.callee.type === 'Identifier' &&
            decl.init.callee.name === 'require' &&
            decl.init.arguments.length === 1 &&
            decl.init.arguments[0].type === 'Literal' &&
            importsToLookFor.includes(
              decl.init.arguments[0].value as $FlowFixMe,
            )
          ) {
            if (decl.id.type === 'Identifier') {
              styleXDefaultImports.add(decl.id.name);
            }
            if (decl.id.type === 'ObjectPattern') {
              decl.id.properties.forEach((prop) => {
                if (
                  prop.type === 'Property' &&
                  prop.key.type === 'Identifier' &&
                  prop.key.name === 'create' &&
                  !prop.computed &&
                  prop.value.type === 'Identifier'
                ) {
                  styleXCreateImports.add(prop.value.name);
                }
              });
            }
          }
        });

        others
          .filter((decl) => decl.id.type === 'Identifier')
          .forEach((decl: VariableDeclarator) => {
            const id: ?Identifier =
              decl.id.type === 'Identifier' ? decl.id : null;
            const init = decl.init;
            if (id != null && init != null) {
              variables.set(id.name, init);
            }
          });
      },
      ImportDeclaration(node: ImportDeclaration) {
        if (
          node.source.type !== 'Literal' ||
          typeof node.source.value !== 'string'
        ) {
          return;
        }
        const sourceValue = node.source.value;

        const foundImportSource = importsToLookFor.find((importSource) => {
          if (typeof importSource === 'string') {
            return importSource === sourceValue;
          }
          return importSource.from === sourceValue;
        });

        const isStylexImport = foundImportSource !== undefined;
        const isStylexResolvedVarsImport =
          isValidStylexResolvedVarsFileExtension(
            sourceValue,
            themeFileExtension,
          );

        if (!(isStylexImport || isStylexResolvedVarsImport)) {
          return;
        }
        if (isStylexImport) {
          if (typeof foundImportSource === 'string') {
            node.specifiers.forEach((specifier) => {
              if (
                specifier.type === 'ImportDefaultSpecifier' ||
                specifier.type === 'ImportNamespaceSpecifier'
              ) {
                styleXDefaultImports.add(specifier.local.name);
              }
              if (
                specifier.type === 'ImportSpecifier' &&
                specifier.imported.name === 'create'
              ) {
                styleXCreateImports.add(specifier.local.name);
              }
              if (
                specifier.type === 'ImportSpecifier' &&
                specifier.imported.name === 'keyframes'
              ) {
                styleXKeyframesImports.add(specifier.local.name);
              }
              if (
                specifier.type === 'ImportSpecifier' &&
                specifier.imported.name === 'positionTry'
              ) {
                styleXPositionTryImports.add(specifier.local.name);
              }
              if (
                specifier.type === 'ImportSpecifier' &&
                specifier.imported.name === 'firstThatWorks'
              ) {
                styleXFirstThatWorksImports.add(specifier.local.name);
              }
              if (
                specifier.type === 'ImportSpecifier' &&
                specifier.imported.name === 'when'
              ) {
                styleXWhenImports.add(specifier.local.name);
              }
              if (
                specifier.type === 'ImportSpecifier' &&
                specifier.imported.name === 'env'
              ) {
                styleXEnvImports.add(specifier.local.name);
              }
            });
          }

          if (typeof foundImportSource === 'object') {
            node.specifiers.forEach((specifier) => {
              if (specifier.type === 'ImportSpecifier') {
                if (specifier.imported.name === foundImportSource.as) {
                  styleXDefaultImports.add(specifier.local.name);
                }
              }
            });
          }
        }

        if (isStylexResolvedVarsImport) {
          node.specifiers.forEach((specifier) => {
            if (specifier.type === 'ImportSpecifier') {
              stylexResolvedVarsTokenImports.add(specifier.local.name);
            }
          });
        }
      },
      // Track same-file token declarations in .stylex files.
      // This handles the case where defineVars/defineVarsNested/defineConstsNested
      // is defined and consumed via stylex.create in the same .stylex file.
      ExportNamedDeclaration(node: any): void {
        if (isStylexFile && node.declaration?.type === 'VariableDeclaration') {
          for (const decl of node.declaration.declarations) {
            if (decl.id?.type === 'Identifier') {
              stylexResolvedVarsTokenImports.add(decl.id.name);
            }
          }
        }
      },
      CallExpression(node: { ...CallExpression, ...Rule.NodeParentExtension }) {
        if (!isStylexDeclaration(node)) {
          return;
        }

        const namespaces = node.arguments[0];
        // const loc: ?AST['SourceLocation'] = namespaces.loc;
        if (namespaces.type !== 'ObjectExpression') {
          return context.report({
            node: namespaces,
            loc: namespaces.loc,
            message: 'Styles must be represented as JavaScript objects',
          } as Rule.ReportDescriptor);
        }

        namespaces.properties.forEach((namespace) => {
          if (namespace.type !== 'Property') {
            return context.report({
              node: namespace,
              loc: namespace.loc,
              message: 'Styles cannot be spread objects',
            });
          }

          let styles = namespace.value;

          if (styles.type !== 'ObjectExpression') {
            if (
              styles.type === 'ArrowFunctionExpression' &&
              (styles.body.type === 'ObjectExpression' ||
                // $FlowFixMe[invalid-compare]
                (styles.body.type === 'TSAsExpression' &&
                  // $FlowFixMe[invalid-compare]
                  styles.body.expression.type === 'ObjectExpression'))
            ) {
              const params = styles.params;
              styles = styles.body;
              // $FlowFixMe[invalid-compare]
              if (styles.type === 'TSAsExpression') {
                styles = styles.expression;
              }
              if (params.some((param) => param.type !== 'Identifier')) {
                return params
                  .filter((param) => param.type !== 'Identifier')
                  .forEach((param) => {
                    context.report({
                      node: param,
                      loc: param.loc,
                      message:
                        'Dynamic Styles can only accept named parameters. Destructuring, spreading or default parameters are not allowed.',
                    });
                  });
              }
              params.forEach((param) => {
                if (param.type === 'Identifier') {
                  dynamicStyleVariables.add(param.name);
                }
              });
            } else {
              // This case should be already handled by type checking.
              return;
            }
          }
          styles.properties.forEach((prop) =>
            checkStyleProperty(prop, 0, null, false),
          );
          // Reset local variables.
          dynamicStyleVariables.clear();
        });
      },
      'Program:exit'() {
        variables.clear();
      },
    };
  },
};
export default stylexValidStyles as typeof stylexValidStyles;
/* eslint-enable object-shorthand */
