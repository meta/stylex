# Changelog

## 0.19.1 (Sep 15, 2026)
- Expand StyleX DevTools to Firefox and Safari, with improved style inspection,
  overrides, and metadata export.
- Breaking: Remove `enableDebugClassNames` and property-prefixed debug class
  names so generated identifiers stay stable between development and production.
- Add deterministic ordering for `defineConsts` pixel `min-width` and
  `max-width` breakpoints, and avoid Chromium rounding gaps at generated
  `max-width` boundaries.
- Fix hashed CSS asset matching in `unplugin` and stale output from concurrent
  PostCSS builds.
- Correct generated `content` values containing quote characters.
- Improve ESLint handling of media query ordering, numeric grid lines,
  type-only exports, and `calc()` outline widths.

## 0.19.0 (Jun 14, 2026)
- New `@stylexjs/atoms` package for inline atomic styles.
- ESLint 10 compatibility.
- Add `gap` auto-fix for `legacy-expand-shorthands` and skip single-value `gap`
  and `flex` in `valid-shorthands`.
- Extend `textWrap` validation with `pretty` and `stable` values.
- Fix `sx` prop runtime stylex import injection.
- Fix pseudo-element + pseudo-class selector ordering.
- Fix aliased theme file resolution to absolute paths.
- Fix `sort-keys` autofix ordering.
- Fix media type parenthesization in `and` chains in the style value parser.

## 0.18.3 (Apr 29, 2026)
- Experimental nested APIs for `defineVars`, `defineConsts`, and `createTheme`.
- Add recursive derivation support for `defineVars`.
- ESLint auto-fixers for shorthand expansion, gated by `styleResolution` config.
- Respect project `browserslist` config instead of hardcoding `>= 1%`.
- Add `:empty` pseudo-class and unclosed string detection to the compiler.
- StyleX DevTools extension now available in the Chrome Web Store.

## 0.18.2 (Mar 23, 2026)
- Bring back `stylex.attrs` for SSR and non-React frameworks.
- Add `layersBefore`, `layersAfter`, and `layersPrefix` for `@layer` control.
- Add `flex`, `grid`, and `animation` shorthand expansion to ESLint plugin.
- ESLint fixes: bare number support, false positive fixes in `stylex.create`.

## 0.18.1 (Mar 5, 2026)
- Add simple `sx={}` JSX prop syntax as an alternative to `stylex.props`.
- Fix `unplugin` `generateBundle` hook to use `emitFile` instead of direct bundle assignment.

## 0.18.0 (Mar 3, 2026)
- Add `stylex.env` API for compile-time constants and shareable design tokens.
- New `@stylexjs/create` CLI for scaffolding projects with `npm create @stylexjs`.
- Add Bun support and improvements to the `unplugin` bundler plugin.
- Support attribute selectors in `stylex.when` and conditional styles.
- Fix priority calculation for compound pseudo selectors.

## 0.17.5 (Jan 12. 2026)
- New `no-conflicting-props` lint rule.
- Add `disallowedPropertiesValidation` config to disable property compiler errors by default.
- Preserve units in zero values for CSS variables.
- Fix `unplugin` cache invalidation on style changes.
- Add `env` compiler option to expose `stylex.env` compile-time constants.

## 0.17.4 (Dec 18. 2025)
- New playground within the docs website.
- Fix named exports detection on Turbopack.

## 0.17.3 (Dec 12, 2025)

- Add docs for setup instructions.
- Improve AST detection for `.stylex` named exports
- Add browser rollup bundle.
- Fix unplugin file suffix parsing.

## 0.17.2 (Dec 1, 2025)

- Pass importSources to babel plugin in `unplugin` plugin.
- Fix custom typescript typedef mistake.

## 0.17.1 (Nov 26, 2025)

- Add support for compiling stylex from imported packages.
- Add runtime injection support for `defineConsts` constants.
- Show full debug file paths.
- Fix media query ordering for queries with 'screen and'.
- Add `debugFilePath` config option.
- Fix unitless zoom bug for float values.
- Fix props compilation for array mutations.

## 0.17.0 (Nov 18, 2025)

- Add docs for ESLint rules and `stylex.when` API.
- Add `defineMarker()` for custom markers for selector combinators.
- New unplugin bundler plugin implementation for various bundlers (Vite,
  Webpack, Rspack, Rollup, etc.).
- Enhance `stylex.props` to precompile more often for better performance.
- Order pseudo-classes and `stylex.when` selectors according to priorities in
  `sort-keys.
- Add support for ternary and logical expressions in `valid-styles`.
- Bump specificity of `stylex.when` selectors over defaults.
- Add polyfill for logical float values in legacy mode.

## 0.16.3 (Oct 27, 2025)

- Add configs to `sort-keys` property ordering.
- Create new `defineConsts` specific file extension.
- Add `defineConsts` and various file extension support to `enforce-extension`.
- Add config for custom module resolution.
- Turn `enableMediaQueryOrder` on by default.
- Updates to docs and Flow.

## 0.16.2 (Oct 13, 2025)

- Handle descendant selector styles in `valid-styles` rule.
- Adjust descendant selector `.when` priorities.
- Fix `defineVars` and `createTheme` at-rules priorities.
- Update `defineConsts` types for non-stylex usage.

## 0.16.1 (Oct 2, 2025)

- New `no-lookahead-selector` lint rule to flag certain descendant and sibling
  selectors.
- Fix priorities for descendant and sibling selectors.
- Fix color functions for `valid-shorthands` rule.
- Fix hoisting issues with duplicate keys in `create` calls.
- Add storybook example.

## 0.16.0 (Sep 25, 2025)

- Added support for descendant and shared selectors.
- Support CSS variable overrides with `defineConsts`.
- Add `valid-styles` support to CSS variable overrides in `create` calls.
- Replace `valid-styles` object check with Flow typing

## 0.15.4 (Sep 7, 2025)

- Add configuration modes to `processStylexRules`.
- Support local resolved constants, `positionTry`, and '0' values in
  `valid-styles` ESLint rule.
- Implement `defineConsts` for dynamic styles.
- Create `.transformed` file extension for preresolved variables.

## 0.15.3 (Aug 13, 2025)

- Optimize precomputed `props` calls in JSX.
- Fix class name construction for dynamic contextual styles.
- Handle all unit values in media query rewriting.

## 0.15.2 (Aug 1, 2025)

- Exclude private dependencies from `@stylexjs/babel-plugin` package.
- Reduce chances of dynamic variable name collisions.

## 0.15.1 (Aug 1, 2025)

### Fixes

- Hoist stylex.create and static className objects to the top level for support
  inside functions.

## 0.15.0 (Jul 31, 2025)

### New features

- Enable media query ordering and parsing behind `enableMediaQueryOrder` flag.
- Integrate media query parser for automatic media query validation and
  normalization.

### Fixes

- Implement merging of width, height, and ranges in media query transformer.
- Optimize dynamic styles output for conditionals and template literals.
- Fix TypeScript types for `stylex.types.*` functions.
- Fix opaque type issues for InlineStyles.

## 0.14.3 (Jul 22, 2025)

### Fixes

- Do not emit class names for `null` or `undefined` dynamic styles.
- Optimize dynamic styles output for binary and unary expressions.

## 0.14.2 (Jul 14, 2025)

### Fixes

- ESLint plugin style validation for length properties (#1136)
- Remove legacy RTL flipping of boxShadow, cursor, textShadow values.

## 0.14.1 (Jul 3, 2025)

### Fixes

- Another fix for TypeScript types.

## 0.14.0 (Jun 30, 2025)

### New features

- Add `viewTransitionClass` API for creating CSS View Transitions.
- ESLint plugin includes `validImports` options for all rules.
- ESLint plugin includes autofix for all remaining nonstandard CSS properties
  when using the `valid-styles` rule.

### Breaking changes

- Make `property-specificity` the default `styleResolution`.

### Fixes

- Fix theming in dev/debug mode.
- Avoid putting certain `@-rules` in `@layer` blocks.
- Fix type exports for TypeScript.

## 0.13.1 (May 21, 2025)

### Fixes

- Export additional Types.

## 0.13.0 (May 19, 2025)

### New features

- Add `positionTry` API for creating `@property-try` declarations.
- Add `defineConsts` API for inlining constant values.
- Re-write of the runtime style injection module to be more reliable.

### Breaking changes

- The `runtimeInjection` compiler option is now disabled by default when `dev`
  is true.
- The ESLint rule `no-legacy-conditional-styles` is renamed to
  `no-legacy-contextual-styles`.
- The `useRemForFontSize` compiler option is renamed to `enableFontSizePxToRem`.
  It is disabled by default and should not be used directly.
- The `genConditionalClasses` compiler option is renamed to
  `enableInlinedConditionalMerge`. It is enabled by default and should not be
  used directly.
- The `attrs` API is removed due to low usage and redundancy with the `props`
  API.

### Fixes

- Fix the TypeScript types for themes and types functions.
- Fix the creation of duplicate classNames when defining nested pseudo-classes.
- Fix that allows the ESLint plugin to support use of `importSources` object
  syntax in `validImports`.
- Fix incorrect compiler error messages.
- Fix a bug that incorrectly wrapped CSS variables in quotes when used in the
  `content` property.
- Fix a bug in the `firstThatWorks` API when the last value was a variable.
- Allow `importSources` to be configured in the PostCSS plugin for React Strict
  DOM compatibility.

### Deprecations

- Deprecate `@stylexjs/shared` package.

## 0.12.0 (Apr 10, 2025)

### New features

- Hash keys in compiled style objects to reduce generated code size.
- New eslint rule to flag use of legacy Media Query and pseudo-class syntax.

### Fixes

- Fix pseudo-elements bug in dynamic styles.
- Performance improvements to `createTheme` compilation by caching object
  evaluation.
- Disallow spreading in `create` calls.

### Deprecations

- Deprecate `@stylexjs/dev-runtime` package.
- Deprecate `@stylexjs/esbuild-plugin` package.
- Deprecate `@stylexjs/nextjs-plugin` package.
- Deprecate `@stylexjs/open-props` package.
- Deprecate `@stylexjs/webpack-plugin` package.

## 0.11.1 (Mar 3, 2025)

### Fixes

- Fix `create` compilation regression for string and number keys.
- Fix babel path resolution within monorepos.

## 0.11.0 (Feb 27, 2025)
