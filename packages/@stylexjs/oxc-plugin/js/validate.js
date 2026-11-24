/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

// Option checks with the Babel plugin's exact messages. Ported from
// @stylexjs/babel-plugin src/utils/validate.js (MIT). Messages must not drift:
// tests and the golden fixtures compare them byte for byte.

const defaultMessage = (expected) => (value, name) =>
  name
    ? `Expected (${name}) to be ${expected}, but got \`${JSON.stringify(value)}\`.`
    : expected;

const defaultUnionMessage = (expected) => (value, name) =>
  name ? `Expected (${name}) to be ${expected}` : expected;

const defaultObjectMessage = (expected) => (value, name) =>
  name ? `Expected (${name}) to be ${expected} but:` : expected;

const indent = (str) =>
  str
    .split('\n')
    .filter((line) => !line.trim().startsWith('But got:'))
    .map((line) => (line.includes(', but got') ? line.replace(/, but got.+$/, '') : line))
    .map((line) => (line.trim()[0] === '-' ? line : `- ${line}`))
    .map((line) => `\n\t${line}`)
    .join('');

const string =
  (message = defaultMessage('a string')) =>
  (value, name) =>
    typeof value !== 'string' ? new Error(message(value, name)) : value;

const nullish =
  (message = defaultMessage('`null` or `undefined`')) =>
  (value, name) =>
    value == null ? value : new Error(message(value, name));

const optional = (check) => (value, name) => (value === undefined ? undefined : check(value, name));

const boolean =
  (message = defaultMessage('a boolean')) =>
  (value, name) =>
    typeof value !== 'boolean' ? new Error(message(value, name)) : value;

const func =
  (message = defaultMessage('a function')) =>
  (value, name) =>
    typeof value !== 'function' ? new Error(message(value, name)) : value;

const literal =
  (expected, message = defaultMessage(`the literal ${JSON.stringify(expected)}`)) =>
  (value, name) =>
    value === expected ? expected : new Error(message(value, name));

const array =
  (check, message = defaultMessage('an array')) =>
  (value, name = 'array') => {
    if (!Array.isArray(value)) return new Error(message(value, name));
    const validated = value.map((item, i) => check(item, name ? `${name}[${i}]` : undefined));
    const errors = validated.filter((item) => item instanceof Error);
    if (errors.length > 0) {
      const list = errors.map((item) => '\t' + item.message).join('\n');
      return new Error(`Failed to validate ${name}:\n${list}`);
    }
    return validated.filter((item) => !(item instanceof Error));
  };

const object =
  (shape, message = defaultMessage('an object where:')) =>
  (value, name) => {
    if (typeof value !== 'object' || value == null) return new Error(message(value, name));
    const result = {};
    for (const key in shape) {
      const check = shape[key];
      const item = check(value[key], name ? `${name}.${key}` : `obj.${key}`);
      if (item instanceof Error) {
        const description = Object.entries(shape)
          .map(([k, c]) => {
            let msg = c(Symbol()).message;
            if (msg.includes('\n')) msg = indent(indent(msg)).split('\n').slice(1).join('\n');
            return `\t- Expected "${k}": to be ${msg}`;
          })
          .join('\n');
        return new Error(`${message(value, name)}\n${description}\nBut got: ${indent(JSON.stringify(value))}`);
      }
      result[key] = item;
    }
    return result;
  };

const objectOf =
  (check, message = defaultObjectMessage('an object')) =>
  (value, name) => {
    if (typeof value !== 'object' || value == null) return new Error(message(value, name));
    const result = {};
    for (const key in value) {
      const item = check(value[key], name ? `${name}.${key}` : `With the key '${key}':`);
      if (item instanceof Error) {
        return new Error(`${message(value, name)}${indent(item.message)}\nBut got: ${indent(JSON.stringify(value))}`);
      }
      result[key] = item;
    }
    return result;
  };

// unionOf, unionOf3 and unionOf4 in the original: one variadic helper, same messages.
const unionOf =
  (...checks) =>
  (value, name) => {
    const errors = [];
    for (const check of checks) {
      const r = check(value);
      if (!(r instanceof Error)) return r;
      errors.push(r);
    }
    return new Error(
      `${defaultUnionMessage('one of')(value, name)}${errors.map((e) => indent(e.message)).join('')}\nBut got: ${JSON.stringify(value)}`,
    );
  };

const PREFIX = '[@stylexjs/babel-plugin]';

// Babel logs `console.error(PREFIX, message)` and falls back to the default.
// Here the message is collected (prefix joined with a space, as the console
// would print it) so callers decide where it goes.
function logAndDefault(check, value, def, name, errors) {
  const result = check(value, name);
  if (result instanceof Error) {
    errors.push(`${PREFIX} ${result.message}`);
    return def;
  }
  return result;
}

module.exports = {
  array,
  boolean,
  func,
  literal,
  logAndDefault,
  nullish,
  object,
  objectOf,
  optional,
  string,
  unionOf,
  PREFIX,
};
