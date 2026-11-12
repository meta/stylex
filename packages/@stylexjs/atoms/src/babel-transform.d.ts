/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

export interface CompileUtils {
  styleXCreateSet: (input: unknown, options: unknown) => [unknown, unknown];
  convertObjectToAST: (obj: unknown) => unknown;
  hoistExpression: (path: unknown, expr: unknown) => unknown;
  injectDevClassNames: (
    styles: unknown,
    name: unknown,
    state: unknown,
  ) => unknown;
}

export declare function createUtilityStylesVisitor(
  state: unknown,
  compile: CompileUtils,
): {
  MemberExpression(path: unknown): void;
  CallExpression(path: unknown): void;
};

export declare function isUtilityStylesIdentifier(
  ident: unknown,
  state: unknown,
  path: unknown,
): boolean;

export declare function getStaticStyleFromPath(
  path: unknown,
  state: unknown,
): { property: string; value: string | number } | null;

export declare function getDynamicStyleFromPath(
  path: unknown,
  state: unknown,
): { property: string; value: unknown } | null;
