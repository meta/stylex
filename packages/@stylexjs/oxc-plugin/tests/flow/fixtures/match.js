// @flow
import * as stylex from '@stylexjs/stylex';

type Status = 'idle' | 'busy' | 'error';
type Shape = {kind: 'circle', r: number} | {kind: 'rect', w: number, h: number};

const styles = stylex.create({
  idle: {color: 'gray'},
  busy: {color: 'blue'},
  error: {color: 'red'},
  big: {fontSize: 24},
  small: {fontSize: 12},
  round: {borderRadius: '50%'},
});

export function statusStyle(status: Status): {className?: string} {
  return stylex.props(
    match (status) {
      'idle' => styles.idle,
      'busy' => styles.busy,
      'error' => styles.error,
    },
  );
}

export function shapeStyle(shape: Shape, n: number) {
  const s = match (shape) {
    {kind: 'circle', const r} if (r > 10) => [styles.round, styles.big],
    {kind: 'circle', ...} => styles.round,
    {kind: 'rect', const w, h: 0 | 1} => styles.small,
    {kind: 'rect', ...const rest} => rest.w > 10 ? styles.big : styles.small,
  };
  const size = match (n) {
    0 | 1 => styles.small,
    -1 => styles.error,
    _ if (n > 100) => match (n % 2) { 0 => styles.big, _ => styles.small },
    _ => null,
  };
  return stylex.props(s, size);
}

export function log(value: mixed): void {
  match (value) {
    [const first, ...const others] => {
      console.log(first, others);
    }
    {name: 'a' | 'b' as const nm, ...} => {
      console.log(nm);
    }
    NaN => {
      console.log('nan');
    }
    _ => {}
  }
}

export function withInstance(e: mixed): string {
  return match (e) {
    TypeError {message: const m} => m,
    Error {} => 'error',
    _ => 'other',
  };
}

export function multiline(
  status: Status,
) {
  const cls = match (
    status
  ) {
    'idle' =>
      styles.idle,
    _ =>
      styles.busy,
  };
  return stylex.props(cls);
}
