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
  CSSProperties,
  CompiledStyles,
  InlineStyles,
  Keyframes,
  MapNamespaces,
  NestedConstsValue,
  NestedStringValue,
  NestedVarsValue,
  StaticStyles,
  StaticStylesWithout,
  StyleX$Create,
  StyleX$CreateTheme,
  StyleX$DefineVars,
  StyleX$DefineConsts,
  StyleXArray,
  StyleXClassNameFor,
  StyleXMarker,
  StyleXStyles,
  StyleXStylesWithout,
  StyleXVar,
  Theme,
  VarGroup,
  PositionTry,
  ViewTransitionClass,
  StyleX$When,
  StyleX$DefaultMarker,
  StyleX$DefineMarker,
  StyleX$Env,
} from './types/StyleXTypes';
import type { ValueWithDefault } from './types/StyleXUtils';
import * as Types from './types/VarTypes';

export type {
  CSSProperties,
  CompiledStyles,
  InlineStyles,
  Keyframes,
  MapNamespaces,
  StaticStyles,
  StaticStylesWithout,
  StyleXArray,
  StyleXClassNameFor,
  StyleXMarker,
  StyleXStyles,
  StyleXStylesWithout,
  StyleXVar,
  Theme,
  Types,
  VarGroup,
  PositionTry,
};

import { styleq } from 'styleq';

const errorForFn = (name: string) =>
  new Error(
    `Unexpected 'stylex.${name}' call at runtime. Styles must be compiled by '@stylexjs/babel-plugin'.`,
  );
const errorForType = (key: keyof typeof types) => errorForFn(`types.${key}`);

export const create: StyleX$Create = function stylexCreate<
  const S extends { readonly [string]: unknown },
>(_styles: S): MapNamespaces<S> {
  throw errorForFn('create');
};

export const createTheme: StyleX$CreateTheme = (_baseTokens, _overrides) => {
  throw errorForFn('createTheme');
};

export const defineConsts: StyleX$DefineConsts = function stylexDefineConsts<
  const T extends { readonly [string]: number | string },
>(_styles: T): T {
  throw errorForFn('defineConsts');
};

export const defineVars: StyleX$DefineVars = function stylexDefineVars(
  _styles: $FlowFixMe,
) {
  throw errorForFn('defineVars');
};

export const unstable_conditional = function stylexConditional<
  const T extends { readonly default: unknown, readonly [string]: unknown },
>(_value: T): T {
  throw errorForFn('unstable_conditional');
};

export const unstable_defineVarsNested = function stylexDefineVarsNested<
  const T extends { readonly [string]: NestedVarsValue },
>(_styles: T): T {
  throw errorForFn('unstable_defineVarsNested');
};

export const unstable_defineConstsNested = function stylexDefineConstsNested<
  const T extends { readonly [string]: NestedConstsValue },
>(_styles: T): T {
  throw errorForFn('unstable_defineConstsNested');
};

export const unstable_createThemeNested = (
  _baseTokens: { readonly [string]: NestedStringValue },
  _overrides: { readonly [string]: NestedVarsValue },
): CompiledStyles => {
  throw errorForFn('unstable_createThemeNested');
};

export const defineMarker: StyleX$DefineMarker = () => {
  throw errorForFn('defineMarker');
};

export const firstThatWorks = <T extends string | number>(
  ..._styles: ReadonlyArray<T>
): ReadonlyArray<T> => {
  throw errorForFn('firstThatWorks');
};

export const keyframes = (_keyframes: Keyframes): string => {
  throw errorForFn('keyframes');
};

export const positionTry = (_positionTry: PositionTry): string => {
  throw errorForFn('positionTry');
};

export function props(
  this: ?unknown,
  ...styles: ReadonlyArray<
    StyleXArray<
      ?CompiledStyles | boolean | Readonly<[CompiledStyles, InlineStyles]>,
    >,
  >
): Readonly<{
  className?: string,
  'data-style-src'?: string,
  style?: Readonly<{ [string]: string | number }>,
}> {
  const [className, style, dataStyleSrc] = styleq(styles);
  const result: {
    className?: string,
    'data-style-src'?: string,
    style?: Readonly<{ [string]: string | number }>,
  } = {};
  if (className != null && className !== '') {
    result.className = className;
  }
  if (style != null && Object.keys(style).length > 0) {
    result.style = style;
  }
  if (dataStyleSrc != null && dataStyleSrc !== '') {
    result['data-style-src'] = dataStyleSrc;
  }
  return result;
}

const toKebabCase = (str: string): string =>
  str.replace(/([A-Z])/g, '-$1').toLowerCase();

export function attrs(
  this: ?unknown,
  ...styles: ReadonlyArray<
    StyleXArray<
      ?CompiledStyles | boolean | Readonly<[CompiledStyles, InlineStyles]>,
    >,
  >
): Readonly<{
  class?: string,
  'data-style-src'?: string,
  style?: string,
}> {
  const {
    className,
    style,
    'data-style-src': dataStyleSrc,
  } = props.apply(this, styles);
  const result: {
    class?: string,
    'data-style-src'?: string,
    style?: string,
  } = {};
  if (className != null) {
    result.class = className;
  }
  if (style != null) {
    result.style = Object.entries(style)
      .map(([key, value]) => {
        if (key.startsWith('--')) {
          return `${key}:${value}`;
        }

        return `${toKebabCase(key)}:${value}`;
      })
      .join(';');
  }
  if (dataStyleSrc != null) {
    result['data-style-src'] = dataStyleSrc;
  }
  return result;
}

export const viewTransitionClass = (
  _viewTransitionClass: ViewTransitionClass,
): string => {
  throw errorForFn('viewTransitionClass');
};

export const defaultMarker: StyleX$DefaultMarker = () => {
  throw errorForFn('defaultMarker');
};

export const when: StyleX$When = {
  ancestor: (_p) => {
    throw errorForFn('when.ancestor');
  },
  descendant: (_p) => {
    throw errorForFn('when.descendant');
  },
  siblingBefore: (_p) => {
    throw errorForFn('when.siblingBefore');
  },
  siblingAfter: (_p) => {
    throw errorForFn('when.siblingAfter');
  },
  anySibling: (_p) => {
    throw errorForFn('when.anySibling');
  },
};

export const env: StyleX$Env = Object.freeze({});

export const types = {
  angle: <T extends string | 0 = string | 0>(
    _v: ValueWithDefault<T>,
  ): Types.Angle<T> => {
    throw errorForType('angle');
  },
  color: <T extends string = string>(
    _v: ValueWithDefault<T>,
  ): Types.Color<T> => {
    throw errorForType('color');
  },
  url: <T extends string = string>(_v: ValueWithDefault<T>): Types.Url<T> => {
    throw errorForType('url');
  },
  image: <T extends string = string>(
    _v: ValueWithDefault<T>,
  ): Types.Image<T> => {
    throw errorForType('image');
  },
  integer: <T extends number = number>(
    _v: ValueWithDefault<T>,
  ): Types.Integer<T> => {
    throw errorForType('integer');
  },
  lengthPercentage: <T extends number | string = number | string>(
    _v: ValueWithDefault<T>,
  ): Types.LengthPercentage<T> => {
    throw errorForType('lengthPercentage');
  },
  length: <T extends number | string = number | string>(
    _v: ValueWithDefault<T>,
  ): Types.Length<T> => {
    throw errorForType('length');
  },
  percentage: <T extends number | string = number | string>(
    _v: ValueWithDefault<T>,
  ): Types.Percentage<T> => {
    throw errorForType('percentage');
  },
  number: <T extends number = number>(
    _v: ValueWithDefault<T>,
  ): Types.Num<T> => {
    throw errorForType('number');
  },
  resolution: <T extends string = string>(
    _v: ValueWithDefault<T>,
  ): Types.Resolution<T> => {
    throw errorForType('resolution');
  },
  time: <T extends string | 0 = string | 0>(
    _v: ValueWithDefault<T>,
  ): Types.Time<T> => {
    throw errorForType('time');
  },
  transformFunction: <T extends string = string>(
    _v: ValueWithDefault<T>,
  ): Types.TransformFunction<T> => {
    throw errorForType('transformFunction');
  },
  transformList: <T extends string = string>(
    _v: ValueWithDefault<T>,
  ): Types.TransformList<T> => {
    throw errorForType('transformList');
  },
};

/**
 * DO NOT USE. Legacy export for Meta
 */

type IStyleX = {
  (...styles: ReadonlyArray<StyleXArray<?CompiledStyles | boolean>>): string,
  create: StyleX$Create,
  createTheme: StyleX$CreateTheme,
  defineConsts: StyleX$DefineConsts,
  defineVars: StyleX$DefineVars,
  env: StyleX$Env,
  defaultMarker: StyleX$DefaultMarker,
  defineMarker: StyleX$DefineMarker,
  firstThatWorks: <T extends string | number>(
    ...v: ReadonlyArray<T>
  ) => ReadonlyArray<T>,
  keyframes: (keyframes: Keyframes) => string,
  positionTry: (positionTry: PositionTry) => string,
  props: (
    this: ?unknown,
    ...styles: ReadonlyArray<
      StyleXArray<
        ?CompiledStyles | boolean | Readonly<[CompiledStyles, InlineStyles]>,
      >,
    >
  ) => Readonly<{
    className?: string,
    'data-style-src'?: string,
    style?: Readonly<{ [string]: string | number }>,
  }>,
  attrs: (
    this: ?unknown,
    ...styles: ReadonlyArray<
      StyleXArray<
        ?CompiledStyles | boolean | Readonly<[CompiledStyles, InlineStyles]>,
      >,
    >
  ) => Readonly<{
    class?: string,
    'data-style-src'?: string,
    style?: string,
  }>,
  viewTransitionClass: (viewTransitionClass: ViewTransitionClass) => string,
  types: typeof types,
  when: typeof when,
  unstable_conditional: <
    const T extends { readonly default: unknown, readonly [string]: unknown },
  >(
    value: T,
  ) => T,
  unstable_defineVarsNested: <
    const T extends { readonly [string]: NestedVarsValue },
  >(
    tokens: T,
  ) => T,
  unstable_defineConstsNested: <
    const T extends { readonly [string]: NestedConstsValue },
  >(
    tokens: T,
  ) => T,
  unstable_createThemeNested: (
    baseTokens: { readonly [string]: NestedStringValue },
    overrides: { readonly [string]: NestedVarsValue },
  ) => CompiledStyles,
  __customProperties?: { [string]: unknown },
  ...
};

export const legacyMerge: IStyleX = /*@__PURE__*/ (function () {
  function _legacyMerge(
    ...styles: ReadonlyArray<StyleXArray<?CompiledStyles | boolean>>
  ): string {
    const [className] = styleq(styles);
    return className;
  }

  _legacyMerge.create = create;
  _legacyMerge.createTheme = createTheme;
  _legacyMerge.defineConsts = defineConsts;
  _legacyMerge.defineMarker = defineMarker;
  _legacyMerge.defineVars = defineVars;
  _legacyMerge.defaultMarker = defaultMarker;
  _legacyMerge.firstThatWorks = firstThatWorks;
  _legacyMerge.keyframes = keyframes;
  _legacyMerge.positionTry = positionTry;
  _legacyMerge.props = props;
  _legacyMerge.attrs = attrs;
  _legacyMerge.types = types;
  _legacyMerge.when = when;
  _legacyMerge.viewTransitionClass = viewTransitionClass;
  _legacyMerge.env = env;
  _legacyMerge.unstable_conditional = unstable_conditional;
  _legacyMerge.unstable_defineVarsNested = unstable_defineVarsNested;
  _legacyMerge.unstable_defineConstsNested = unstable_defineConstsNested;
  _legacyMerge.unstable_createThemeNested = unstable_createThemeNested;
  return _legacyMerge;
})();
