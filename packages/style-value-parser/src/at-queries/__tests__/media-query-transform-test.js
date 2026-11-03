/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 *
 */

import { lastMediaQueryWinsTransform } from '../media-query-transform.js';

describe('Media Query Transformer', () => {
  test('basic usage: multiple widths', () => {
    const originalStyles = {
      gridColumn: {
        default: '1 / 2',
        '@media (max-width: 1440px)': '1 / 4',
        '@media (max-width: 1024px)': '1 / 3',
        '@media (max-width: 768px)': '1 / -1',
      },
    };

    const expectedStyles = {
      gridColumn: {
        default: '1 / 2',
        '@media (min-width: 1024.01px) and (max-width: 1440px)': '1 / 4',
        '@media (min-width: 768.01px) and (max-width: 1024px)': '1 / 3',
        '@media (max-width: 768px)': '1 / -1',
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('keeps Chromium from rounding generated max-width boundaries', () => {
    const originalStyles = {
      color: {
        default: 'blue',
        '@media (min-width: 400px)': 'red',
        '@media (min-width: 600px)': 'green',
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);

    expect(result).toEqual({
      color: {
        default: 'blue',
        '@media (min-width: 400px) and (max-width: 599.98px)': 'red',
        '@media (min-width: 600px)': 'green',
      },
    });
  });

  test('basic usage: nested query', () => {
    const originalStyles = {
      gridColumn: {
        default: '1 / 2',
        '@media (max-width: 1440px)': {
          '@media (max-height: 1024px)': '1 / 3',
          '@media (max-height: 768px)': '1 / -1',
        },
        '@media (max-width: 1024px)': '1 / 3',
        '@media (max-width: 768px)': '1 / -1',
      },
    };

    const expectedStyles = {
      gridColumn: {
        default: '1 / 2',
        '@media (min-width: 1024.01px) and (max-width: 1440px)': {
          '@media (min-height: 768.01px) and (max-height: 1024px)': '1 / 3',
          '@media (max-height: 768px)': '1 / -1',
        },
        '@media (min-width: 768.01px) and (max-width: 1024px)': '1 / 3',
        '@media (max-width: 768px)': '1 / -1',
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('basic usage: nested query', () => {
    const originalStyles = {
      gridColumn: {
        default: '1 / 2',
        '@media (max-width: 1440px)': {
          '@media (max-width: 1024px)': '1 / 3',
          '@media (max-width: 768px)': '1 / -1',
        },
        '@media (max-width: 1024px)': '1 / 3',
        '@media (max-width: 768px)': '1 / -1',
      },
      padding: '10px',
    };

    const expectedStyles = {
      gridColumn: {
        default: '1 / 2',
        '@media (min-width: 1024.01px) and (max-width: 1440px)': {
          '@media (min-width: 768.01px) and (max-width: 1024px)': '1 / 3',
          '@media (max-width: 768px)': '1 / -1',
        },
        '@media (min-width: 768.01px) and (max-width: 1024px)': '1 / 3',
        '@media (max-width: 768px)': '1 / -1',
      },
      padding: '10px',
    };

    const result = lastMediaQueryWinsTransform(originalStyles);

    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('basic usage: complex object', () => {
    const originalStyles = {
      gridColumn: {
        default: '1 / 2',
        '@media (max-width: 1440px)': '1 / 4',
        '@media (max-width: 1024px)': '1 / 3',
        '@media (max-width: 768px)': '1 / -1',
      },
      grid: {
        default: '1 / 2',
        '@media (max-width: 1440px)': '1 / 4',
      },
      gridRow: {
        default: '1 / 2',
        padding: '10px',
      },
    };

    const expectedStyles = {
      gridColumn: {
        default: '1 / 2',
        '@media (min-width: 1024.01px) and (max-width: 1440px)': '1 / 4',
        '@media (min-width: 768.01px) and (max-width: 1024px)': '1 / 3',
        '@media (max-width: 768px)': '1 / -1',
      },
      grid: {
        default: '1 / 2',
        '@media (max-width: 1440px)': '1 / 4',
      },
      gridRow: {
        default: '1 / 2',
        padding: '10px',
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('basic usage: lots and lots of max-widths', () => {
    const originalStyles = {
      gridColumn: {
        default: '1 / 2',
        '@media (max-width: 1440px)': '1 / 4',
        '@media (max-width: 1024px)': '1 / 3',
        '@media (max-width: 768px)': '1 / -1',
        '@media (max-width: 458px)': '1 / -1',
      },
    };

    const expectedStyles = {
      gridColumn: {
        default: '1 / 2',
        '@media (min-width: 1024.01px) and (max-width: 1440px)': '1 / 4',
        '@media (min-width: 768.01px) and (max-width: 1024px)': '1 / 3',
        '@media (min-width: 458.01px) and (max-width: 768px)': '1 / -1',
        '@media (max-width: 458px)': '1 / -1',
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('basic usage: lots and lots of min-widths', () => {
    const originalStyles = {
      gridColumn: {
        default: '1 / 2',
        '@media (min-width: 768px)': '1 / -1',
        '@media (min-width: 1024px)': '1 / 3',
        '@media (min-width: 1440px)': '1 / 4',
      },
    };

    const expectedStyles = {
      gridColumn: {
        default: '1 / 2',
        '@media (min-width: 768px) and (max-width: 1023.98px)': '1 / -1',
        '@media (min-width: 1024px) and (max-width: 1439.98px)': '1 / 3',
        '@media (min-width: 1440px)': '1 / 4',
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('basic usage: multiple heights', () => {
    const originalStyles = {
      foo: {
        gridRow: {
          default: '1 / 2',
          '@media (max-height: 1200px)': '1 / 4',
          '@media (max-height: 900px)': '1 / 3',
          '@media (max-height: 600px)': '1 / -1',
        },
      },
    };

    const expectedStyles = {
      foo: {
        gridRow: {
          default: '1 / 2',
          '@media (min-height: 900.01px) and (max-height: 1200px)': '1 / 4',
          '@media (min-height: 600.01px) and (max-height: 900px)': '1 / 3',
          '@media (max-height: 600px)': '1 / -1',
        },
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('basic usage: min/max heights', () => {
    const originalStyles = {
      foo: {
        gridRow: {
          default: '1 / 2',
          '@media (min-height: 1200px) and (max-height: 1400px)': '1 / 4',
          '@media (max-height: 900px)': '1 / 3',
          '@media (max-height: 600px)': '1 / -1',
        },
      },
    };

    const expectedStyles = {
      foo: {
        gridRow: {
          default: '1 / 2',
          '@media (min-height: 1200px) and (max-height: 1400px)': '1 / 4',
          '@media (min-height: 600.01px) and (max-height: 900px)': '1 / 3',
          '@media (max-height: 600px)': '1 / -1',
        },
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('single word condition', () => {
    const originalStyles = {
      mode: {
        default: 'normal',
        '@media (color)': 'colorful',
        '@media (monochrome)': 'grayscale',
      },
    };

    const expectedStyles = {
      mode: {
        default: 'normal',
        '@media (color) and (not (monochrome))': 'colorful',
        '@media (monochrome)': 'grayscale',
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('handles only screen media queries without parenthesizing the media type', () => {
    const originalStyles = {
      color: {
        default: null,
        '@media only screen and (max-width: 600px)': 'red',
        '@media only screen and (max-width: 400px)': 'blue',
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    const resultStr = JSON.stringify(result);
    expect(resultStr).not.toContain('only (screen)');
    expect(resultStr).toContain('only screen');
  });

  test('handles comma-separated (or) media queries', () => {
    const originalStyles = {
      width: {
        default: '100%',
        '@media screen, (max-width: 800px)': '80%',
        '@media (max-width: 500px)': '60%',
      },
    };

    const expectedStyles = {
      width: {
        default: '100%',
        '@media screen and (not (max-width: 500px)), (min-width: 500.01px) and (max-width: 800px)':
          '80%',
        '@media (max-width: 500px)': '60%',
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test.skip('handles and media queries', () => {
    const originalStyles = {
      width: {
        default: '100%',
        '@media (min-width: 900px)': '80%',
        '@media (min-width: 500px) and (max-width: 899px) and (max-height: 300px)':
          '50%',
      },
    };

    const expectedStyles = {
      width: {
        default: '100%',
        '@media (min-width: 900px) and (not ((min-width: 500px) and (max-width: 899px) and (max-height: 300px)))':
          '80%',
        '@media (min-width: 500px) and (max-width: 899px) and (max-height: 300px)':
          '50%',
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test.skip('combination of keywords and rules', () => {
    const originalStyles = {
      width: {
        default: '100%',
        '@media screen and (min-width: 900px)': '80%',
        '@media print and (max-width: 500px)': '50%',
      },
    };

    const expectedStyles = {
      width: {
        default: '100%',
        '@media screen and (min-width: 900px) and (not (print and (max-width: 500px)))':
          '80%',
        '@media print and (max-width: 500px)': '50%',
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('mixed responsive and print values anchor to a screen media type', () => {
    // Regression test for https://github.com/facebook/stylex/issues/1860.
    // A `print` media type must not be emitted as a `(not (print))` boolean
    // feature (which never matches in browsers); the responsive branch is
    // instead anchored to the complementary `screen` media type.
    const originalStyles = {
      marginLeft: {
        default: 224,
        '@media (max-width: 900px)': {
          default: 168,
          '@media (max-width: 620px)': 132,
        },
        '@media print': 0,
      },
    };

    const expectedStyles = {
      marginLeft: {
        default: 224,
        '@media screen and (max-width: 900px)': {
          default: 168,
          '@media (max-width: 620px)': 132,
        },
        '@media print': 0,
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('basic usage: does not modify single queries', () => {
    const originalStyles = {
      gridColumn: {
        default: '1 / 2',
        '@media (max-width: 1440px)': '1 / 4',
      },
    };

    const expectedStyles = {
      gridColumn: {
        default: '1 / 2',
        '@media (max-width: 1440px)': '1 / 4',
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('ignores legacy media query syntax', () => {
    const originalStyles = {
      width: '100%',
      '@media (min-width: 600px)': {
        width: '50%',
      },
    };

    const expectedStyles = {
      width: '100%',
      '@media (min-width: 600px)': {
        width: '50%',
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('mixed min/max width and height', () => {
    const originalStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (max-width: 1440px) and (max-height: 900px)': '1 / 4',
          '@media (max-width: 1024px)': '1 / 3',
          '@media (max-width: 768px)': '1 / -1',
        },
      },
    };

    const expectedStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (min-width: 1024.01px) and (max-width: 1440px) and (max-height: 900px)':
            '1 / 4',
          '@media (min-width: 768.01px) and (max-width: 1024px)': '1 / 3',
          '@media (max-width: 768px)': '1 / -1',
        },
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('mixed min/max width and height with only height changing', () => {
    const originalStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (max-width: 1440px) and (max-height: 900px)': '1 / 4',
          '@media (max-height: 700px)': '1 / 3',
          '@media (max-height: 500px)': '1 / -1',
        },
      },
    };

    const expectedStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (max-width: 1440px) and (min-height: 700.01px) and (max-height: 900px)':
            '1 / 4',
          '@media (min-height: 500.01px) and (max-height: 700px)': '1 / 3',
          '@media (max-height: 500px)': '1 / -1',
        },
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('mixed min/max width with disjoint ranges', () => {
    const originalStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (max-width: 1440px) and (min-width: 900px)': '1 / 4',
          '@media (max-width: 800px) and (min-width: 600px)': '1 / 3',
        },
      },
    };

    const expectedStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (min-width: 900px) and (max-width: 1440px)': '1 / 4',
          '@media (min-width: 600px) and (max-width: 800px)': '1 / 3',
        },
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('mixed min/max width with many disjoint ranges', () => {
    const originalStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (max-width: 1440px) and (min-width: 900px)': '1 / 4',
          '@media (max-width: 800px) and (min-width: 600px)': '1 / 3',
          '@media (max-width: 500px) and (min-width: 400px)': '1 / 1',
        },
      },
    };

    const expectedStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (min-width: 900px) and (max-width: 1440px)': '1 / 4',
          '@media (min-width: 600px) and (max-width: 800px)': '1 / 3',
          '@media (min-width: 400px) and (max-width: 500px)': '1 / 1',
        },
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('mixed min/max width with overlapping ranges', () => {
    const originalStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (max-width: 1440px) and (min-width: 900px)': '1 / 4',
          '@media (max-width: 1040px) and (min-width: 600px)': '1 / 3',
        },
      },
    };

    const expectedStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (min-width: 1040.01px) and (max-width: 1440px)': '1 / 4',
          '@media (min-width: 600px) and (max-width: 1040px)': '1 / 3',
        },
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('mixed min/max width with mixed ranges', () => {
    const originalStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (max-width: 1440px) and (min-width: 900px)': '1 / 4',
          '@media (max-width: 1100px) and (min-width: 1000px)': '1 / 3',
          '@media (max-width: 500px) and (min-width: 400px)': '1 / 1',
        },
      },
    };

    const expectedStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media ((min-width: 900px) and (max-width: 999.98px)) or ((min-width: 1100.01px) and (max-width: 1440px))':
            '1 / 4',
          '@media (min-width: 1000px) and (max-width: 1100px)': '1 / 3',
          '@media (min-width: 400px) and (max-width: 500px)': '1 / 1',
        },
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('mixed min/max width with intersecting ranges', () => {
    const originalStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (max-width: 1440px) and (min-width: 900px)': '1 / 4',
          '@media (max-width: 1100px) and (min-width: 1000px)': '1 / 3',
        },
      },
    };

    const expectedStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media ((min-width: 900px) and (max-width: 999.98px)) or ((min-width: 1100.01px) and (max-width: 1440px))':
            '1 / 4',
          '@media (min-width: 1000px) and (max-width: 1100px)': '1 / 3',
        },
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('mixed min/max width with many intersecting ranges', () => {
    const originalStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (max-width: 1440px) and (min-width: 900px)': '1 / 4',
          '@media (max-width: 1100px) and (min-width: 1000px)': '1 / 3',
          '@media (max-width: 1050px) and (min-width: 1010px)': '1 / -1',
        },
      },
    };

    const expectedStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media ((min-width: 900px) and (max-width: 999.98px)) or ((min-width: 1100.01px) and (max-width: 1440px))':
            '1 / 4',
          '@media ((min-width: 1000px) and (max-width: 1009.98px)) or ((min-width: 1050.01px) and (max-width: 1100px))':
            '1 / 3',
          '@media (min-width: 1010px) and (max-width: 1050px)': '1 / -1',
        },
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('media queries with em units', () => {
    const originalStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (max-width: 90em) and (min-width: 60em)': '1 / 4',
          '@media (max-width: 70em) and (min-width: 65em)': '1 / 3',
        },
      },
    };

    const expectedStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media ((min-width: 60em) and (max-width: 64.99em)) or ((min-width: 70.01em) and (max-width: 90em))':
            '1 / 4',
          '@media (min-width: 65em) and (max-width: 70em)': '1 / 3',
        },
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('media queries with mixed units', () => {
    const originalStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (max-width: 1200px) and (min-height: 50vh)': '1 / 4',
          '@media (max-width: 800px) and (min-height: 30vh)': '1 / 3',
        },
      },
    };

    const expectedStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (min-width: 800.01px) and (max-width: 1200px) and (min-height: 50vh)':
            '1 / 4',
          '@media (max-width: 800px) and (min-height: 30vh)': '1 / 3',
        },
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('skips range simplification for media queries with conflicting units in same dimension across queries', () => {
    const originalStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (min-width: 768px) and (max-width: 1200px)': '1 / 4',
          '@media (min-width: 50em)': '1 / 3',
        },
      },
    };

    const expectedStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (min-width: 768px) and (max-width: 1200px) and (not (min-width: 50em))':
            '1 / 4',
          '@media (min-width: 50em)': '1 / 3',
        },
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });

  test('skips range simplification for media queries with conflicting units in same dimension and query', () => {
    const originalStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (min-width: 768px) and (max-width: 1200em)': '1 / 4',
          '@media (min-width: 50em)': '1 / 3',
        },
      },
    };

    const expectedStyles = {
      foo: {
        gridColumn: {
          default: '1 / 2',
          '@media (min-width: 768px) and (max-width: 1200em) and (not (min-width: 50em))':
            '1 / 4',
          '@media (min-width: 50em)': '1 / 3',
        },
      },
    };

    const result = lastMediaQueryWinsTransform(originalStyles);
    expect(JSON.stringify(result)).toBe(JSON.stringify(expectedStyles));
  });
});
