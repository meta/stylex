/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

export const IMPORT_FILE_PARSING_ERROR = `There was error when attempting to parse the imported file.
Please ensure that the 'babelrc' file is configured to be able to parse this file.
`;

// export const
export const IMPORT_FILE_EVAL_ERROR = `There was an error when attempting to evaluate the imported file.
Please ensure that the imported file is self-contained and does not rely on dynamic behavior.
`;

export const DEFAULT_IMPORT = `Error: Cannot use default imports.

Please define your styles without depending on values imported from other files.

You *may* use named imports to use variables defined with \`defineVars\` in a file with \`.stylex.js\` or \`.stylex.ts\` file.
See: https://stylexjs.com/docs/learn/theming/defining-variables/#rules-when-defining-variables for more information.
`;

export const PATH_WITHOUT_NODE = `Unexpected error:
Could not resolve the code being evaluated.
`;

export const UNEXPECTED_MEMBER_LOOKUP = `Unexpected error:
Could not determine the property being accessed.
`;

export const BLOCKED_PROPERTY_ACCESS = `Access to this property is not allowed during compilation.
Accessing prototype-chain properties such as 'constructor', '__proto__', or 'prototype'
is blocked to prevent arbitrary code execution.
`;

export const BLOCKED_FUNCTION_CALL = `Calling this function is not allowed during compilation.
Functions that compile strings into code, such as 'Function' and 'eval', are blocked to
prevent arbitrary code execution.
`;

export const IMPORT_PATH_RESOLUTION_ERROR = `Could not resolve the path to the imported file.
Please ensure that the theme file has a .stylex.js or .stylex.ts extension and follows the
rules for defining variables:

https://stylexjs.com/docs/learn/theming/defining-variables/#rules-when-defining-variables
`;

export const NON_CONSTANT = 'Referenced value is not a constant.\n\n';

export const USED_BEFORE_DECLARATION =
  'Referenced value is used before declaration.\n\n';

export const UNINITIALIZED_CONST =
  'Referenced constant is not initialized.\n\n';

export const UNDEFINED_CONST = 'Referenced constant is not defined.';

export const UNSUPPORTED_OPERATOR = (op: string): string =>
  `Unsupported operator: ${op}\n\n`;

export const OBJECT_METHOD = 'Unsupported object method.\n\n';

export const UNSUPPORTED_EXPRESSION = (type: string): string =>
  `Unsupported expression: ${type}\n\n`;
