// @flow
import * as stylex from '@stylexjs/stylex';

enum Size {
  Small,
  Medium,
  Large,
}
export enum Tone of string {
  Info = 'info',
  Warn = 'warn',
}
enum Code of number {One = 1, Two = 2}
enum Flag of boolean {On = true, Off = false}
enum Open {A, B, ...}

const styles = stylex.create({
  small: {fontSize: 12},
  medium: {fontSize: 16},
  large: {fontSize: 20},
  info: {color: 'blue'},
  warn: {color: 'orange'},
});

export function sizeStyle(size: Size) {
  return stylex.props(
    match (size) {
      Size.Small => styles.small,
      Size.Medium => styles.medium,
      Size.Large => styles.large,
    },
  );
}

export function toneStyle(tone: Tone) {
  switch (tone) {
    case Tone.Info:
      return stylex.props(styles.info);
    case Tone.Warn:
      return stylex.props(styles.warn);
  }
}

export const all = [Size.Small, Code.One, Flag.On, Open.A];
