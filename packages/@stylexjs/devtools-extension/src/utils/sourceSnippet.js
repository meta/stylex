/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

type TemplateFrame = {
  inExpression: boolean,
  expressionDepth: number,
};

function findMatchingClosingBraceLine(
  lines: Array<string>,
  startLine: number,
): number | null {
  const startIndex = startLine - 1;
  const firstLine = lines[startIndex] ?? '';
  const propertyOpening = /:\s*{/.exec(firstLine);
  const startColumn =
    propertyOpening == null
      ? firstLine.indexOf('{')
      : firstLine.indexOf('{', propertyOpening.index);
  if (startColumn === -1) return null;

  let depth = 0;
  let inSingleQuote = false;
  let inDoubleQuote = false;
  let inLineComment = false;
  let inBlockComment = false;
  let escapeNext = false;
  const templateStack: Array<TemplateFrame> = [];

  for (let lineIndex = startIndex; lineIndex < lines.length; lineIndex += 1) {
    const text = lines[lineIndex] ?? '';
    inLineComment = false;
    const columnStart = lineIndex === startIndex ? startColumn : 0;

    for (
      let columnIndex = columnStart;
      columnIndex < text.length;
      columnIndex += 1
    ) {
      const character = text[columnIndex];
      const nextCharacter = text[columnIndex + 1];

      if (inLineComment) break;

      if (inBlockComment) {
        if (character === '*' && nextCharacter === '/') {
          inBlockComment = false;
          columnIndex += 1;
        }
        continue;
      }

      if (inSingleQuote || inDoubleQuote) {
        if (escapeNext) {
          escapeNext = false;
          continue;
        }
        if (character === '\\') {
          escapeNext = true;
          continue;
        }
        if (
          (inSingleQuote && character === "'") ||
          (inDoubleQuote && character === '"')
        ) {
          inSingleQuote = false;
          inDoubleQuote = false;
        }
        continue;
      }

      const templateFrame = templateStack.at(-1) ?? null;
      if (templateFrame != null && !templateFrame.inExpression) {
        if (escapeNext) {
          escapeNext = false;
          continue;
        }
        if (character === '\\') {
          escapeNext = true;
          continue;
        }
        if (character === '`') {
          templateStack.pop();
          continue;
        }
        if (character === '$' && nextCharacter === '{') {
          templateFrame.inExpression = true;
          templateFrame.expressionDepth = 1;
          depth += 1;
          columnIndex += 1;
        }
        continue;
      }

      if (character === '/' && nextCharacter === '/') {
        inLineComment = true;
        columnIndex += 1;
        continue;
      }
      if (character === '/' && nextCharacter === '*') {
        inBlockComment = true;
        columnIndex += 1;
        continue;
      }
      if (character === "'") {
        inSingleQuote = true;
        escapeNext = false;
        continue;
      }
      if (character === '"') {
        inDoubleQuote = true;
        escapeNext = false;
        continue;
      }
      if (character === '`') {
        templateStack.push({ inExpression: false, expressionDepth: 0 });
        escapeNext = false;
        continue;
      }

      if (character === '{') {
        depth += 1;
        if (templateFrame != null && templateFrame.inExpression) {
          templateFrame.expressionDepth += 1;
        }
        continue;
      }

      if (character === '}') {
        depth -= 1;
        if (templateFrame != null && templateFrame.inExpression) {
          templateFrame.expressionDepth -= 1;
          if (templateFrame.expressionDepth === 0) {
            templateFrame.inExpression = false;
          }
        }
        if (depth === 0) return lineIndex + 1;
      }
    }
  }

  return null;
}

export function formatSourceSnippet(
  content: string,
  line: number | null,
): string {
  const lines = content.replace(/\r\n/g, '\n').split('\n');
  if (lines.length === 0) return '';

  const target =
    typeof line === 'number' && Number.isFinite(line)
      ? Math.min(Math.max(Math.floor(line), 1), lines.length)
      : 1;
  const end =
    findMatchingClosingBraceLine(lines, target) ??
    Math.min(target + 6, lines.length);
  const width = String(end).length;
  const output = [];

  for (let current = target; current <= end; current += 1) {
    const marker = current === target ? '>' : ' ';
    output.push(
      `${marker} ${String(current).padStart(width, ' ')} | ${
        lines[current - 1] ?? ''
      }`,
    );
  }

  return output.join('\n');
}
