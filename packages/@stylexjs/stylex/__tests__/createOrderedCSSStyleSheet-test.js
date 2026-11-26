/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

/* eslint-disable no-useless-concat */

import { createOrderedCSSStyleSheet } from '../src/stylesheet/createOrderedCSSStyleSheet';

const insertStyleElement = () => {
  const element = document.createElement('style');
  const head = document.head;
  head.insertBefore(element, head.firstChild);
  return element;
};

const removeStyleElement = (element) => {
  document.head.removeChild(element);
};

describe('createOrderedCSSStyleSheet', () => {
  describe('#insert', () => {
    test('does not throw on invalid rule', () => {
      const sheet = createOrderedCSSStyleSheet();
      expect(() => sheet.insert('.selector {', 0)).not.toThrow();
    });

    test('insertion of CSSStyleRule', () => {
      const sheet = createOrderedCSSStyleSheet();
      sheet.insert('.selector { color: red } }', 0);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        .selector { color: red } }"
      `);
    });

    test('insertion of CSSMediaRule', () => {
      const sheet = createOrderedCSSStyleSheet();
      sheet.insert('@media (min-width:320px) { .selector { color: red } }', 0);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        @media (min-width:320px) { .selector { color: red } }"
      `);
    });

    test('insertion of CSSMediaRule within CSSConditionRule', () => {
      const sheet = createOrderedCSSStyleSheet();
      sheet.insert(
        '@supports (display:grid) { ' +
          '@media (min-width:320px) { ' +
          '.selector { color: red } ' +
          '} ' +
          '}',
        0,
      );
      sheet.insert(
        '@container main (max-width: 850px) { ' +
          '@media (min-width:320px) { ' +
          '.selector { color: red } ' +
          '} ' +
          '}',
        0,
      );
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        @container main (max-width: 850px) { @media (min-width:320px) { .selector { color: red } } }
        @supports (display:grid) { @media (min-width:320px) { .selector { color: red } } }"
      `);
    });

    test('rules under a compound or named condition are each inserted once', () => {
      const sheet = createOrderedCSSStyleSheet();
      const compound =
        '@media (prefers-reduced-motion: reduce) and (pointer: coarse)';
      const named = '@container sidebar (min-width: 400px)';
      sheet.insert(`${compound} { .a { opacity: 0 } }`, 0);
      sheet.insert(`${compound} { .b { opacity: 0 } }`, 0);
      sheet.insert(`${compound} { .a { opacity: 0 } }`, 0);
      sheet.insert(`${named} { .c { opacity: 0 } }`, 0);
      sheet.insert(`${named} { .d { opacity: 0 } }`, 0);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        @container sidebar (min-width: 400px) { .c { opacity: 0 } }
        @container sidebar (min-width: 400px) { .d { opacity: 0 } }
        @media (prefers-reduced-motion: reduce) and (pointer: coarse) { .a { opacity: 0 } }
        @media (prefers-reduced-motion: reduce) and (pointer: coarse) { .b { opacity: 0 } }"
      `);
    });

    test('insertion order for same group', () => {
      const sheet = createOrderedCSSStyleSheet();

      expect(sheet.getTextContent()).toMatchInlineSnapshot('""');

      sheet.insert('.a { color: red } }', 0);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        .a { color: red } }"
      `);

      sheet.insert('.b { color: red } }', 0);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        .a { color: red } }
        .b { color: red } }"
      `);

      sheet.insert('.c { color: red } }', 0);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        .a { color: red } }
        .b { color: red } }
        .c { color: red } }"
      `);
    });

    test('deduplication for same group', () => {
      const sheet = createOrderedCSSStyleSheet();

      sheet.insert('@media (min-width: 320px) { .a { color: red } } }', 0);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        @media (min-width: 320px) { .a { color: red } } }"
      `);

      sheet.insert('@media (min-width: 320px) { .a { color: red } } }', 0);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        @media (min-width: 320px) { .a { color: red } } }"
      `);

      sheet.insert('@media (min-width: 320px) { .a { color: red } } }', 0);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        @media (min-width: 320px) { .a { color: red } } }"
      `);
    });

    test('order for same group', () => {
      const sheet = createOrderedCSSStyleSheet();

      sheet.insert('.c { color: red } }', 0);
      sheet.insert('.b { color: red } }', 0);
      sheet.insert('.a { color: red } }', 0);

      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        .a { color: red } }
        .b { color: red } }
        .c { color: red } }"
      `);
    });

    test('insertion order for different groups', () => {
      const sheet = createOrderedCSSStyleSheet();

      sheet.insert('.nine-1 {}', 9.9);
      sheet.insert('.nine-2 {}', 9.9);
      sheet.insert('.three {}', 3);
      sheet.insert('.one {}', 1);
      sheet.insert('.two {}', 2.2);
      sheet.insert('.four-1 {}', 4);
      sheet.insert('.four-2 {}', 4);
      sheet.insert('.twenty {}', 20);
      sheet.insert('.ten {}', 10);
      sheet.insert('.twenty-point2 {}', 20.2);

      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="1"]{}
        .one {}
        [stylesheet-group="2.2"]{}
        .two {}
        [stylesheet-group="3"]{}
        .three {}
        [stylesheet-group="4"]{}
        .four-1 {}
        .four-2 {}
        [stylesheet-group="9.9"]{}
        .nine-1 {}
        .nine-2 {}
        [stylesheet-group="10"]{}
        .ten {}
        [stylesheet-group="20"]{}
        .twenty {}
        [stylesheet-group="20.2"]{}
        .twenty-point2 {}"
      `);
    });
  });

  describe('#update', () => {
    test('updates an existing rule with same selector', () => {
      const sheet = createOrderedCSSStyleSheet();
      sheet.insert('.selector { color: red }', 0);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        .selector { color: red }"
      `);

      sheet.update('.selector { color: red }', '.selector { color: blue }', 0);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        .selector { color: blue }"
      `);
    });

    test('updates a media query rule', () => {
      const sheet = createOrderedCSSStyleSheet();
      sheet.insert(
        '@media (min-width: 768px) { .responsive { display: flex } }',
        6,
      );
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="6"]{}
        @media (min-width: 768px) { .responsive { display: flex } }"
      `);

      sheet.update(
        '@media (min-width: 768px) { .responsive { display: flex } }',
        '@media (min-width: 1024px) { .responsive { display: grid } }',
        6,
      );
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
"[stylesheet-group="6"]{}
@media (min-width: 1024px) { .responsive { display: grid } }
@media (min-width: 768px) { .responsive { display: flex } }"
`);
    });

    test('updates multiple properties in a rule', () => {
      const sheet = createOrderedCSSStyleSheet();
      sheet.insert('.box { width: 10px; height: 10px; color: red }', 3);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="3"]{}
        .box { width: 10px; height: 10px; color: red }"
      `);

      sheet.update(
        '.box { width: 10px; height: 10px; color: red }',
        '.box { width: 20px; height: 20px; color: blue }',
        3,
      );
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="3"]{}
        .box { width: 20px; height: 20px; color: blue }"
      `);
    });

    test('inserts new rule if old rule does not exist', () => {
      const sheet = createOrderedCSSStyleSheet();
      sheet.insert('.existing { color: red }', 0);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        .existing { color: red }"
      `);

      sheet.update('.nonexistent { color: blue }', '.new { color: green }', 0);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        .existing { color: red }
        .new { color: green }"
      `);
    });

    test('inserts new rule if selectors are different', () => {
      const sheet = createOrderedCSSStyleSheet();
      sheet.insert('.old { color: red }', 0);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        .old { color: red }"
      `);

      sheet.update('.old { color: red }', '.new { color: blue }', 0);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="0"]{}
        .new { color: blue }
        .old { color: red }"
      `);
    });

    test('updates rule in different group', () => {
      const sheet = createOrderedCSSStyleSheet();
      sheet.insert('.one { color: red }', 1);
      sheet.insert('.two { color: green }', 2);
      sheet.insert('.three { color: blue }', 3);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="1"]{}
        .one { color: red }
        [stylesheet-group="2"]{}
        .two { color: green }
        [stylesheet-group="3"]{}
        .three { color: blue }"
      `);

      sheet.update('.two { color: green }', '.two { color: yellow }', 2);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="1"]{}
        .one { color: red }
        [stylesheet-group="2"]{}
        .two { color: yellow }
        [stylesheet-group="3"]{}
        .three { color: blue }"
      `);
    });

    test('updates nested at-rules', () => {
      const sheet = createOrderedCSSStyleSheet();
      sheet.insert(
        '@media (min-width: 768px) { @supports (display: grid) { .nested { display: grid } } }',
        9,
      );
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group="9"]{}
        @media (min-width: 768px) { @supports (display: grid) { .nested { display: grid } } }"
      `);

      sheet.update(
        '@media (min-width: 768px) { @supports (display: grid) { .nested { display: grid } } }',
        '@media (min-width: 1024px) { @supports (display: flex) { .nested { display: flex } } }',
        9,
      );
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
"[stylesheet-group="9"]{}
@media (min-width: 1024px) { @supports (display: flex) { .nested { display: flex } } }
@media (min-width: 768px) { @supports (display: grid) { .nested { display: grid } } }"
`);
    });

    test('handles update when rule has specificity selectors', () => {
      const sheet = createOrderedCSSStyleSheet();
      sheet.insert('.selector:not(#\\#):not(#\\#):not(#\\#) { color: red }', 3);
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
"[stylesheet-group="3"]{}
.selector:not(#\\#):not(#\\#):not(#\\#) { color: red }"
`);

      sheet.update(
        '.selector:not(#\\#):not(#\\#):not(#\\#) { color: red }',
        '.selector:not(#\\#):not(#\\#):not(#\\#) { color: blue }',
        3,
      );
      expect(sheet.getTextContent()).toMatchInlineSnapshot(`
"[stylesheet-group="3"]{}
.selector:not(#\\#):not(#\\#):not(#\\#) { color: blue }"
`);
    });
  });

  describe('client-side hydration', () => {
    let element;

    beforeEach(() => {
      if (element != null) {
        removeStyleElement(element);
      }
      element = insertStyleElement();
    });

    test('from SSR CSS', () => {
      // Setup SSR CSS
      const serverSheet = createOrderedCSSStyleSheet();
      serverSheet.insert('.one { width: 10px; }', 1);
      serverSheet.insert('.two-1 { height: 20px; }', 2);
      serverSheet.insert('.two-2 { color: red; }', 2);
      serverSheet.insert('@keyframes anim { 0% { opacity: 1; } }', 3);
      serverSheet.insert(
        '@media (min-width: 320px) { .four-1 { color: red; } }',
        4,
      );
      serverSheet.insert(
        '@supports (display: grid) { ' +
          '@media (min-width:320px) { ' +
          '.five { color: red; } ' +
          '} ' +
          '}',
        5,
      );
      const textContent = serverSheet.getTextContent();

      // Add SSR CSS to client style sheet
      element.appendChild(document.createTextNode(textContent));
      const clientSheet = createOrderedCSSStyleSheet(element.sheet);
      const hydratedTextContent = clientSheet.getTextContent();
      expect(hydratedTextContent).toMatchInlineSnapshot(`
"[stylesheet-group="1"] {}
.one {width: 10px;}
[stylesheet-group="2"] {}
.two-1 {height: 20px;}
.two-2 {color: red;}
[stylesheet-group="3"] {}
@keyframes anim { 
  0% {opacity: 1;} 
}
[stylesheet-group="4"] {}
@media (min-width: 320px) {.four-1 {color: red;}}
[stylesheet-group="5"] {}
@supports (display: grid) {@media (min-width:320px) {.five {color: red;}}}"
`);

      // Attempt to duplicate part of the hydrated CSS
      clientSheet.insert(
        '@media (min-width: 320px) { ' + '.four-1 { color: red; } ' + '}',
        4,
      );
      expect(clientSheet.getTextContent()).toEqual(hydratedTextContent);

      // Attempt to add similar MediaCSSRule
      clientSheet.insert(
        '@media (min-width: 320px) { ' + '.four-2 { color: blue; } ' + '}',
        4,
      );
      expect(clientSheet.getTextContent()).not.toEqual(hydratedTextContent);
      expect(clientSheet.getTextContent()).toMatchInlineSnapshot(`
"[stylesheet-group="1"] {}
.one {width: 10px;}
[stylesheet-group="2"] {}
.two-1 {height: 20px;}
.two-2 {color: red;}
[stylesheet-group="3"] {}
@keyframes anim { 
  0% {opacity: 1;} 
}
[stylesheet-group="4"] {}
@media (min-width: 320px) { .four-2 { color: blue; } }
@media (min-width: 320px) {.four-1 {color: red;}}
[stylesheet-group="5"] {}
@supports (display: grid) {@media (min-width:320px) {.five {color: red;}}}"
`);
    });

    test('works when the group marker is in single quotes', () => {
      // Setup SSR CSS
      const serverSheet = createOrderedCSSStyleSheet();
      serverSheet.insert('.a { color: red }', 0);
      serverSheet.insert('.b { color: red }', 1);
      const textContent = serverSheet.getTextContent().replace(/"/g, "'");

      // Add SSR CSS to client style sheet
      element.appendChild(document.createTextNode(textContent));
      const clientSheet = createOrderedCSSStyleSheet(element.sheet);
      clientSheet.insert('.c { color: red }', 0);
      expect(clientSheet.getTextContent()).toMatchInlineSnapshot(`
        "[stylesheet-group='0'] {}
        .a {color: red;}
        .c { color: red }
        [stylesheet-group='1'] {}
        .b {color: red;}"
      `);
    });
  });
});
