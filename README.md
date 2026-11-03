# StyleX &middot; [![GitHub license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/facebook/stylex/blob/main/LICENSE) [![npm version](https://img.shields.io/npm/v/@stylexjs/stylex.svg?style=flat)](https://www.npmjs.com/package/@stylexjs/stylex) [![tests](https://github.com/facebook/stylex/actions/workflows/tests.yml/badge.svg)](https://github.com/facebook/stylex/actions/workflows/tests.yml) [![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/facebook/stylex/blob/main/.github/CONTRIBUTING.md)

StyleX is a JavaScript library for defining styles for optimized user
interfaces.

## Documentation

[Documentation Website](https://stylexjs.com)

Documentation for individual packages can be found in their respective README
files. Start with
[`@stylexjs/stylex`](https://github.com/facebook/stylex/blob/main/packages/@stylexjs/stylex).

### Example

Here is a simple example of StyleX use:

```js
import * as stylex from '@stylexjs/stylex';

const styles = stylex.create({
  root: {
    padding: 10,
  },
  element: {
    backgroundColor: 'red',
  },
});

const styleProps = stylex.props(styles.root, styles.element);
```

## Development

This is the development monorepo for StyleX.

### Structure

- `.github`
  - Contains workflows used by GitHub Actions.
  - Contains issue templates and contribution guidelines.
- `examples`
  - Contains examples using StyleX and its integration with build tools.
- `packages`
  - Contains the public and private packages managed in the monorepo.
  - [babel-plugin](https://github.com/facebook/stylex/blob/main/packages/@stylexjs/babel-plugin)
  - [benchmarks](https://github.com/facebook/stylex/blob/main/packages/benchmarks)
  - [cli](https://github.com/facebook/stylex/blob/main/packages/@stylexjs/cli)
  - [docs](https://github.com/facebook/stylex/blob/main/packages/docs)
  - [eslint-plugin](https://github.com/facebook/stylex/blob/main/packages/@stylexjs/eslint-plugin)
  - [postcss-plugin](https://github.com/facebook/stylex/blob/main/packages/@stylexjs/postcss-plugin)
  - [rollup-plugin](https://github.com/facebook/stylex/blob/main/packages/@stylexjs/rollup-plugin)
  - [scripts](https://github.com/facebook/stylex/blob/main/packages/scripts)
  - [shared](https://github.com/facebook/stylex/tree/main/packages/@stylexjs/shared)
  - [style-value-parser](https://github.com/facebook/stylex/blob/main/packages/style-value-parser)
  - [stylex](https://github.com/facebook/stylex/blob/main/packages/@stylexjs/stylex)


### Tasks

First, `yarn install` the yarn workspace.

- `build`
  - Use `yarn build` to run the build script in every package.
  - Use `yarn workspace <package-name> build` to run the build script for a
    specific package.
- `test`
  - Use `yarn test` to run tests for every package.
  - Use `yarn workspace <package-name> test` to run the test script for a
    specific package. More details can be found in the contributing guide below.

## Contributing

Development happens in the open on GitHub and we are grateful for contributions
including bug fixes, improvements, and ideas.

### Code of Conduct

This project expects all participants to adhere to Meta's OSS
[Code of Conduct](https://opensource.fb.com/code-of-conduct/). Please read
the full text so that you can understand what actions will and will not be
tolerated.

### Contributing Guide

Read the
[contributing guide](https://github.com/facebook/stylex/blob/main/.github/CONTRIBUTING.md)
to learn about our development process, how to propose bug fixes and
improvements, and how to build and test your changes.

### Architectural Principles

Before proposing a change or addition to the StyleX API, you should familiarize
yourself with the
[goals and architectural principles](https://stylexjs.com/docs/learn/thinking-in-stylex/)
of the project.

### License

StyleX is [MIT licensed](./LICENSE).

