# Contributing

## Reporting Issues and Asking Questions

Before opening an issue, please search the issue tracker to make sure your issue
hasn't already been reported. Please note that your issue may be closed if it
doesn't include the information requested in the issue template.

## Getting started

Visit the issue tracker to find a list of open issues that need attention.

Fork, then clone the repo:

```
git clone https://github.com/your-username/stylex.git
```

Make sure you have yarn@1.22 and node@>=18 installed. Then install the package
dependencies:

```
yarn install
```

## Automated tests

To run the linter:

```
yarn lint
```

To run flow:

```
yarn flow
```

## Compile and build

To compile the source code:

```
yarn build
```

To run all the tests (will build automatically):

```
yarn test
```

…in watch mode (will build only once):

```
yarn test --watch
```

## Documentation

If necessary, first build the StyleX packages (`yarn build`), then start the
docs locally:

```
yarn workspace docs start
```

### New Features

Please, familiarize yourself with
[StyleX's goals and architectural principles](https://stylexjs.com/docs/learn/thinking-in-stylex/),
and open an issue with a proposal when suggesting a new feature of behavioural
change. We don't want you to waste your efforts on a pull request that we won't
want to accept.

## Pull requests

**Before submitting a pull request**, please make sure the following is done:

1. Fork the repository and create your branch from `main`.
2. If you've added code that should be tested, add tests!
3. If you've changed APIs, update the documentation.
4. Ensure the tests pass (`yarn test`).

You can now submit a pull request, referencing any issues it addresses.

Please try to keep your pull request focused in scope and avoid including
unrelated commits.

After you have submitted your pull request, we'll try to get back to you as soon
as possible. We may suggest some changes or improvements.

Thank you for contributing!

## Typical Editor setup

### VS Code

If you're using Visual Studio Code, we recommend the following setup for the
best experience.

#### Extensions

We recommend you have the following extensions installed:

- [Flow Language Support](https://marketplace.visualstudio.com/items?itemName=flowtype.flow-for-vscode)
- [ESLint](https://marketplace.visualstudio.com/items?itemName=dbaeumer.vscode-eslint)
- [Prettier - Code formatter](https://marketplace.visualstudio.com/items?itemName=esbenp.prettier-vscode)

#### Turn off Typescript within JS files

Additionally, since StyleX is authored with the
[Flow typesystem](https://flow.org), it is helpful to turn off Typescript
type-checking within Javascript files:

```json
{
  "javascript.validate.enable": false
}
```
