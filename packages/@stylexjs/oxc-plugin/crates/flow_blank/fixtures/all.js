/**
 * @flow strict
 */
import type { A, B } from './types';
import typeof TypeofDefault from './x';
import { type C, typeof D, value, type E } from './mixed';
import Def, { type OnlyType } from './def';
import { type T1, type T2 } from './alltypes';
import * as NS from './ns';
export type { A };
export type Alias<T> = { +x: T, -y?: ?string, ... };
type Local = $ReadOnly<{| a: number |}>;
opaque type Op: string = string;
export opaque type Op2 = number;
interface I { m(): void }
export interface I2 extends I { n: number }
declare var dv: number;
declare function df(x: number): string;
declare class DC { m(): void }
declare module 'mod' { declare module.exports: number; }
declare export function dex(): void;
declare type DT = number;
declare opaque type DOT;

function f<T: Object = {}>(this: Window, x: T, y?: number, ...rest: Array<mixed>): T {
  return (x: any);
}
function g(this: Window) {}
function opt(a?: string, { b }: { b: number } = {}, c?: number = 1) {}
const arrow = <T>(x: T): T => x;
const arrow2 = async (x: number): Promise<void> => {};
const multiline = (x: number)
  : number => x;
const cast = ((value: any): string);
const asExpr = value as string;
const asConst = ['a'] as const;
const call = f<number>(1);
const nw = new Map<string, number>();
const optCall = value?.<number>(1);

class K<+T> extends Base<T> implements I, I2 {
  +covariant: number = 1;
  -contra: string;
  static +sv: number = 2;
  declare dfield: number;
  plain: string;
  #priv: number = 3;
  method<U>(u: U): U { return u; }
  static async *gen(): AsyncGenerator<number, void, void> {}
}

function pred(x: mixed): boolean %checks { return typeof x === 'string'; }
function guard(x: mixed): x is string { return typeof x === 'string'; }

let v: number = 1, w: Array<{ a: number, b: string }> = [];
for (const [k, val]: [string, number] of []) {}
export default (function (x: number): number { return x; });
export const exported: number = 1;
export { value as renamed };
const jsx = <Foo bar={(1: number)} />;
type AfterExpr = number
;(function iife() {})();
import { K1, type K2, type K3, } from './trailing';
import { type K4, K5, type K6 } from './mixed2';
import Dflt, { type K7, type K8, } from './dflt';
const parenAs = (value) as string;
const parenConst = (['b']) as const;
foo((
  { a: 1 }
) as Obj, 2);
class Hazard {
  a: number
  ;[computed]() {}
  b: string
  *gen() {}
  c: number = 1;
}
