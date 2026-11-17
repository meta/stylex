/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import * as stylex from '@stylexjs/stylex';

export const styles = stylex.create({
  root: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    padding: 8,
    margin: 0,
    borderRadius: 4,
    borderWidth: 1,
    borderStyle: 'solid',
    borderColor: 'gray',
    color: 'black',
    backgroundColor: {
      default: 'white',
      ':hover': '#eee',
    },
  },
  primary: {
    color: 'white',
    borderColor: 'blue',
    backgroundColor: {
      default: 'blue',
      ':hover': 'darkblue',
    },
  },
  large: {
    padding: 16,
    fontSize: 18,
    borderRadius: 8,
  },
  compact: {
    paddingBlock: 4,
    paddingInline: 8,
    gap: 4,
  },
  disabled: {
    opacity: 0.5,
    cursor: 'not-allowed',
  },
  text: {
    fontFamily: 'system-ui',
    lineHeight: 1.4,
    textDecoration: 'none',
    overflow: 'hidden',
  },
  responsive: {
    marginInline: {
      default: 8,
      '@media (min-width: 800px)': 16,
    },
    width: {
      default: '100%',
      '@media (min-width: 800px)': 600,
    },
  },
});

export const dynamic = stylex.create({
  size: (width, height) => ({ width, height }),
  spacing: (padding) => ({ padding }),
});
