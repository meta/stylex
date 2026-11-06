// Represents the current phase of the StyleX visitor pipeline for a file.
//
// The pipeline runs in this fixed order:
//   1. `Discover` — gather imports, transform compiled-JSX `sx`, count
//      references, pre-fill declarations.
//   2. `TransformProducers` — transform `stylex.create` / `defineVars` /
//      `keyframes` / etc.
//   3. `TransformConsumers` — transform `stylex.props` / `stylex.attrs` and
//      flush runtime style-injection items.
//   4. `Finalize` — sweep unused declarations and prune surviving style
//      objects to the keys actually used. The mark step that decides what
//      survives runs as a free-standing helper before this phase rather
//      than as its own cycle, so this enum has only four variants.
#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy)]
pub enum TransformationCycle {
  Discover,
  TransformProducers,
  TransformConsumers,
  Finalize,
}
