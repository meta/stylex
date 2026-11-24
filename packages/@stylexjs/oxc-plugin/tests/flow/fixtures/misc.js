/* @flow */
import * as stylex from '@stylexjs/stylex';

// Comment types stay comments.
function legacy(x /*: number */) /*: string */ {
  return String(x);
}

class Field {
  value: string;
  ['computed']: number = 1;
  *gen(): Iterator<number> {
    yield 1;
  }
}

const généré: string = 'é';
const ref = generic<string>('x');
const call = new Map<string, number>();
const opt = obj?.method<number>(1);

const styles = stylex.create({
  // non-ASCII text before a style changes byte offsets but not lines
  root: {content: '"→"', color: 'red'},
  hover: {color: {default: 'black', ':hover': 'blue'}},
  dyn: (w: number) => ({width: w}),
});

export function View({w}: {w: number}): React$Node {
  return <div {...stylex.props(styles.root, styles.hover, styles.dyn(w))}>{généré}</div>;
}
