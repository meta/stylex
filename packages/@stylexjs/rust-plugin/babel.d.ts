import type { PluginObj } from '@babel/core';
import type { CompilerOptions, ProcessOptions, Rule } from './index';

declare function styleXTransform(): PluginObj;
declare namespace styleXTransform {
  function withOptions(
    options: CompilerOptions,
  ): [typeof styleXTransform, CompilerOptions];
  function processStylexRules(
    rules: readonly Rule[],
    options?: ProcessOptions | boolean,
  ): string;
}
export = styleXTransform;
