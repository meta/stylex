// @flow
import * as stylex from '@stylexjs/stylex';

const styles = stylex.create({
  on: {color: 'green'},
  off: {color: 'gray'},
});

record Toggle {
  on: boolean = false,
  label: string,
  'data-id': string = 'toggle',
  static styles: typeof styles = styles,
  props(): {className?: string} {
    return stylex.props(this.on ? styles.on : styles.off);
  }
  static make(label: string): Toggle {
    return Toggle {label};
  }
}

export const t: Toggle = Toggle {label: 'x', on: true};
