/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import { readFileSync } from 'fs';
import { join } from 'path';
import { ScrollableCodeBlock } from './ScrollableCodeBlock';

function getFileContent(filename: string): string {
  const filePath = join(process.cwd(), 'static', 'llm', filename);
  return readFileSync(filePath, 'utf-8');
}

export function LLMInstallationFile() {
  const content = getFileContent('stylex-installation.md');
  return (
    <ScrollableCodeBlock
      content={content}
      maxHeight={400}
      title="stylex-installation.md"
    />
  );
}

export function LLMStylingFile() {
  const content = getFileContent('stylex-authoring.md');
  return (
    <ScrollableCodeBlock
      content={content}
      maxHeight={400}
      title="stylex-authoring.md"
    />
  );
}

export function LLMMigrationFile() {
  const content = getFileContent('stylex-migration-css-modules.md');
  return (
    <ScrollableCodeBlock
      content={content}
      maxHeight={400}
      title="stylex-migration-css-modules.md"
    />
  );
}
