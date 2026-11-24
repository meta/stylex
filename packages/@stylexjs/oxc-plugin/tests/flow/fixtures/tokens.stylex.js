// @flow
import * as stylex from '@stylexjs/stylex';
import type {VarGroup} from '@stylexjs/stylex';

type Tokens = {
  +accent: string,
  +space: string,
};

export const colors: VarGroup<Tokens> = stylex.defineVars({
  accent: 'blue',
  space: '4px',
});

export const consts = stylex.defineConsts({
  radius: '4px',
} as const);
