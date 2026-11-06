/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import parser from 'postcss-value-parser';
import { hasValidTokenBoundaries, isNumericUnit } from 'style-value-parser';
import * as errMsgs from './evaluation-errors';

// Custom property names may contain any character except ')' — including
// unicode identifiers, which `defineVars`/`defineConsts` pass through
// verbatim for keys that start with '--'.
const CSS_VAR_PATTERN = /^var\(--[^)]+\)$/;

function isBalancedCalc(value: string): boolean {
  if (!value.startsWith('calc(') || !value.endsWith(')')) {
    return false;
  }
  let depth = 0;
  for (let i = 4; i < value.length; i++) {
    const char = value[i];
    if (char === '(') {
      depth++;
    } else if (char === ')') {
      depth--;
      // The opening paren of `calc(` must close at the very last character,
      // so strings like `calc(1) + calc(2)` are not a single calc term.
      if (depth === 0) {
        return i === value.length - 1;
      }
    }
  }
  return false;
}

export function isCssVarOrCalc(value: mixed): implies value is string {
  return (
    typeof value === 'string' &&
    (CSS_VAR_PATTERN.test(value) || isBalancedCalc(value))
  );
}

// A finite number or a var()/calc() reference. The only operand kinds
// allowed for '+', where a unit string would be ambiguous with the
// space-separated list concatenation that `+` also expresses.
export function isStrictCalcTerm(
  value: mixed,
): implies value is number | string {
  if (typeof value === 'number') {
    return Number.isFinite(value);
  }
  return isCssVarOrCalc(value);
}

// A strict calc term, or a numeric string with a known CSS unit ('10px',
// '1.5rem', '50%'). Used for '-', '*', and '/', which have no
// string-concatenation reading to preserve.
export function isCalcTerm(value: mixed): implies value is number | string {
  if (isStrictCalcTerm(value)) {
    return true;
  }
  if (typeof value !== 'string') {
    return false;
  }
  const parsed = parser.unit(value);
  if (parsed === false || parsed.number === '') {
    return false;
  }
  const unit = parsed.unit.toLowerCase();
  return unit === '' || isNumericUnit(unit);
}

function calcOperandToString(value: number | string): string {
  if (typeof value === 'number') {
    // These are intermediate operands, not final declaration values. Rounding
    // here can turn a small nonzero divisor into zero or overflow a large one.
    return String(value);
  }
  if (isBalancedCalc(value)) {
    // Strip nested `calc(...)` down to plain parens to keep the output flat.
    return `(${value.slice(5, -1)})`;
  }
  return value;
}

export function buildBinaryCalc(
  left: number | string,
  operator: string,
  right: number | string,
): string {
  return `calc(${calcOperandToString(left)} ${operator} ${calcOperandToString(right)})`;
}

export function buildUnaryMinusCalc(arg: number | string): string {
  return `calc(-1 * ${calcOperandToString(arg)})`;
}

export type CssTokenEvaluationResult =
  | { type: 'unhandled' }
  | { type: 'value', value: mixed }
  | { type: 'deopt', reason: string };

function value(value: mixed): CssTokenEvaluationResult {
  return { type: 'value', value };
}

function deopt(reason: string): CssTokenEvaluationResult {
  return { type: 'deopt', reason };
}

const unhandled: CssTokenEvaluationResult = { type: 'unhandled' };

export function evaluateCssTokenConcat(
  parts: $ReadOnlyArray<mixed>,
): CssTokenEvaluationResult {
  const offsets = [];
  let css = '';
  for (const part of parts) {
    if (isCssVarOrCalc(part)) {
      offsets.push(css.length);
      css += '0';
    } else {
      css += String(part);
    }
  }
  if (offsets.length > 0 && !hasValidTokenBoundaries(css, offsets)) {
    return deopt(errMsgs.INVALID_CSS_VAR_CONCAT);
  }
  return value(parts.map((part) => String(part)).join(''));
}

export function evaluateCssTokenUnary(
  operator: string,
  arg: mixed,
): CssTokenEvaluationResult {
  if (!isCssVarOrCalc(arg)) {
    return unhandled;
  }

  switch (operator) {
    case '-':
      return value(buildUnaryMinusCalc(arg));
    case '!':
    case '+':
    case '~':
    case 'typeof':
      return deopt(errMsgs.UNSUPPORTED_CSS_VAR_OPERATOR(operator));
    default:
      return unhandled;
  }
}

export function evaluateCssTokenCondition(
  arg: mixed,
): CssTokenEvaluationResult {
  // Treat var()/calc() strings as opaque, including literal spellings, just as
  // arithmetic and comparisons do elsewhere in this evaluator.
  return isCssVarOrCalc(arg)
    ? deopt(errMsgs.UNSUPPORTED_CSS_VAR_CONDITION)
    : unhandled;
}

export function evaluateCssTokenBinary(
  operator: string,
  left: mixed,
  right: mixed,
): CssTokenEvaluationResult {
  if (!isCssVarOrCalc(left) && !isCssVarOrCalc(right)) {
    return unhandled;
  }

  switch (operator) {
    case '+':
      // Only numbers and var()/calc() refs become calc() addition. A unit
      // string operand is excluded because `+` on strings also expresses list
      // concatenation, and `token + '4px'` vs `token + ' 4px'` must not
      // silently mean different things.
      if (isStrictCalcTerm(left) && isStrictCalcTerm(right)) {
        return value(buildBinaryCalc(left, operator, right));
      }
      return evaluateCssTokenConcat([left, right]);
    case '-':
    case '*':
    case '/':
      if (isCalcTerm(left) && isCalcTerm(right)) {
        return value(buildBinaryCalc(left, operator, right));
      }
      return deopt(errMsgs.INVALID_CALC_OPERAND(operator));
    case '==':
    case '!=':
    case '===':
    case '!==':
    case '<':
    case '>':
    case '<=':
    case '>=':
      return deopt(errMsgs.UNSUPPORTED_CSS_VAR_COMPARISON(operator));
    default:
      return deopt(errMsgs.UNSUPPORTED_CSS_VAR_OPERATOR(operator));
  }
}

export function evaluateCssTokenCall(
  calleeName: ?string,
  args: $ReadOnlyArray<mixed>,
): CssTokenEvaluationResult {
  const cssArgIndex = args.findIndex((arg) => isCssVarOrCalc(arg));
  if (calleeName == null || cssArgIndex === -1) {
    return unhandled;
  }

  if (calleeName === 'Number' && cssArgIndex === 0 && args.length > 0) {
    return value(args[0]);
  }

  if (calleeName === 'String') {
    return unhandled;
  }

  return deopt(errMsgs.UNSUPPORTED_CSS_VAR_FUNCTION(calleeName));
}
