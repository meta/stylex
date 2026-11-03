/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import * as React from 'react';
import { useCallback, useEffect, useRef, useState } from 'react';
import type { StylexDebugData } from '../../types.js';
import { copyText } from '../../utils/clipboard.js';
import { exportMetadata } from '../../utils/exportMetadata.js';
import { Button } from './Button';

export function CopyMetadataButton({
  data,
}: {
  data: StylexDebugData,
}): React.Node {
  const [label, setLabel] = useState('Copy metadata');
  const timeoutRef = useRef<?TimeoutID>(null);

  useEffect(
    () => () => {
      if (timeoutRef.current != null) {
        clearTimeout(timeoutRef.current);
      }
    },
    [],
  );

  const handleClick = useCallback(async () => {
    if (timeoutRef.current != null) {
      clearTimeout(timeoutRef.current);
    }
    const ok = await copyText(exportMetadata(data));
    setLabel(ok ? 'Copied!' : 'Copy failed');
    timeoutRef.current = setTimeout(() => {
      setLabel('Copy metadata');
      timeoutRef.current = null;
    }, 1500);
  }, [data]);

  return (
    <Button
      onClick={handleClick}
      title="Copy this element's StyleX metadata as markdown"
    >
      {label}
    </Button>
  );
}
