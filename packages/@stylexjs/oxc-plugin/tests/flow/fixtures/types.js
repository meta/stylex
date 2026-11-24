/**
 * Every type-only Flow construct next to stylex.create and stylex.props.
 * @flow strict
 */
'use strict';
import type {StyleXStyles, StaticStyles} from '@stylexjs/stylex';
import typeof TheStylex from '@stylexjs/stylex';
import * as stylex from '@stylexjs/stylex';
import {type Node, typeof Fragment as F, useState} from 'react';
export type Props = $ReadOnly<{
  xstyle?: StyleXStyles<{color?: string}>,
  size: 'small' | 'large',
  ...
}>;
opaque type Px: number = number;
export opaque type Id = string;
interface Sized { size: number }
declare var __DEV__: boolean;
declare function track(x: mixed): void;
declare class Thing<+T> extends Base<T> { static make(): Thing<T> }
declare module 'some-mod' { declare module.exports: {x: number} }
declare export function exported(x: number): string;
declare export var v: number;
declare export default class D {}
export interface Shape { w: number }
export type {Node};

const styles = stylex.create({
  root: {
    color: 'red',
    padding: (16: Px),
  },
  small: {fontSize: 12},
  large: {fontSize: 20},
});

class Box<T: {+id: string}> implements Sized {
  size: number = 1;
  +label: string;
  declare extra: string;
  static #count: number = 0;
  render(): Node {
    return <div {...stylex.props(styles.root)} />;
  }
}

function pick<T>(xs: $ReadOnlyArray<T>, i?: number = 0): ?T {
  return xs[i];
}

const sizeStyle = (size: Props['size']): StaticStyles<> =>
  size === 'small' ? styles.small : styles.large;

export default function Button({xstyle, size}: Props): Node {
  const [on] = useState<boolean>(false);
  const casted = ((styles.root: any): StaticStyles<>);
  const asCast = styles.small as StaticStyles<>;
  return (
    <button {...stylex.props(styles.root, sizeStyle(size), on && asCast, xstyle)}>
      {pick<string>(['a'])}
    </button>
  );
}

function isString(x: mixed): x is string {
  return typeof x === 'string';
}

function checks(x: mixed): boolean %checks {
  return typeof x === 'number';
}

const arrow = (x: number)
  : number => x + 1;
