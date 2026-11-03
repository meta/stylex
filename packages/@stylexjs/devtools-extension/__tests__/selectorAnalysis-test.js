/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @jest-environment jsdom
 */

import { parseSelectorCandidates } from '../src/inspected/selectorAnalysis';

test('parses comma selectors, escaped classes, and pseudo combinations', () => {
  const parent = document.createElement('div');
  const element = document.createElement('button');
  parent.className = 'group';
  element.className = 'sm:hidden target';
  parent.appendChild(element);
  document.body.appendChild(parent);

  const candidates = parseSelectorCandidates(
    '.sm\\:hidden, .group:hover .target:focus::before',
    element,
    new Set(element.classList),
  );

  expect(candidates.map(({ className }) => className)).toEqual([
    'sm:hidden',
    'group',
    'target',
  ]);
  expect(
    candidates.find(({ className }) => className === 'target'),
  ).toMatchObject({
    matchesClass: true,
    pseudoElement: '::before',
    selectorConditions: [
      expect.objectContaining({
        kind: 'selector',
        text: '.group:hover &:focus',
      }),
    ],
  });
});

test('parses functional pseudos without splitting selector delimiters', () => {
  const element = document.createElement('div');
  element.className = 'xfoo';
  document.body.appendChild(element);

  const candidates = parseSelectorCandidates(
    '.xfoo:not(.disabled, .loading)',
    element,
    new Set(element.classList),
  );

  expect(candidates[0].className).toBe('xfoo');
  expect(candidates[0].matchesClass).toBe(true);
});

test('preserves functional pseudo-element arguments', () => {
  const element = document.createElement('div');
  element.className = 'xfoo';
  document.body.appendChild(element);

  const candidates = parseSelectorCandidates(
    '.xfoo::part(label)',
    element,
    new Set(element.classList),
  );

  expect(candidates[0]).toMatchObject({
    className: 'xfoo',
    matchesClass: true,
    pseudoElement: '::part(label)',
  });
});

test('distinguishes defineVars root fallbacks from createTheme selectors', () => {
  const element = document.createElement('div');
  element.className = 'xVars xTheme';
  document.body.appendChild(element);

  const variableCandidates = parseSelectorCandidates(
    ':root, .xVars',
    element,
    new Set(element.classList),
  );
  const themeCandidates = parseSelectorCandidates(
    '.xTheme, .xTheme:root',
    element,
    new Set(element.classList),
  );

  expect(variableCandidates).toEqual([
    expect.objectContaining({
      className: 'xVars',
      isRootFallback: true,
      matchesClass: true,
    }),
  ]);
  expect(themeCandidates).toEqual(
    expect.arrayContaining([
      expect.objectContaining({
        className: 'xTheme',
        contextSelector: 'default',
        isRootFallback: false,
        matchesClass: true,
      }),
    ]),
  );
  expect(themeCandidates.every(({ isRootFallback }) => !isRootFallback)).toBe(
    true,
  );
});

test('collapses repeated atomic classes used for specificity', () => {
  const element = document.createElement('div');
  element.className = 'x1w1tq1y';
  document.body.appendChild(element);

  const candidates = parseSelectorCandidates(
    '.x1w1tq1y.x1w1tq1y, .x1w1tq1y.x1w1tq1y:hover',
    element,
    new Set(element.classList),
  );

  expect(candidates).toHaveLength(2);
  expect(candidates[0]).toMatchObject({
    className: 'x1w1tq1y',
    contextSelector: 'default',
    selectorConditions: [],
  });
  expect(candidates[1]).toMatchObject({
    className: 'x1w1tq1y',
    contextSelector: ':hover',
    selectorConditions: [
      expect.objectContaining({ kind: 'selector', text: ':hover' }),
    ],
  });
});
