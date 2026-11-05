/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

const isVar = (arg: string | number) =>
  typeof arg === 'string' && arg.match(/^var\(--[a-zA-Z0-9-_]+\)$/);

export default function stylexFirstThatWorks(
  ...args: ReadonlyArray<string | number>
): ReadonlyArray<string | number> | string {
  const firstVar = args.findIndex(isVar);

  if (firstVar === -1) {
    return [...args].reverse();
  }
  const priorities = args.slice(0, firstVar).reverse();
  const rest = args.slice(firstVar);
  const firstNonVar = rest.findIndex((arg) => !isVar(arg));
  const varParts = rest
    .slice(0, firstNonVar === -1 ? rest.length : firstNonVar + 1)
    .reverse();

  // A number fallback only gets its unit once the property is known, so keep
  // each var separate and let the compiler add the unit and nest them.
  if (typeof varParts[0] === 'number') {
    return [...varParts, ...priorities];
  }

  const vars = varParts
    .map(String)
    .map((arg) => (isVar(arg) ? arg.slice(4, -1) : arg));

  const varChain = vars.reduce<string>(
    (soFar, varName) =>
      soFar
        ? `var(${varName}, ${String(soFar)})`
        : varName.startsWith('--')
          ? `var(${varName})`
          : varName,
    '',
  );
  if (priorities.length === 0) {
    return varChain;
  }
  return [varChain, ...priorities];
}
