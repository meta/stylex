/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

type Groups = { [key: number]: { start: ?number, rules: Array<string> } };
type SeenRules = { [key: string]: boolean };

export type OrderedCSSStyleSheet = Readonly<{
  getTextContent: () => string,
  insert: (cssText: string, groupValue: number) => void,
  update: (oldCssText: string, newCssText: string, groupValue: number) => void,
}>;

/**
 * Order-based insertion of CSS.
 *
 * Each rule is associated with a numerically defined group.
 * Groups are ordered within the style sheet according to their number, with the
 * lowest first.
 *
 * Groups are implemented using marker rules. The selector of the first rule of
 * each group is used only to encode the group number for hydration. An
 * alternative implementation could rely on CSSMediaRule, allowing groups to be
 * treated as a sub-sheet, but the Edge implementation of CSSMediaRule is
 * broken.
 * https://developer.mozilla.org/en-US/docs/Web/API/CSSMediaRule
 * https://gist.github.com/necolas/aa0c37846ad6bd3b05b727b959e82674
 */
export function createOrderedCSSStyleSheet(
  sheet: ?CSSStyleSheet,
): OrderedCSSStyleSheet {
  const groups: Groups = {};
  const seenRules: SeenRules = {};

  /**
   * Hydrate approximate record from any existing rules in the sheet.
   */
  if (sheet != null) {
    let group = 0;
    for (let i = 0; i < sheet.cssRules.length; i++) {
      const cssRule = sheet.cssRules[i];
      const cssText = cssRule.cssText;
      // Create record of existing selectors and rules
      if (cssText.indexOf('stylesheet-group') > -1) {
        group = decodeGroupRule(cssRule);
        groups[group] = { start: i, rules: [cssText] };
      } else {
        const key = getSeenRuleKey(cssText);
        if (key != null) {
          seenRules[key] = true;
          groups[group].rules.push(cssText);
        }
      }
    }
  }

  function sheetInsert(sheet: CSSStyleSheet, group: number, text: string) {
    const orderedGroups = getOrderedGroups(groups);
    const groupIndex = orderedGroups.indexOf(group);
    const nextGroupIndex = groupIndex + 1;
    const nextGroup = orderedGroups[nextGroupIndex];
    // Insert rule before the next group, or at the end of the stylesheet
    const position =
      nextGroup != null && groups[nextGroup].start != null
        ? groups[nextGroup].start
        : sheet.cssRules.length;
    const isInserted = insertRuleAt(sheet, text, position);

    if (isInserted) {
      // Set the starting index of the new group
      if (groups[group].start == null) {
        groups[group].start = position;
      }
      // Increment the starting index of all subsequent groups
      for (let i = nextGroupIndex; i < orderedGroups.length; i += 1) {
        const groupNumber = orderedGroups[i];
        const previousStart = groups[groupNumber].start || 0;
        groups[groupNumber].start = previousStart + 1;
      }
    }

    return isInserted;
  }

  function insert(cssText: string, groupValue: number) {
    const group = Number(groupValue);

    if (groups[group] == null) {
      const markerRule = encodeGroupRule(group);
      groups[group] = { start: null, rules: [markerRule] };
      if (sheet != null) {
        sheetInsert(sheet, group, markerRule);
      }
    }

    const key = getSeenRuleKey(cssText);
    if (key != null && seenRules[key] == null) {
      seenRules[key] = true;
      let shouldUpdate = true;
      if (sheet != null) {
        const isInserted = sheetInsert(sheet, group, cssText);
        if (!isInserted) {
          shouldUpdate = false;
        }
      }
      if (shouldUpdate) {
        groups[group].rules.push(cssText);
      }
    }
  }

  function update(oldCssText: string, newCssText: string, groupValue: number) {
    const group = Number(groupValue);
    const oldKey = getSeenRuleKey(oldCssText);
    const newKey = getSeenRuleKey(newCssText);

    if (oldKey !== newKey || oldKey == null) {
      insert(newCssText, groupValue);
      return;
    }

    if (seenRules[oldKey]) {
      if (groups[group] && groups[group].rules) {
        const rules = groups[group].rules;
        let foundIndex = -1;
        for (let i = 0; i < rules.length; i++) {
          if (getSeenRuleKey(rules[i]) === oldKey) {
            foundIndex = i;
            break;
          }
        }
        if (foundIndex !== -1) {
          rules[foundIndex] = newCssText;
        }
      }

      if (sheet != null) {
        const cssRules = sheet.cssRules;
        for (let i = cssRules.length - 1; i >= 0; i--) {
          const rule = cssRules[i];
          const ruleCssText = rule.cssText;
          const ruleKey = getSeenRuleKey(ruleCssText);
          if (ruleKey === oldKey) {
            try {
              sheet.deleteRule(i);
              sheetInsert(sheet, group, newCssText);
              break;
            } catch (e) {
              // Ignore errors
            }
          }
        }
      }
    } else {
      insert(newCssText, groupValue);
    }
  }

  const OrderedCSSStyleSheet = {
    /**
     * The textContent of the style sheet.
     */
    getTextContent(): string {
      return getOrderedGroups(groups)
        .map((group) => {
          const rules = groups[group].rules;
          // Sorting provides deterministic order of styles in group for
          // build-time extraction of the style sheet.
          const marker = rules.shift();
          rules.sort();
          if (marker !== undefined) {
            rules.unshift(marker);
          }
          return rules.join('\n');
        })
        .join('\n');
    },

    /**
     * Insert a rule into the style sheet
     */
    insert,

    /**
     * Update an existing rule with new cssText
     */
    update,
  } as const;

  return OrderedCSSStyleSheet;
}

/**
 * Helper functions
 */

function encodeGroupRule(group: number) {
  return `[stylesheet-group="${group}"]{}`;
}

const groupPattern = /["']/g;
function decodeGroupRule(cssRule: CSSRule) {
  // $FlowExpectedError[prop-missing]
  return Number(cssRule.selectorText.split(groupPattern)[1]);
}

function getOrderedGroups(obj: { [key: number]: any }) {
  return Object.keys(obj)
    .map(Number)
    .sort((a, b) => (a > b ? 1 : -1));
}

// Generate a key for cssText, attempting to correct for cssText differences
// when hydrating from an existing style sheet.
//
// This way is more reliable than using the whole cssText.
// The browser excludes vendor-prefixed properties and rewrites
// certain values making cssText more likely to be different from the string
// that was inserted.
const selectorPattern = /\s*([,])\s*/g;
// Any conditional prelude, up to its block: a compound query (`… and (…)`), a
// named container or a media type is still one condition around a rule.
const conditionalRulePattern = /^@(media|supports|container)\b[^{]*\{/;
function getSeenRuleKey(cssText: string): string | null {
  if (conditionalRulePattern.test(cssText)) {
    const index = cssText.indexOf('{');
    const query = cssText.substring(0, index).trim();
    const rest = cssText.substring(index + 1).trim();
    const next = getSeenRuleKey(rest);
    const normalizedNext =
      next !== null && next !== '' ? next.replace(selectorPattern, '$1') : '';
    return `${query} { ${normalizedNext}`;
  } else {
    // Simple CSS rule, extract selector text directly
    const selector = cssText.split('{')[0].trim();
    return selector !== '' ? selector.replace(selectorPattern, '$1') : null;
  }
}

// Utility around style sheet insertion
function insertRuleAt(
  root: CSSStyleSheet,
  cssText: string,
  position: number,
): boolean {
  try {
    root.insertRule(cssText, position);
    return true;
  } catch (e) {
    // JSDOM doesn't support `CSSSMediaRule#insertRule`.
    // Also ignore errors that occur from attempting to insert vendor-prefixed selectors.
    return false;
  }
}
