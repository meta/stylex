# tools

- `fuzz/run.cjs` is a differential fuzzer. It compiles random StyleX programs with the Babel
  plugin (`packages/@stylexjs/babel-plugin/lib`) and with a candidate, and buckets the
  differences. Run `node tools/fuzz/run.cjs --candidate js/index.js --count 5000 --seed S`.
  `--case i` reruns one program and prints both outputs. The generator is `fuzz/gen.cjs`, the
  comparison is `fuzz/compare.cjs` and the minimizer is `fuzz/minimize.cjs`.
- `fuzz/files.cjs` runs the repo's own StyleX sources (docs, shared-ui, typescript-tests,
  atoms, examples) through both compilers. `fuzz/reduce-file.cjs` shrinks a file that differs.
- `perf/run.cjs` times each file of the benchmark fixtures with this package and with the Babel
  plugin. It pins each compiler with taskset and waits for a quiet machine.
- `perf/pgo-train.cjs` is the training run for `scripts/build-pgo.cjs`.
