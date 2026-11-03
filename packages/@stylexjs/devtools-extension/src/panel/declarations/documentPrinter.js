/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

export type Doc =
  | { type: 'text', value: string }
  | { type: 'concat', parts: $ReadOnlyArray<Doc> }
  | { type: 'line', flatValue: string }
  | { type: 'group', value: Doc }
  | { type: 'align', offset: number, value: Doc }
  | { type: 'if-break', broken: Doc, flat: Doc };

type Mode = 'break' | 'flat';
type Command = {
  doc: Doc,
  indent: number,
  mode: Mode,
};

export function text(value: string): Doc {
  return { type: 'text', value };
}

export function concat(parts: $ReadOnlyArray<Doc>): Doc {
  return { type: 'concat', parts };
}

export function line(flatValue: string = ''): Doc {
  return { type: 'line', flatValue };
}

export function group(value: Doc): Doc {
  return { type: 'group', value };
}

export function align(offset: number, value: Doc): Doc {
  return { type: 'align', offset, value };
}

export function ifBreak(broken: Doc, flat: Doc): Doc {
  return { type: 'if-break', broken, flat };
}

function pushParts(
  commands: Array<Command>,
  parts: $ReadOnlyArray<Doc>,
  indent: number,
  mode: Mode,
): void {
  for (let index = parts.length - 1; index >= 0; index -= 1) {
    commands.push({ doc: parts[index], indent, mode });
  }
}

function fits(remainingWidth: number, sourceCommands: Array<Command>): boolean {
  const commands = sourceCommands.slice();
  let remaining = remainingWidth;

  while (remaining >= 0 && commands.length > 0) {
    const command = commands.pop();
    if (command == null) continue;

    const { doc, indent, mode } = command;
    switch (doc.type) {
      case 'text': {
        if (doc.value.includes('\n')) return true;
        remaining -= doc.value.length;
        break;
      }
      case 'concat': {
        pushParts(commands, doc.parts, indent, mode);
        break;
      }
      case 'line': {
        if (mode === 'break') return true;
        remaining -= doc.flatValue.length;
        break;
      }
      case 'group': {
        commands.push({ doc: doc.value, indent, mode: 'flat' });
        break;
      }
      case 'align': {
        commands.push({ doc: doc.value, indent, mode });
        break;
      }
      case 'if-break': {
        commands.push({
          doc: mode === 'break' ? doc.broken : doc.flat,
          indent,
          mode,
        });
        break;
      }
      default:
        throw new Error('Unknown document node.');
    }
  }

  return remaining >= 0;
}

function nextColumn(column: number, value: string): number {
  const lastNewline = value.lastIndexOf('\n');
  return lastNewline === -1
    ? column + value.length
    : value.length - lastNewline - 1;
}

export function printDoc(doc: Doc, maxLineLength: number): string {
  const commands: Array<Command> = [{ doc, indent: 0, mode: 'break' }];
  let column = 0;
  let output = '';

  while (commands.length > 0) {
    const command = commands.pop();
    if (command == null) continue;

    const { doc: current, indent, mode } = command;
    switch (current.type) {
      case 'text': {
        output += current.value;
        column = nextColumn(column, current.value);
        break;
      }
      case 'concat': {
        pushParts(commands, current.parts, indent, mode);
        break;
      }
      case 'line': {
        if (mode === 'flat') {
          output += current.flatValue;
          column += current.flatValue.length;
        } else {
          output += `\n${' '.repeat(indent)}`;
          column = indent;
        }
        break;
      }
      case 'group': {
        const flatCommand: Command = {
          doc: current.value,
          indent,
          mode: 'flat',
        };
        const nextMode =
          mode === 'flat' ||
          fits(maxLineLength - column, [...commands, flatCommand])
            ? 'flat'
            : 'break';
        commands.push({ doc: current.value, indent, mode: nextMode });
        break;
      }
      case 'align': {
        commands.push({
          doc: current.value,
          indent: Math.max(0, column + current.offset),
          mode,
        });
        break;
      }
      case 'if-break': {
        commands.push({
          doc: mode === 'break' ? current.broken : current.flat,
          indent,
          mode,
        });
        break;
      }
      default:
        throw new Error('Unknown document node.');
    }
  }

  return output;
}
