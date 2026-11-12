/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use client';

import * as stylex from '@stylexjs/stylex';
import { useState } from 'react';
import { Button } from '@stylexjs/shared-ui';
import { tokens } from '@stylexjs/shared-ui/tokens.stylex';

export function Counter() {
  const [count, setCount] = useState(0);

  const handleIncrement = () => setCount((c) => c + 1);

  return (
    <section {...stylex.props(styles.wrapper)}>
      <div>Count: {count}</div>
      <Button onClick={handleIncrement} xstyle={styles.button}>
        Increment
      </Button>
    </section>
  );
}

const styles = stylex.create({
  wrapper: {
    marginBlockStart: '1rem',
    marginInline: '-1rem',
    borderWidth: 1,
    borderStyle: 'dashed',
    borderColor: '#60a5fa',
    backgroundColor: `color-mix(in oklab, ${tokens.primaryColor} 5%, transparent)`,
    borderRadius: 6,
    padding: '1rem',
  },
  button: {
    marginBlockStart: '0.5rem',
    borderWidth: 0,
  },
});
