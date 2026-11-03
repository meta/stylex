/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import type {
  StylexDebugData,
  StylexDeclaration,
  StylexOverride,
} from '../types.js';
import { formatCssValue } from './css.js';

// Serializes the StyleX inspection of the selected element into compact
// markdown. The value here is the StyleX-specific metadata that is hard to
// reconstruct from a screenshot: the resolved declarations, the condition
// under which each applies, the atomic class that produced it, and the
// source file:line that authored it.

function conditionLabel(decl: StylexDeclaration): string {
  if (decl.conditions.length > 0) {
    return decl.conditions.map(({ text }) => text).join(', ');
  }
  return 'default';
}

function renderSources(data: StylexDebugData): Array<string> {
  if (data.sources.length === 0) return [];
  const lines = ['## Sources'];
  for (const source of data.sources) {
    lines.push(
      `- ${source.line != null ? `${source.file}:${source.line}` : source.file}`,
    );
  }
  return lines;
}

function renderMatchedStyles(data: StylexDebugData): Array<string> {
  // section (pseudo-element, '' for the base element) -> property -> entries
  const sections: Map<
    string,
    Map<string, Array<StylexDeclaration>>,
  > = new Map();
  for (const cls of data.matched.classes) {
    for (const decl of cls.declarations) {
      const sectionKey = decl.pseudoElement ?? '';
      let properties = sections.get(sectionKey);
      if (properties == null) {
        properties = new Map();
        sections.set(sectionKey, properties);
      }
      const bucket = properties.get(decl.property);
      if (bucket == null) {
        properties.set(decl.property, [decl]);
      } else {
        bucket.push(decl);
      }
    }
  }
  if (sections.size === 0) return [];

  // Render the base element first, then any pseudo-element sections.
  const orderedKeys = [
    ...(sections.has('') ? [''] : []),
    ...[...sections.keys()].filter((key) => key !== ''),
  ];

  const lines = ['## Matched styles'];
  for (const sectionKey of orderedKeys) {
    const properties = sections.get(sectionKey);
    if (properties == null) continue;
    if (sectionKey !== '') {
      lines.push(`### ${sectionKey}`);
    }
    for (const [property, entries] of properties) {
      if (entries.length === 1) {
        const entry = entries[0];
        const cond = conditionLabel(entry);
        const condText = cond === 'default' ? '' : ` [${cond}]`;
        lines.push(
          `- ${property}: ${formatCssValue(entry.value, entry.important)}${condText} (.${entry.className})`,
        );
        continue;
      }
      lines.push(`- ${property}:`);
      for (const entry of entries) {
        lines.push(
          `  - ${conditionLabel(entry)}: ${formatCssValue(entry.value, entry.important)} (.${entry.className})`,
        );
      }
    }
  }
  return lines;
}

function renderOverrides(overrides: Array<StylexOverride>): Array<string> {
  if (overrides.length === 0) return [];
  const lines = ['## Overrides (live, not yet in source)'];
  for (const override of overrides) {
    lines.push(
      `- ${override.property}: ${formatCssValue(override.value, override.important)} (${override.kind})`,
    );
  }
  return lines;
}

export function exportMetadata(data: StylexDebugData): string {
  const blocks: Array<Array<string>> = [
    [`# StyleX metadata — <${data.element.tagName}>`],
    renderSources(data),
    renderMatchedStyles(data),
    renderOverrides(data.overrides),
  ];
  return blocks
    .filter((block) => block.length > 0)
    .map((block) => block.join('\n'))
    .join('\n\n');
}
