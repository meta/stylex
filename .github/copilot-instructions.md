## StyleX Code Review Guidelines

Focus on critical issues only. Do not nitpick naming, style preferences, or suggest refactors on bug-fix PRs.

### Must-haves

- **CLA**: External contributors must sign the CLA at https://code.facebook.com/cla. Flag if the `CLA Signed` label is missing on external PRs.
- **CI signals**: Flag if any CI check is failing and the PR does not acknowledge it.
- **Tests**: New babel-plugin features or bug fixes must include tests. Prefer `toMatchInlineSnapshot()` for all assertion snapshots.
- **Flow syntax**: Use modern Flow — `Readonly<>` not `$ReadOnly<>`, `unknown` not `mixed`, `ReadonlyArray` not `$ReadOnlyArray`.
- **Lint and formatting**: Code must pass eslint and prettier. Flag obvious violations.

### Babel plugin

- New compile-time APIs must throw at runtime with a clear error message.
- New APIs should include documentation, eslint plugin support, and tests.
- Review test snapshot output and compiled JS/CSS to verify no regressions in generated code.

### Release PRs

- Must include a changelog entry.
- Version bumps must be consistent across all workspace packages.
- Release blog posts should credit community contributors with `(Thanks [username](url)!)` format.

### Documentation

- Code examples in docs and blog posts must use valid CSS and JS syntax.
