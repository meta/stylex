/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { NodePath } from '@babel/traverse';
import type { PluginObj } from '@babel/core';
import type { StyleXOptions } from './utils/state-manager';

import * as t from '@babel/types';
import { getPriority } from '@stylexjs/shared';
import { MediaQuery } from 'style-value-parser';
import type { MediaQueryRule } from 'style-value-parser';
import StateManager from './utils/state-manager';
import {
  EXTENSIONS,
  filePathResolver,
  matchesFileSuffix,
  getRelativePath,
} from './utils/state-manager';
import { readImportDeclarations, readRequires } from './visitors/imports';
import transformStyleXCreate from './visitors/stylex-create';
import transformStyleXCreateTheme from './visitors/stylex-create-theme';
import transformStyleXDefineVars from './visitors/stylex-define-vars';
import transformStyleXDefineConsts from './visitors/stylex-define-consts';
import transformStyleXKeyframes from './visitors/stylex-keyframes';
import transformStyleXPositionTry from './visitors/stylex-position-try';
import transformStylexCall, {
  skipStylexMergeChildren,
} from './visitors/stylex-merge';
import transformStylexProps from './visitors/stylex-props';
import { skipStylexPropsChildren } from './visitors/stylex-props';
import transformStyleXViewTransitionClass from './visitors/stylex-view-transition-class';
import transformStyleXDefaultMarker from './visitors/stylex-default-marker';
import {
  LOGICAL_FLOAT_START_VAR,
  LOGICAL_FLOAT_END_VAR,
} from './shared/preprocess-rules/legacy-expand-shorthands';
import transformStyleXDefineMarker from './visitors/stylex-define-marker';
import transformStyleXDefineVarsNested from './visitors/stylex-define-vars-nested';
import transformStyleXDefineConstsNested from './visitors/stylex-define-consts-nested';
import transformStyleXCreateThemeNested from './visitors/stylex-create-theme-nested';
import { createUtilityStylesVisitor } from '@stylexjs/atoms/babel-transform';
import { convertObjectToAST } from './utils/js-to-ast';
import styleXCreateSet from './shared/stylex-create';
import { getProgramPath, hoistExpression } from './utils/ast-helpers';
import { injectDevClassNames } from './utils/dev-classname';

const NAME = 'stylex';

export type Options = StyleXOptions;

/**
 * Entry point for the StyleX babel plugin.
 */
function styleXTransform(): PluginObj<> {
  // To simplify state management, we use a StateManager object to abstract
  // away some of the details.
  let state: StateManager;

  /**
   * Babel plugins will run the `visitor` on "enter" by default.
   *
   * This visitor object can be read mostly top-down, except, Program.exit runs
   * after all other visitors.
   */
  return {
    name: NAME,
    visitor: {
      Program: {
        // First we reads all relevant imports and requires.
        // Store them in the StateManager object.
        enter: (path: NodePath<t.Program>, s: any) => {
          state = new StateManager(s);

          for (const block of path.get('body')) {
            if (block.isImportDeclaration()) {
              // Read and remember 'stylex' Imports
              // Consider user `path.referencesImport`
              // But what about `requires`?
              readImportDeclarations(block, state);
            }

            if (block.isVariableDeclaration()) {
              for (const decl of block.get('declarations')) {
                // Read and remember 'stylex' requires
                readRequires(decl, state);
              }
            }
          }
        },
        // After all other visitors are done, we can remove `styles=stylex.create(...)`
        // variables entirely if they're not needed.
        exit: (path: NodePath<t.Program>) => {
          path.traverse({
            ImportDeclaration(path: NodePath<t.ImportDeclaration>) {
              const filename = state.filename;
              if (filename == null || !state.options.rewriteAliases) {
                return;
              }

              const source = path.node.source.value;

              const aliases = state.options.aliases;

              const themeFileExtension = '.stylex';
              if (!matchesFileSuffix(themeFileExtension)(source)) {
                return;
              }
              const resolvedFilePath = filePathResolver(
                source,
                filename,
                aliases,
              );

              if (resolvedFilePath == null) {
                return;
              }

              let relativeFilePath = getRelativePath(
                filename,
                resolvedFilePath,
              );

              const extension = EXTENSIONS.find((ext) =>
                relativeFilePath.endsWith(ext),
              );
              if (extension != null) {
                relativeFilePath = relativeFilePath.slice(0, -extension.length);
              }

              path.node.source.value = relativeFilePath;
            },
            Identifier(path: NodePath<t.Identifier>) {
              // Look for variables bound to `stylex.create` calls that are used
              // outside of `stylex(...)` calls
              if (path.isReferencedIdentifier()) {
                const { name } = path.node;
                if (state.styleMap.has(name)) {
                  const parentPath = path.parentPath;
                  if (parentPath.isMemberExpression()) {
                    const { property, computed } = parentPath.node;
                    if (property.type === 'Identifier' && !computed) {
                      state.markComposedNamespace([name, property.name, true]);
                    } else if (property.type === 'StringLiteral' && computed) {
                      state.markComposedNamespace([name, property.value, true]);
                    } else if (property.type === 'NumericLiteral' && computed) {
                      state.markComposedNamespace([
                        name,
                        String(property.value),
                        true,
                      ]);
                    } else {
                      state.markComposedNamespace([name, true, true]);
                    }
                  } else {
                    state.markComposedNamespace([name, true, true]);
                  }
                }
              }
            },
            CallExpression(path: NodePath<t.CallExpression>) {
              // Don't traverse the children of `stylex(...)` calls.
              // This is important for detecting which `stylex.create()` calls
              // should be kept.
              skipStylexMergeChildren(path, state);
              skipStylexPropsChildren(path, state);
            },
          });
          // Run atoms visitor first to transform x.prop.value patterns
          // This runs BEFORE stylex.props so that atomic styles are already
          // compiled when stylex.props processes them
          const atomsVisitor = createUtilityStylesVisitor(state, {
            styleXCreateSet,
            convertObjectToAST,
            hoistExpression,
            injectDevClassNames,
          });
          path.traverse(atomsVisitor);

          path.traverse({
            CallExpression(path: NodePath<t.CallExpression>) {
              transformStylexCall(path, state);
              transformStylexProps(path, state);
            },
          });

          const varsToKeep: { [string]: true | Array<string> } = {};
          for (const [varName, namespaceName] of state.styleVarsToKeep) {
            if (varsToKeep[varName] === true) {
              continue;
            }
            if (varsToKeep[varName] == null) {
              varsToKeep[varName] =
                namespaceName === true ? true : [namespaceName];
            } else if (Array.isArray(varsToKeep[varName])) {
              if (namespaceName === true) {
                varsToKeep[varName] = true;
              } else {
                varsToKeep[varName].push(namespaceName);
              }
            }
          }

          const varsToKeepOld = new Set(
            [...state.styleVarsToKeep.values()].map(
              ([varName, _namespaceName]) => varName,
            ),
          );
          state.styleVars.forEach((path, varName) => {
            if (isExported(path)) {
              return;
            }

            if (varsToKeep[varName] === true) {
              return;
            }

            const namespacesToKeep: Array<string> = varsToKeep[varName];

            if (namespacesToKeep == null) {
              path.remove();
              return;
            }

            if (path.isVariableDeclarator()) {
              const init = path.get('init');
              if (init != null && init.isObjectExpression()) {
                for (const prop of init.get('properties')) {
                  if (prop.isObjectProperty()) {
                    const key = prop.get('key').node;
                    const keyAsString =
                      key.type === 'Identifier'
                        ? key.name
                        : key.type === 'StringLiteral'
                          ? key.value
                          : key.type === 'NumericLiteral'
                            ? String(key.value)
                            : null;

                    if (keyAsString != null) {
                      if (!namespacesToKeep.includes(keyAsString)) {
                        prop.remove();
                      } else {
                        const allNullsToKeep = [
                          ...state.styleVarsToKeep.values(),
                        ]
                          .filter(
                            ([v, namespaceName]) =>
                              v === varName && namespaceName === keyAsString,
                          )
                          .map(
                            ([_v, _namespaceName, nullPropsToKeep]) =>
                              nullPropsToKeep,
                          );
                        if (!allNullsToKeep.includes(true)) {
                          const nullsToKeep = new Set<string>(
                            allNullsToKeep
                              .filter((x): x is Array<string> => x !== true)
                              .flat(),
                          );
                          const styleObject = prop.get('value');
                          if (styleObject.isObjectExpression()) {
                            for (const styleProp of styleObject.get(
                              'properties',
                            )) {
                              if (
                                styleProp.isObjectProperty() &&
                                styleProp.get('value').isNullLiteral()
                              ) {
                                const styleKey = styleProp.get('key').node;
                                const styleKeyAsString =
                                  styleKey.type === 'Identifier'
                                    ? styleKey.name
                                    : styleKey.type === 'StringLiteral'
                                      ? styleKey.value
                                      : styleKey.type === 'NumericLiteral'
                                        ? String(styleKey.value)
                                        : null;

                                if (
                                  styleKeyAsString != null &&
                                  !nullsToKeep.has(styleKeyAsString)
                                ) {
                                  styleProp.remove();
                                }
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }

            if (!varsToKeepOld.has(varName) && !isExported(path)) {
              path.remove();
            }
          });
        },
      },

      JSXOpeningElement(path: NodePath<t.JSXOpeningElement>) {
        const sxPropName = state.options.sxPropName;
        if (sxPropName === false) {
          return;
        }
        const node = path.node;
        if (
          node.name.type !== 'JSXIdentifier' ||
          typeof node.name.name !== 'string' ||
          node.name.name[0] !== node.name.name[0].toLowerCase()
        ) {
          return;
        }
        for (const attr of path.get('attributes')) {
          if (
            !attr.isJSXAttribute() ||
            !attr.get('name').isJSXIdentifier() ||
            attr.get('name').node.name !== sxPropName
          ) {
            continue;
          }
          const valuePath = attr.get('value');
          if (!valuePath.isJSXExpressionContainer()) {
            continue;
          }
          const value = valuePath.get('expression');
          if (value.isJSXEmptyExpression()) {
            continue;
          }
          const stylexLocalName = getStylexRuntimeBinding(path, state);
          attr.replaceWith(
            t.jsxSpreadAttribute(
              t.callExpression(
                t.memberExpression(
                  t.identifier(stylexLocalName),
                  t.identifier('props'),
                ),
                [value.node],
              ),
            ),
          );
          break;
        }
      },

      CallExpression(path: NodePath<t.CallExpression>) {
        const parentPath = path.parentPath;
        if (parentPath.isVariableDeclarator()) {
          // Look for `stylex.keyframes` calls
          // Needs to be handled *before* `stylex.create` as the `create` call
          // may use the generated animation name.
          transformStyleXKeyframes(
            parentPath as NodePath<t.VariableDeclarator>,
            state,
          );
          // Look for `stylex.viewTransitionClass` calls
          // Needs to be handled *after* `stylex.keyframes` since the `viewTransitionClass`
          // call may use the generated animation name.
          transformStyleXViewTransitionClass(
            parentPath as NodePath<t.VariableDeclarator>,
            state,
          );
          // Look for `stylex.positionTry` calls
          // Needs to be handled *before* `stylex.create` as the `create` call
          // may use the generated position-try name.
          transformStyleXPositionTry(
            parentPath as NodePath<t.VariableDeclarator>,
            state,
          );
        }

        transformStyleXDefaultMarker(path, state);
        transformStyleXDefineMarker(path, state);
        transformStyleXDefineVars(path, state);
        transformStyleXDefineConsts(path, state);
        transformStyleXCreateTheme(path, state);
        transformStyleXDefineVarsNested(path, state);
        transformStyleXDefineConstsNested(path, state);
        transformStyleXCreateThemeNested(path, state);
        transformStyleXCreate(path, state);
      },
    },
  };
}

function stylexPluginWithOptions(
  options: Partial<StyleXOptions>,
): [typeof styleXTransform, Partial<StyleXOptions>] {
  return [styleXTransform, options];
}
styleXTransform.withOptions = stylexPluginWithOptions;

function isExported(path: null | NodePath<t.Node>): boolean {
  if (path == null || path.isProgram()) {
    return false;
  }
  if (path.isExportNamedDeclaration() || path.isExportDefaultDeclaration()) {
    return true;
  }
  return isExported(path.parentPath);
}

function getStylexRuntimeBinding(
  path: NodePath<t.JSXOpeningElement>,
  state: StateManager,
): string {
  const program = getProgramPath(path);
  if (program == null) {
    throw new Error('Unable to find program path for JSX element.');
  }

  for (const localName of state.stylexImport) {
    const programBinding = program.scope.getBinding(localName);
    if (
      programBinding != null &&
      path.scope.getBinding(localName) === programBinding
    ) {
      return localName;
    }
  }

  const existingImport = program.node.body.find(
    (statement) =>
      statement.type === 'ImportDeclaration' &&
      state.importSources.includes(statement.source.value),
  );
  const existingImportSource =
    existingImport?.type === 'ImportDeclaration'
      ? existingImport.source.value
      : null;
  const importSource =
    existingImportSource ?? state.importSources[2] ?? state.importSources[0];
  const stylexLocalName =
    existingImportSource != null || path.scope.hasBinding('stylex')
      ? program.scope.generateUid('stylex')
      : 'stylex';

  const [importPath] = program.unshiftContainer(
    'body',
    t.importDeclaration(
      [t.importNamespaceSpecifier(t.identifier(stylexLocalName))],
      t.stringLiteral(importSource),
    ),
  );
  program.scope.registerDeclaration(importPath);
  state.stylexImport.add(stylexLocalName);
  return stylexLocalName;
}

/**
 *
 * @param rules An array of CSS rules that has been generated and collected from all JS files
 * in a project
 * @returns A string that represents the final CSS file.
 *
 * This function take an Array of CSS rules, de-duplicates them, sorts them priority and generates
 * a final CSS file.
 *
 * When Stylex is correctly configured, the babel plugin will return an array of CSS rule objects.
 * You're expected to concatenate all the Rules into a single Array and use this function to convert
 * that into the final CSS file.
 *
 * End-users can choose to not use this function and use their own logic instead.
 */
export type Rule = [
  string,
  {
    ltr: string,
    rtl?: null | string,
    constKey?: string,
    constVal?: string | number,
  },
  number,
];

function getLogicalFloatVars(rules: Array<Rule>): string {
  const hasLogicalFloat = rules.some(([, { ltr, rtl }]) => {
    const ltrStr = String(ltr);
    const rtlStr = rtl ? String(rtl) : '';
    return (
      ltrStr.includes(LOGICAL_FLOAT_START_VAR) ||
      ltrStr.includes(LOGICAL_FLOAT_END_VAR) ||
      rtlStr.includes(LOGICAL_FLOAT_START_VAR) ||
      rtlStr.includes(LOGICAL_FLOAT_END_VAR)
    );
  });

  return hasLogicalFloat
    ? `:root, [dir="ltr"] {
  ${LOGICAL_FLOAT_START_VAR}: left;
  ${LOGICAL_FLOAT_END_VAR}: right;
}
[dir="rtl"] {
  ${LOGICAL_FLOAT_START_VAR}: right;
  ${LOGICAL_FLOAT_END_VAR}: left;
}
`
    : '';
}

type WidthBound = $ReadOnly<{ kind: 'min' | 'max', value: number }>;

// A rule's width bound plus the at-rule context it sits in. Only rules with
// an identical context are comparable: a breakpoint nested under `@supports`
// says nothing about one nested under a different condition.
type WidthSortKey = $ReadOnly<{ context: string, bound: WidthBound }>;

// The `min-width`/`max-width` px bounds a media query imposes, or null if it
// has none to sort by: `not` and `or` can widen the matched range or split it
// in two, and rem/em bounds are unreadable at build time. Zero counts as a
// bound whatever unit it carries, or none at all.
function collectWidthBounds(rule: MediaQueryRule): Array<WidthBound> | null {
  if (rule.type === 'pair') {
    // Media feature names are ASCII case-insensitive.
    const key = rule.key.toLowerCase();
    const kind =
      key === 'min-width' ? 'min' : key === 'max-width' ? 'max' : null;
    // A non-width feature (e.g. `orientation`) constrains no width.
    if (kind == null) {
      return [];
    }

    const v = rule.value;
    // Zero is the one length that needs no unit. Any other unitless number is
    // invalid CSS for a media feature — the browser drops the whole query — so
    // leave those unsorted rather than assume px.
    if (v === 0) {
      return [{ kind, value: 0 }];
    }

    if (
      v != null &&
      typeof v === 'object' &&
      // A `Fraction` value (e.g. `aspect-ratio`) is array-shaped, not a length.
      !Array.isArray(v) &&
      typeof v.value === 'number' &&
      // Units are ASCII case-insensitive, and zero is zero in any unit.
      (v.value === 0 || String(v.unit).toLowerCase() === 'px')
    ) {
      return [{ kind, value: v.value }];
    }
    return null;
  }
  if (rule.type === 'and') {
    const bounds: Array<WidthBound> = [];
    for (const r of rule.rules) {
      const found = collectWidthBounds(r);
      if (found == null) {
        return null;
      }

      bounds.push(...found);
    }

    return bounds;
  }

  // A negated media type (e.g. `not screen`) inverts the whole expression.
  if (rule.type === 'media-keyword') {
    return rule.not ? null : [];
  }

  return null;
}

// The single width bound a media query sorts by, or null if it has none.
function mediaQueryWidthSortKey(prelude: string): WidthBound | null {
  let parsed;
  try {
    parsed = MediaQuery.parser.parseToEnd(prelude);
  } catch {
    return null;
  }
  const bounds = collectWidthBounds(parsed.queries);
  // Zero bounds means no width condition; more than one (a range, or a
  // redundant pair like `(min-width: 500px) and (min-width: 900px)`) has no
  // single value to sort by.
  if (bounds == null || bounds.length !== 1) {
    return null;
  }

  return bounds[0];
}

// The leading at-rule preludes of a rule, outermost first, e.g.
// `@supports (x){@media (y){.a{…}}}` -> ['@supports (x)', '@media (y)'].
function atRulePreludes(rule: string): Array<string> {
  const preludes = [];
  let index = 0;
  while (rule[index] === '@') {
    const brace = rule.indexOf('{', index);
    if (brace === -1) {
      break;
    }

    preludes.push(rule.slice(index, brace).trimEnd());
    index = brace + 1;
  }

  return preludes;
}

// The sort key for a rule's at-rule chain, or null if it has no single
// `@media` to sort by. Media queries nested in other at-rules still sort, but
// only against rules sharing the same surrounding conditions.
function widthSortKeyForChain(preludes: Array<string>): WidthSortKey | null {
  const mediaIndexes = [];
  preludes.forEach((prelude, i) => {
    if (prelude.startsWith('@media ')) {
      mediaIndexes.push(i);
    }
  });
  // Zero means nothing to sort by; more than one means nested media queries
  // whose combined bound isn't a single value.
  if (mediaIndexes.length !== 1) {
    return null;
  }

  const mediaIndex = mediaIndexes[0];
  const bound = mediaQueryWidthSortKey(preludes[mediaIndex]);
  if (bound == null) {
    return null;
  }

  // Blank out the media query itself so the context captures the surrounding
  // at-rules and the query's depth among them, but not its breakpoint.
  const context = preludes
    .map((p, i) => (i === mediaIndex ? '@media' : p))
    .join('{');
  return { context, bound };
}

// The property name a rule declares, e.g. `.a{width:500px}` -> `width`.
// Breakpoints are ordered within a property, so the property has to be
// compared without its value — two breakpoints for the same property differ
// in value by definition, and comparing that first would preempt the order.
function declaredPropertyName(rule: string): string {
  const declaration = rule.slice(rule.lastIndexOf('{') + 1);
  const colon = declaration.indexOf(':');
  return colon === -1 ? declaration : declaration.slice(0, colon);
}

// A total ordering over width sort keys, so the sort stays consistent no
// matter which pairs get compared: rules with a bound first, then grouped by
// at-rule context, then `min-width` ascending ahead of `max-width` descending
// — the conventional mobile-first order.
function compareWidthSortKeys(
  a: WidthSortKey | null,
  b: WidthSortKey | null,
): number {
  if (a == null || b == null) {
    if (a == null && b == null) {
      return 0;
    }

    return a == null ? 1 : -1;
  }

  if (a.context !== b.context) {
    return a.context.localeCompare(b.context);
  }

  if (a.bound.kind !== b.bound.kind) {
    return a.bound.kind === 'min' ? -1 : 1;
  }

  return a.bound.kind === 'min'
    ? a.bound.value - b.bound.value
    : b.bound.value - a.bound.value;
}

function processStylexRules(
  rules: Array<Rule>,
  config?:
    | boolean
    | {
        useLayers?:
          | boolean
          | $ReadOnly<{
              before?: $ReadOnlyArray<string>,
              after?: $ReadOnlyArray<string>,
              prefix?: string,
            }>,
        enableLTRRTLComments?: boolean,
        legacyDisableLayers?: boolean,
        useLegacyClassnamesSort?: boolean,
        ...
      },
): string {
  const rawConfig =
    typeof config === 'boolean' ? { useLayers: config } : (config ?? {});
  const {
    enableLTRRTLComments = false,
    legacyDisableLayers = false,
    useLegacyClassnamesSort = false,
  } = rawConfig;

  const rawUseLayers = rawConfig.useLayers ?? false;
  const useLayers = rawUseLayers !== false;
  const layersBefore =
    typeof rawUseLayers === 'object' ? (rawUseLayers.before ?? []) : [];
  const layersAfter =
    typeof rawUseLayers === 'object' ? (rawUseLayers.after ?? []) : [];
  const layerPrefix =
    typeof rawUseLayers === 'object' ? (rawUseLayers.prefix ?? '') : '';

  if (rules.length === 0) {
    return '';
  }

  const constantRules = rules.filter(
    ([, ruleObj]) => ruleObj?.constKey != null && ruleObj?.constVal != null,
  );
  const nonConstantRules = rules.filter(
    ([, ruleObj]) => !(ruleObj?.constKey != null && ruleObj?.constVal != null),
  );

  const constsMap: Map<string, string | number> = new Map();
  for (const [keyhash, ruleObj] of constantRules) {
    // $FlowFixMe[incompatible-type] - null check above
    const constVal: string | number = ruleObj.constVal;
    const constName = `var(--${keyhash})`;
    constsMap.set(constName, constVal);
  }

  function resolveConstant(
    value: string | number,
    visited: Set<string> = new Set(),
  ): string | number {
    if (typeof value !== 'string') return value;
    const regex = /var\((--[A-Za-z0-9_-]+)\)/g;
    let result: string = value;
    let match: RegExp$matchResult | null;
    while ((match = regex.exec(result)) !== null) {
      if (match == null) continue;
      const ref = match[1];
      if (visited.has(ref)) {
        throw new Error(`circular reference detected for constant ${ref}`);
      }
      const refKey = `var(${ref})`;
      const refValue = constsMap.get(refKey);
      if (refValue == null) continue;
      visited.add(ref);
      const replacement = resolveConstant(refValue, visited);
      result = result.replace(match[0], () => replacement.toString());
      visited.delete(ref);
      regex.lastIndex = 0;
    }
    return result;
  }

  for (const [key, value] of constsMap.entries()) {
    constsMap.set(key, resolveConstant(value));
  }

  // A `defineConsts` at-rule used as a condition key in `stylex.create()` is
  // compiled as a `var(--hash){…}` wrapper, and its share of the rule's priority
  // comes from that placeholder, which `getPriority` cannot tell from a
  // selector. Now that the constant's value is known, give it the share the
  // literal at-rule gets, so the rule sorts (and lands in the layer) exactly as
  // if it were written inline. Variable and theme rules count at-rules in their
  // own fractional priorities, which never include the placeholder's share, so
  // they are left as they are.
  const constWrapperPattern = /var\(--[A-Za-z0-9_-]+\)(?=\{)/g;
  const adjustConstAtRulePriority = (
    rule: unknown,
    priority: number,
  ): number => {
    if (typeof rule !== 'string' || constsMap.size === 0) {
      return priority;
    }
    let adjusted = priority;
    for (const match of rule.matchAll(constWrapperPattern)) {
      const placeholder = match[0];
      const value = constsMap.get(placeholder);
      const placeholderShare = getPriority(placeholder);
      if (
        typeof value === 'string' &&
        value.startsWith('@') &&
        adjusted >= placeholderShare
      ) {
        adjusted += getPriority(value) - placeholderShare;
      }
    }
    return adjusted;
  };

  // Parsing is expensive and the comparator runs O(n log n) times. Key on the
  // at-rule chain, not the whole rule — every rule has a distinct class name,
  // so a full-rule key would never hit.
  const widthSortKeys: Map<string, WidthSortKey | null> = new Map();
  const getWidthSortKey = (rule: string): WidthSortKey | null => {
    if (rule[0] !== '@') {
      return null;
    }

    const preludes = atRulePreludes(rule);
    const chain = preludes.join('{');
    const cached = widthSortKeys.get(chain);
    if (cached !== undefined) {
      return cached;
    }

    const key = widthSortKeyForChain(preludes);
    widthSortKeys.set(chain, key);
    return key;
  };

  const sortedRules: Array<Rule> = nonConstantRules
    .map(([key, { ...styleObj }, priority]): Rule => {
      const adjustedPriority = adjustConstAtRulePriority(
        styleObj.ltr,
        priority,
      );
      Object.keys(styleObj).forEach((dir) => {
        let original = styleObj[dir];
        for (const [varRef, constValue] of constsMap.entries()) {
          if (typeof original !== 'string') continue;
          const replacement = String(constValue);
          original = original.replaceAll(varRef, () => replacement);
          if (replacement.startsWith('var(') && replacement.endsWith(')')) {
            const inside = replacement.slice(4, -1).trim();
            const commaIdx = inside.indexOf(',');
            const targetName = (
              commaIdx >= 0 ? inside.slice(0, commaIdx) : inside
            ).trim();
            const constName = varRef.slice(4, -1);
            original = original.replaceAll(`${constName}:`, `${targetName}:`);
          }
          styleObj[dir] = original;
        }
      });
      return [key, styleObj, adjustedPriority];
    })
    .sort(
      (
        [classname1, { ltr: rule1 }, firstPriority]: [string, any, number],
        [classname2, { ltr: rule2 }, secondPriority]: [string, any, number],
      ) => {
        const priorityComparison = firstPriority - secondPriority;
        if (priorityComparison !== 0) return priorityComparison;

        if (useLegacyClassnamesSort) {
          return classname1.localeCompare(classname2);
        } else {
          const nameComparison = declaredPropertyName(rule1).localeCompare(
            declaredPropertyName(rule2),
          );
          if (nameComparison !== 0) return nameComparison;

          // Only rules for the same property compete in the cascade, so the
          // breakpoint order applies within a property, after it. Ordering by
          // width first would decide some pairs by width and others by
          // declaration text, which is not a consistent ordering.
          const mqComparison = compareWidthSortKeys(
            getWidthSortKey(rule1),
            getWidthSortKey(rule2),
          );
          if (mqComparison !== 0) return mqComparison;

          const property1 = rule1.slice(rule1.lastIndexOf('{'));
          const property2 = rule2.slice(rule2.lastIndexOf('{'));
          const propertyComparison = property1.localeCompare(property2);
          if (propertyComparison !== 0) return propertyComparison;
          return rule1.localeCompare(rule2);
        }
      },
    );

  let lastKPri = -1;
  const grouped = sortedRules.reduce((acc: Array<Array<Rule>>, rule) => {
    const [key, styleObj, priority] = rule;
    const priorityLevel = Math.floor(priority / 1000);

    if (priorityLevel === lastKPri) {
      acc[acc.length - 1].push([key, styleObj, priority]);
      return acc;
    }

    lastKPri = priorityLevel;
    acc.push([[key, styleObj, priority]]);
    return acc;
  }, []);

  const logicalFloatVars = getLogicalFloatVars(sortedRules);

  const layerName = (index: number): string =>
    layerPrefix
      ? `${layerPrefix}.priority${index + 1}`
      : `priority${index + 1}`;

  const header = useLayers
    ? '\n@layer ' +
      [
        ...layersBefore,
        ...grouped.map((_, index) => layerName(index)),
        ...layersAfter,
      ].join(', ') +
      ';\n'
    : '';

  const collectRules = (rules: Array<Rule>, index: number): string =>
    Array.from(new Map(rules.map(([a, b]) => [a, b])).values())
      .flatMap((rule) => {
        const { ltr, rtl } = rule;
        let ltrRule = ltr,
          rtlRule = rtl;

        if (!useLayers && !legacyDisableLayers) {
          ltrRule = addSpecificityLevel(ltrRule, index);
          rtlRule = rtlRule && addSpecificityLevel(rtlRule, index);
        }

        // check if the selector looks like .xtrlmmh, .xtrlmmh:root
        // if so, turn it into .xtrlmmh.xtrlmmh, .xtrlmmh.xtrlmmh:root
        // This is to ensure the themes always have precedence over the
        // default variable values
        ltrRule = ltrRule.replace(
          /\.([a-zA-Z0-9]+), \.([a-zA-Z0-9]+):root/g,
          '.$1.$1, .$1.$1:root',
        );
        if (rtlRule) {
          rtlRule = rtlRule.replace(
            /\.([a-zA-Z0-9]+), \.([a-zA-Z0-9]+):root/g,
            '.$1.$1, .$1.$1:root',
          );
        }

        return rtlRule
          ? enableLTRRTLComments
            ? [
                `/* @ltr begin */${ltrRule}/* @ltr end */`,
                `/* @rtl begin */${rtlRule}/* @rtl end */`,
              ]
            : [
                addAncestorSelector(ltrRule, "html:not([dir='rtl'])"),
                addAncestorSelector(rtlRule, "html[dir='rtl']"),
              ]
          : [ltrRule];
      })
      .join('\n');

  const collectedCSS = grouped
    .map((group, index) => {
      // A `priorityLevel` (Math.floor(priority / 1000)) can mix rules that must
      // stay outside layers (`@property`, `@keyframes`, `@position-try` are all
      // priority 0) with rules that belong inside them (CSS custom properties
      // are priority 1). Deciding based on `group[0][2]` alone would let the
      // first rule's priority pull the whole group out of its layer, so we
      // partition the group by whether each rule is layerable (priority > 0).
      if (!useLayers) {
        return collectRules(group, index);
      }

      const layerable = group.filter(([, , pri]) => pri > 0);
      const unlayered = group.filter(([, , pri]) => pri === 0);

      const parts = [];
      if (unlayered.length > 0) {
        parts.push(collectRules(unlayered, index));
      }
      // Don't put @property, @keyframe, @position-try in layers
      if (layerable.length > 0) {
        parts.push(
          `@layer ${layerName(index)}{\n${collectRules(layerable, index)}\n}`,
        );
      }
      return parts.join('\n');
    })
    .join('\n');

  return logicalFloatVars + header + collectedCSS;
}

styleXTransform.processStylexRules = processStylexRules;

/**
 * Adds an ancestor selector in a media-query-aware way.
 *
 * Helper function for `processStylexRules`.
 */
function addAncestorSelector(
  selector: string,
  ancestorSelector: string,
): string {
  // These at-rules contain declarations, not selectors, so we cannot add one.
  if (
    selector.startsWith('@keyframes') ||
    selector.startsWith('@position-try')
  ) {
    return selector;
  }
  if (!selector.startsWith('@')) {
    return `${ancestorSelector} ${selector}`;
  }

  const lastAtRule = selector.lastIndexOf('@');
  const atRuleBracketIndex = selector.indexOf('{', lastAtRule);
  const mediaQueryPart = selector.slice(0, atRuleBracketIndex + 1);
  const rest = selector.slice(atRuleBracketIndex + 1);
  return `${mediaQueryPart}${ancestorSelector} ${rest}`;
}

/**
 * Adds :not(#\#) to bump up specificity. as a polyfill for @layer
 */
function addSpecificityLevel(selector: string, index: number): string {
  if (
    selector.startsWith('@keyframes') ||
    selector.startsWith('@position-try')
  ) {
    return selector;
  }
  const pseudo = Array.from({ length: index })
    .map(() => ':not(#\\#)')
    .join('');

  const lastOpenCurly = selector.includes('::')
    ? selector.indexOf('::')
    : selector.lastIndexOf('{');
  const beforeCurly = selector.slice(0, lastOpenCurly);
  const afterCurly = selector.slice(lastOpenCurly);

  return `${beforeCurly}${pseudo}${afterCurly}`;
}

export type StyleXTransformObj = $ReadOnly<{
  (): PluginObj<>,
  withOptions: typeof stylexPluginWithOptions,
  processStylexRules: typeof processStylexRules,
  ...
}>;

export default styleXTransform as StyleXTransformObj;
