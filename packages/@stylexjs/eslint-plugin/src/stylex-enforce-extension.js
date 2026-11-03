/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import type { Node } from 'estree';
import createImportTracker from './utils/createImportTracker';

/*:: import { Rule } from 'eslint'; */

const stylexEnforceExtension = {
  meta: {
    type: 'problem',
    docs: {
      description:
        'Ensure that files exporting `defineVars`, `defineConsts`, or `defineMarker` variables end with a configurable extension, defaulting to `.stylex`, with an option to enforce an additional `.const` suffix for `defineConsts`. Mixed exports are not allowed unless explicitly configured for legacy support.',
      category: 'Possible Errors',
      recommended: false,
    },
    schema: [
      {
        type: 'object',
        properties: {
          validImports: {
            type: 'array',
            items: {
              oneOf: [
                { type: 'string' },
                {
                  type: 'object',
                  properties: {
                    from: { type: 'string' },
                    as: { type: 'string' },
                  },
                },
              ],
            },
            default: ['stylex', '@stylexjs/stylex'],
          },
          themeFileExtension: {
            type: 'string',
            default: '.stylex',
          },
          legacyAllowMixedExports: {
            type: 'boolean',
            default: false,
          },
          enforceDefineConstsExtension: {
            type: 'boolean',
            default: false,
          },
        },
        additionalProperties: false,
      },
    ],
  },
  create(context: Rule.RuleContext): { ... } {
    let hasRestrictedExports = false;
    let hasOtherExports = false;
    let hasDefineConstsExports = false;
    let hasDefineVarsExports = false;
    let hasDefineMarkerExports = false;
    let reportedDefaultExportError = false;
    const fileName = context.filename;
    const options = context.options[0] || {};
    const {
      validImports: importsToLookFor = ['stylex', '@stylexjs/stylex'],
      legacyAllowMixedExports = false,
      enforceDefineConstsExtension = false,
    } = options;

    const themeFileExtension = (
      options.themeFileExtension || '.stylex'
    ).replace(/\.(js|ts|tsx|jsx|mjs|cjs)$/, '');

    const supportedExtensions = ['.js', '.ts', '.tsx', '.jsx', '.mjs', '.cjs'];
    const allThemeExtensions = supportedExtensions.map(
      (ext) => themeFileExtension + ext,
    );
    const allConstExtensions = supportedExtensions.map(
      (ext) => themeFileExtension + '.const' + ext,
    );

    const importTracker = createImportTracker(importsToLookFor);
    const defineVarsVariables = new Set<string>();
    const defineConstsVariables = new Set<string>();
    const defineMarkerVariables = new Set<string>();

    const getDefaultExportErrorMessage = (type: string) =>
      `Default exports are not allowed for variables from \`stylex.${type}()\`. Use named exports instead.`;

    function isDefineVarsExport(node: Node): boolean {
      const callee =
        node.type === 'VariableDeclarator'
          ? (node.init as any)?.callee
          : node.type === 'CallExpression'
            ? node.callee
            : null;

      return (
        (callee?.type === 'MemberExpression' &&
          callee.object?.type === 'Identifier' &&
          importTracker.isStylexDefaultImport(callee.object.name) &&
          callee.property?.type === 'Identifier' &&
          (callee.property.name === 'defineVars' ||
            callee.property.name === 'unstable_defineVarsNested')) ||
        (callee?.type === 'Identifier' &&
          (importTracker.isStylexNamedImport('defineVars', callee.name) ||
            importTracker.isStylexNamedImport(
              'unstable_defineVarsNested',
              callee.name,
            )))
      );
    }

    function isDefineConstsExport(node: Node): boolean {
      const callee =
        node.type === 'VariableDeclarator'
          ? (node.init as any)?.callee
          : node.type === 'CallExpression'
            ? node.callee
            : null;

      return (
        (callee?.type === 'MemberExpression' &&
          callee.object?.type === 'Identifier' &&
          importTracker.isStylexDefaultImport(callee.object.name) &&
          callee.property?.type === 'Identifier' &&
          (callee.property.name === 'defineConsts' ||
            callee.property.name === 'unstable_defineConstsNested')) ||
        (callee?.type === 'Identifier' &&
          (importTracker.isStylexNamedImport('defineConsts', callee.name) ||
            importTracker.isStylexNamedImport(
              'unstable_defineConstsNested',
              callee.name,
            )))
      );
    }

    function isDefineMarkerExport(node: Node): boolean {
      const callee =
        node.type === 'VariableDeclarator'
          ? (node.init as any)?.callee
          : node.type === 'CallExpression'
            ? node.callee
            : null;

      return (
        (callee?.type === 'MemberExpression' &&
          callee.object?.type === 'Identifier' &&
          importTracker.isStylexDefaultImport(callee.object.name) &&
          callee.property?.type === 'Identifier' &&
          callee.property.name === 'defineMarker') ||
        (callee?.type === 'Identifier' &&
          importTracker.isStylexNamedImport('defineMarker', callee.name))
      );
    }

    function checkDefaultExport(node: Node, declaration: Node): boolean {
      if (isDefineConstsExport(declaration)) {
        context.report({
          node,
          message: getDefaultExportErrorMessage('defineConsts'),
        });
        reportedDefaultExportError = true;
        hasDefineConstsExports = true;
        hasRestrictedExports = true;
        return true;
      }
      if (isDefineVarsExport(declaration)) {
        context.report({
          node,
          message: getDefaultExportErrorMessage('defineVars'),
        });
        reportedDefaultExportError = true;
        hasDefineVarsExports = true;
        hasRestrictedExports = true;
        return true;
      }
      if (isDefineMarkerExport(declaration)) {
        context.report({
          node,
          message: getDefaultExportErrorMessage('defineMarker'),
        });
        reportedDefaultExportError = true;
        hasDefineMarkerExports = true;
        hasRestrictedExports = true;
        return true;
      }
      if (declaration.type === 'Identifier') {
        const varName = declaration.name;
        if (defineConstsVariables.has(varName)) {
          context.report({
            node,
            message: getDefaultExportErrorMessage('defineConsts'),
          });
          reportedDefaultExportError = true;
          hasDefineConstsExports = true;
          hasRestrictedExports = true;
          return true;
        }
        if (defineVarsVariables.has(varName)) {
          context.report({
            node,
            message: getDefaultExportErrorMessage('defineVars'),
          });
          reportedDefaultExportError = true;
          hasDefineVarsExports = true;
          hasRestrictedExports = true;
          return true;
        }
        if (defineMarkerVariables.has(varName)) {
          context.report({
            node,
            message: getDefaultExportErrorMessage('defineMarker'),
          });
          reportedDefaultExportError = true;
          hasDefineMarkerExports = true;
          hasRestrictedExports = true;
          return true;
        }
      }
      if (declaration.type === 'VariableDeclaration') {
        const declarations = declaration.declarations || [];
        for (const decl of declarations) {
          if (isDefineConstsExport(decl)) {
            context.report({
              node,
              message: getDefaultExportErrorMessage('defineConsts'),
            });
            reportedDefaultExportError = true;
            hasDefineConstsExports = true;
            hasRestrictedExports = true;
            return true;
          }
          if (isDefineVarsExport(decl)) {
            context.report({
              node,
              message: getDefaultExportErrorMessage('defineVars'),
            });
            reportedDefaultExportError = true;
            hasDefineVarsExports = true;
            hasRestrictedExports = true;
            return true;
          }
          if (isDefineMarkerExport(decl)) {
            context.report({
              node,
              message: getDefaultExportErrorMessage('defineMarker'),
            });
            reportedDefaultExportError = true;
            hasDefineMarkerExports = true;
            hasRestrictedExports = true;
            return true;
          }
        }
      }
      return false;
    }

    function checkExports(node: Node): void {
      const declaration = (node as any).declaration;
      const isDefaultExport = node.type === 'ExportDefaultDeclaration';

      if (!declaration) return;

      if (isDefaultExport && checkDefaultExport(node, declaration)) {
        return;
      }

      const declarations = Array.isArray(declaration.declarations)
        ? declaration.declarations
        : [declaration];

      declarations.forEach((decl: Node) => {
        if (isDefineConstsExport(decl)) {
          hasDefineConstsExports = true;
          if (
            decl.type === 'VariableDeclarator' &&
            decl.id?.type === 'Identifier'
          ) {
            defineConstsVariables.add(decl.id.name);
          }
        } else if (isDefineVarsExport(decl)) {
          hasDefineVarsExports = true;
          if (
            decl.type === 'VariableDeclarator' &&
            decl.id?.type === 'Identifier'
          ) {
            defineVarsVariables.add(decl.id.name);
          }
        } else if (isDefineMarkerExport(decl)) {
          hasDefineMarkerExports = true;
          if (
            decl.type === 'VariableDeclarator' &&
            decl.id?.type === 'Identifier'
          ) {
            defineMarkerVariables.add(decl.id.name);
          }
        } else {
          hasOtherExports = true;
        }
      });
      hasRestrictedExports =
        hasDefineConstsExports ||
        hasDefineVarsExports ||
        hasDefineMarkerExports;
    }

    function reportErrors(node: Node): void {
      const isStylexFile = allThemeExtensions.some((ext) =>
        fileName.endsWith(ext),
      );
      const isConstFile = allConstExtensions.some((ext) =>
        fileName.endsWith(ext),
      );

      const currentExt =
        fileName.match(/\.(js|ts|tsx|jsx|mjs|cjs)$/)?.[0] || '';
      const suggestedExtension = currentExt
        ? themeFileExtension + currentExt
        : themeFileExtension + '.js';
      const suggestedConstExtension = currentExt
        ? themeFileExtension + '.const' + currentExt
        : themeFileExtension + '.const.js';

      if (enforceDefineConstsExtension) {
        if (hasDefineConstsExports && !isConstFile) {
          context.report({
            node,
            message: `Files that export variables from \`stylex.defineConsts()\` must end with a \`${suggestedConstExtension}\` extension.`,
          });
        }

        if (!hasDefineConstsExports && isConstFile) {
          context.report({
            node,
            message: `Only variables from \`stylex.defineConsts()\` can be exported from a file with a \`${suggestedConstExtension}\` extension.`,
          });
        }

        if (
          hasDefineConstsExports &&
          (hasOtherExports || hasDefineVarsExports || hasDefineMarkerExports)
        ) {
          context.report({
            node,
            message:
              'Files that export variables from `stylex.defineConsts()` must not export anything else.',
          });
        }
      }

      const functionsToLint = enforceDefineConstsExtension
        ? '`stylex.defineVars()` or `stylex.defineMarker()`'
        : '`stylex.defineVars()`, `stylex.defineConsts()`, or `stylex.defineMarker()`';

      if (hasRestrictedExports && hasOtherExports && !legacyAllowMixedExports) {
        if (
          !enforceDefineConstsExtension ||
          hasDefineVarsExports ||
          hasDefineMarkerExports
        ) {
          context.report({
            node,
            message: `Files that export variables from ${functionsToLint} must not export anything else.`,
          });
        }
      }

      if (hasRestrictedExports && !isStylexFile) {
        if (
          !enforceDefineConstsExtension ||
          hasDefineVarsExports ||
          hasDefineMarkerExports
        ) {
          context.report({
            node,
            message: `Files that export variables from ${functionsToLint} must end with a \`${suggestedExtension}\` extension.`,
          });
        }
      }

      if (!hasRestrictedExports && isStylexFile) {
        if (
          !enforceDefineConstsExtension ||
          hasDefineVarsExports ||
          hasDefineMarkerExports
        ) {
          context.report({
            node,
            message: `Only variables from ${functionsToLint} can be exported from a file with a \`${suggestedExtension}\` extension.`,
          });
        }
      }
    }

    return {
      ImportDeclaration: importTracker.ImportDeclaration,
      VariableDeclaration(node: Node): void {
        const declarations = (node as any).declarations || [];
        declarations.forEach((decl: Node) => {
          if (isDefineConstsExport(decl)) {
            if (
              decl.type === 'VariableDeclarator' &&
              decl.id?.type === 'Identifier'
            ) {
              defineConstsVariables.add(decl.id.name);
            }
          } else if (isDefineVarsExport(decl)) {
            if (
              decl.type === 'VariableDeclarator' &&
              decl.id?.type === 'Identifier'
            ) {
              defineVarsVariables.add(decl.id.name);
            }
          } else if (isDefineMarkerExport(decl)) {
            if (
              decl.type === 'VariableDeclarator' &&
              decl.id?.type === 'Identifier'
            ) {
              defineMarkerVariables.add(decl.id.name);
            }
          }
        });
      },
      'ExportNamedDeclaration, ExportDefaultDeclaration'(node: Node): void {
        if ((node as any).exportKind === 'type') {
          return;
        }
        reportedDefaultExportError = false;
        checkExports(node);
        if (!reportedDefaultExportError) {
          reportErrors(node);
        }
      },
      'Program:exit'() {
        importTracker.clear();
      },
    };
  },
};

export default stylexEnforceExtension;
