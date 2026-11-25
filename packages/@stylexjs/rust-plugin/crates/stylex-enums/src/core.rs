// The phase of the StyleX visitor pipeline, in order: `Discover`, `TransformProducers`
// (`create`, `defineVars`, …), `TransformConsumers` (`props`, `attrs`), `Finalize` (sweep unused
// declarations and prune style objects).
#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy)]
pub enum TransformationCycle {
  Discover,
  TransformProducers,
  TransformConsumers,
  Finalize,
}
