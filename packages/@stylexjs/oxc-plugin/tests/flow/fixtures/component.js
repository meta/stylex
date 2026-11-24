// @flow
import * as React from 'react';
import * as stylex from '@stylexjs/stylex';
import type {StyleXStyles} from '@stylexjs/stylex';

function usesCardEarly(): React.Node {
  return <Card title="early" />;
}

export component Card(
  title: string,
  'data-testid' as testId?: string,
  xstyle?: StyleXStyles<>,
  compact: boolean = false,
) renders Empty {
  const [open, setOpen] = useOpen(compact);
  return (
    <div data-testid={testId} {...stylex.props(styles.card, compact && styles.compact, xstyle)}>
      <h2 {...stylex.props(styles.title)}>{title}</h2>
    </div>
  );
}

hook useOpen(initial: boolean): [boolean, (boolean) => void] {
  const [open, setOpen] = React.useState(initial);
  return [open, setOpen];
}

export hook useCardStyle(compact: boolean): {className?: string, style?: {...}} {
  return stylex.props(styles.card, compact && styles.compact);
}

component Empty() renders null {
  return null;
}

component Spread(...props: {label: string}) {
  return <span {...stylex.props(styles.title)}>{props.label}</span>;
}

component Mixed(label: string, ...{icon, size = 12}: {icon: string, size?: number}) {
  return <i {...stylex.props(styles.icon)}>{icon}{label}{size}</i>;
}

export component Input(ref: React.RefSetter<HTMLInputElement>, value: string) {
  return <input ref={ref} value={value} {...stylex.props(styles.input)} />;
}

export default component Page(ref as pageRef: React.RefSetter<HTMLElement>, children: React.Node) {
  return <main ref={pageRef} {...stylex.props(styles.page)}>{children}</main>;
}

function Wrapper(): React.Node {
  component Local(n: number) renders* Card {
    return <Card title={String(n)} />;
  }
  hook useLocal(): number {
    return 1;
  }
  return <Local n={1} />;
}

const styles = stylex.create({
  card: {padding: 8, borderRadius: 4},
  compact: {padding: 2},
  title: {fontWeight: 'bold'},
  icon: {width: 16},
  input: {borderWidth: 1},
  page: {minHeight: '100vh'},
});
