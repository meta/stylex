// @flow
// Babel's scope counts names in type positions as references. A style object named in a type
// keeps all its styles; one named only as an object type key loses the unused ones.
import * as stylex from '@stylexjs/stylex';

const viaTypeof = stylex.create({used: {color: 'red'}, unused: {color: 'blue'}});
const viaParam = stylex.create({used: {color: 'green'}, unused: {color: 'gray'}});
const viaIndexer = stylex.create({used: {margin: 1}, unused: {margin: 2}});
const viaGeneric = stylex.create({used: {padding: 1}, unused: {padding: 2}});
const viaKeyOnly = stylex.create({used: {width: 1}, unused: {width: 2}});

type Keys = $Keys<typeof viaTypeof>;
type Render = (...viaParam: $ReadOnlyArray<mixed>) => void;
type Dict = {[viaIndexer: string]: number};
function take(x: viaGeneric) {}
type Shape = {viaKeyOnly: number};

export const props = stylex.props(
  viaTypeof.used,
  viaParam.used,
  viaIndexer.used,
  viaGeneric.used,
  viaKeyOnly.used,
);
