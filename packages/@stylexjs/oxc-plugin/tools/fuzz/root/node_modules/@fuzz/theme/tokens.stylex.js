import * as stylex from '@stylexjs/stylex';
export const tokens = stylex.defineVars({ color: 'red', bg: 'white', radius: '4px', space: '8px' });
export const cardMarker = stylex.defineMarker();
export const consts = stylex.defineConsts({ sm: '@media (max-width: 600px)', primary: 'red', gap: 8 });
