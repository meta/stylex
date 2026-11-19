/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { MediaQueryRule } from './media-query';

import { MediaQuery } from './media-query.js';

// Every property that varies by media query repeats the same few query
// strings, and parsing one is far more expensive than looking it up. The parsed
// queries are only read afterwards: combining them builds new nodes.
const parsedMediaQueries: Map<string, MediaQuery> = new Map();

function parseMediaQuery(key: string): MediaQuery {
  let parsed = parsedMediaQueries.get(key);
  if (parsed === undefined) {
    parsed = MediaQuery.parser.parseToEnd(key);
    parsedMediaQueries.set(key, parsed);
  }
  return parsed;
}

export function lastMediaQueryWinsTransform(styles: Object): Object {
  return dfsProcessQueries(styles, 0);
}

function combineMediaQueryWithNegations(
  current: MediaQuery,
  negations: MediaQuery[],
): MediaQuery {
  if (negations.length === 0) {
    return current;
  }

  let combinedAst;

  if (current.queries.type === 'or') {
    combinedAst = {
      type: 'or',
      rules: current.queries.rules.map((rule) => ({
        type: 'and',
        rules: [
          rule,
          ...negations.map((mq) => ({ type: 'not', rule: mq.queries })),
        ],
      })),
    };
  } else {
    combinedAst = {
      type: 'and',
      rules: [
        current.queries,
        ...negations.map((mq) => ({ type: 'not', rule: mq.queries })),
      ],
    } as const;
  }

  return new MediaQuery(combinedAst as $FlowFixMe as MediaQueryRule);
}

function dfsProcessQueries(
  obj: { [key: string]: any },
  depth: number,
): {
  [key: string]: any,
} {
  if (Array.isArray(obj)) {
    // Ignore `firstThatWorks` arrays
    return obj;
  }
  const result: { [key: string]: any } = {};

  Object.entries(obj).forEach(([key, value]) => {
    if (typeof value === 'object' && value !== null) {
      result[key] = dfsProcessQueries(value, depth + 1);
    } else {
      result[key] = value;
    }
  });

  if (
    depth >= 1 &&
    Object.keys(result).some((key) => key.startsWith('@media '))
  ) {
    const mediaKeys = Object.keys(result).filter((key) =>
      key.startsWith('@media '),
    );

    const negations = [];
    const accumulatedNegations = [];

    for (let i = mediaKeys.length - 1; i > 0; i--) {
      // Skip last iteration
      const mediaQuery = parseMediaQuery(mediaKeys[i]);
      negations.push(mediaQuery);
      accumulatedNegations.push([...negations]); // Clone array before pushing
    }
    accumulatedNegations.reverse();
    accumulatedNegations.push([]);

    for (let i = 0; i < mediaKeys.length; i++) {
      const currentKey = mediaKeys[i];
      const currentValue = result[currentKey];

      const baseMediaQuery = parseMediaQuery(currentKey);
      const reversedNegations = [...accumulatedNegations[i]].reverse();

      const combinedQuery = combineMediaQueryWithNegations(
        baseMediaQuery,
        reversedNegations,
      );

      const newMediaKey = combinedQuery.toString();

      delete result[currentKey];
      result[newMediaKey] = currentValue;
    }
  }

  return result;
}
