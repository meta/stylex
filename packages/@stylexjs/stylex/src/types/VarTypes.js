/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { ValueWithDefault } from './StyleXUtils';

export type CSSSyntax =
  | '*'
  | '<length>'
  | '<number>'
  | '<percentage>'
  | '<length-percentage>'
  | '<color>'
  | '<image>'
  | '<url>'
  | '<integer>'
  | '<angle>'
  | '<time>'
  | '<resolution>'
  | '<transform-function>'
  | '<custom-ident>'
  | '<transform-list>';

type CSSSyntaxType = CSSSyntax;
type InnerValue = null | string | number;

interface ICSSType<out _T extends InnerValue> {
  readonly value: ValueWithDefault<string>;
  readonly syntax: CSSSyntaxType;
}
declare export class Angle<out T extends InnerValue> implements ICSSType<T> {
  readonly value: ValueWithDefault<string>;
  readonly syntax: CSSSyntaxType;
}
declare export class Color<out T extends InnerValue> implements ICSSType<T> {
  readonly value: ValueWithDefault<string>;
  readonly syntax: CSSSyntaxType;
}
declare export class Url<out T extends InnerValue> implements ICSSType<T> {
  readonly value: ValueWithDefault<string>;
  readonly syntax: CSSSyntaxType;
}
declare export class Image<out T extends InnerValue> implements ICSSType<T> {
  readonly value: ValueWithDefault<string>;
  readonly syntax: CSSSyntaxType;
}
declare export class Integer<out T extends InnerValue> implements ICSSType<T> {
  readonly value: ValueWithDefault<string>;
  readonly syntax: CSSSyntaxType;
}
declare export class LengthPercentage<out T extends InnerValue>
  implements ICSSType<T>
{
  readonly value: ValueWithDefault<string>;
  readonly syntax: CSSSyntaxType;
}
declare export class Length<out T extends InnerValue> implements ICSSType<T> {
  readonly value: ValueWithDefault<string>;
  readonly syntax: CSSSyntaxType;
}
declare export class Percentage<out T extends InnerValue>
  implements ICSSType<T>
{
  readonly value: ValueWithDefault<string>;
  readonly syntax: CSSSyntaxType;
}
declare export class Num<out T extends InnerValue> implements ICSSType<T> {
  readonly value: ValueWithDefault<string>;
  readonly syntax: CSSSyntaxType;
}
declare export class Resolution<out T extends InnerValue>
  implements ICSSType<T>
{
  readonly value: ValueWithDefault<string>;
  readonly syntax: CSSSyntaxType;
}
declare export class Time<out T extends InnerValue> implements ICSSType<T> {
  readonly value: ValueWithDefault<string>;
  readonly syntax: CSSSyntaxType;
}
declare export class TransformFunction<out T extends InnerValue>
  implements ICSSType<T>
{
  readonly value: ValueWithDefault<string>;
  readonly syntax: CSSSyntaxType;
}
declare export class TransformList<out T extends InnerValue>
  implements ICSSType<T>
{
  readonly value: ValueWithDefault<string>;
  readonly syntax: CSSSyntaxType;
}

export type CSSType<out T extends InnerValue> =
  | Angle<T>
  | Color<T>
  | Url<T>
  | Image<T>
  | Integer<T>
  | LengthPercentage<T>
  | Length<T>
  | Percentage<T>
  | Num<T>
  | Resolution<T>
  | Time<T>
  | TransformFunction<T>
  | TransformList<T>;
