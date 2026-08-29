# compiler-conformance

Golden fixtures that describe what a StyleX compiler must do, independently of
how it is implemented.

Each fixture pins the four observable outputs of compiling one file: the
generated JavaScript, the StyleX metadata, the generated CSS, and the
diagnostics. Fixtures are plain data — an implementation written in any language
can read them and check itself against the same contract.

The StyleX Babel plugin is the first consumer, via the adapter in
`packages/@stylexjs/babel-plugin/test-utils/babel-conformance-adapter.js` and
the test in `packages/@stylexjs/babel-plugin/__tests__/golden-fixtures-test.js`.

## Layout

```
fixtures/<fixture-name>/
  manifest.json    how to compile the fixture
  input.js         the entry file (name set by `entry`)
  expected.json    the recorded result
  ...              any extra files the entry imports
```

## `manifest.json`

| Field            | Type       | Default                                               | Meaning                                                      |
| ---------------- | ---------- | ----------------------------------------------------- | ------------------------------------------------------------ |
| `description`    | `string`   | `""`                                                  | What behavior this fixture pins down.                        |
| `entry`          | `string`   | `"input.js"`                                          | File in the fixture directory to compile.                    |
| `syntax`         | `string[]` | `["flow"]`                                            | Source syntax the entry uses, e.g. `["flow", "jsx"]`.        |
| `pluginOptions`  | `object`   | `{}`                                                  | StyleX compiler options.                                     |
| `processOptions` | `object`   | `{"useLayers": false, "enableLTRRTLComments": false}` | Options for the step that assembles rules into a stylesheet. |

`pluginOptions` and `processOptions` are StyleX's own documented options, so
they carry over unchanged between implementations. `syntax` is deliberately
abstract: an adapter maps it onto whatever its parser needs.

Avoid options that take an absolute path. Fixture output must not depend on
where the repository is checked out.

## `expected.json`

For a fixture that compiles successfully:

```json
{
  "status": "ok",
  "js": "…generated JavaScript…",
  "metadata": [
    ["x1e2nbdu", { "ltr": ".x1e2nbdu{color:red}", "rtl": null }, 3000]
  ],
  "css": ".x1e2nbdu{color:red}",
  "warnings": [],
  "errors": []
}
```

For a fixture whose transform is expected to fail:

```json
{
  "status": "error",
  "error": { "message": "create() can only accept an object." },
  "warnings": [],
  "errors": []
}
```

`warnings` and `errors` are non-fatal diagnostics, split by the channel they
were reported on. A fixture can succeed and still report diagnostics —
`options-warning` does exactly that. `error` is the fatal one that ended the
transform.

## How results are compared

**`js` is compared semantically**, as a normalized syntax tree. Ignored are
whitespace, indentation, semicolons, comments, quote style and numeric literal
spelling, how a non-computed property key is written (`{a: 1}`, `{'a': 1}` and
`{1: x}` name the same properties as `{"a": 1}`, `{a: 1}` and `{"1": x}`), and
property shorthand (`{a}` equals `{a: a}`). So an implementation only has to
agree on the program it emits, not on how it prints it. The string stored in
`expected.json` is one valid printing of that program; reformatting it does not
change the outcome. When the programs genuinely differ, the Babel test falls
back to asserting on the sources so the failure shows a readable diff.

**`metadata`, `css`, `warnings`, `errors` and `status` are compared exactly**,
after normalization:

- Line endings are normalized to `\n`, and trailing whitespace is stripped from
  each CSS line.
- Absolute paths become `<FIXTURE_ROOT>` and `<REPO_ROOT>`.
- In diagnostics, terminal color escapes are removed, a leading
  `<FIXTURE_ROOT>/<entry>: ` location prefix is dropped, a trailing source
  excerpt (code frame) is dropped, and a leading `[implementation-name]` tag is
  collapsed to `[stylex]`. What is compared is the message itself, not how an
  implementation decorates it. Color matters here in practice: Babel
  syntax-highlights a code frame whenever `CI` is set, so an uncolored message
  locally and a colored one on CI have to normalize to the same text.
- Object keys are sorted, because key order carries no meaning in JSON. **Array
  order is preserved**, because the order of emitted rules is part of the
  contract.

## Consuming this suite from another implementation

Implement an adapter — compile one fixture, return its result in the shape below
— and the shared runner does the rest:

```js
{
  status: 'ok',
  js: '…',           // generated JavaScript
  metadata: [...],   // StyleX metadata
  css: '…',          // stylesheet assembled from the metadata
  warnings: [...],   // non-fatal diagnostics
  errors: [...],
}
```

or, when the transform fails:

```js
{ status: 'error', error: { message: '…' }, warnings: [...], errors: [...] }
```

From JavaScript, drive it with the helpers this package exports:

```js
const {
  exactPart,
  getFixtureNames,
  isJsEquivalent,
  jsPart,
  loadFixture,
  normalizeResult,
  readExpected,
} = require('compiler-conformance');

for (const name of getFixtureNames()) {
  const fixture = loadFixture(name);
  const actual = normalizeResult(fixture, myAdapter.transform(fixture));
  const expected = readExpected(name, fixture);

  assert.deepStrictEqual(exactPart(actual), exactPart(expected));
  assert.ok(isJsEquivalent(jsPart(expected), jsPart(actual), fixture.syntax));
}
```

From another language, read `manifest.json` and `expected.json` directly and
apply the rules in [How results are compared](#how-results-are-compared). The
fixtures are the contract; these helpers are only a convenience.

Two things to be aware of when porting:

- Some fixtures depend on their own location. `define-vars-theme` uses
  `commonJS` module resolution, which names a file
  `<package name>:<path within the package>` — here
  `compiler-conformance:fixtures/define-vars-theme/input.stylex.js`. That name
  is hashed into the generated CSS variables, so it is stable across checkouts
  but changes if the fixture is moved or the package is renamed.
- `props-and-merge` is the only fixture that needs JSX.

## Updating the recorded output

Re-record every `expected.json` from the current Babel output and review the
diff:

```
STYLEX_UPDATE_GOLDEN=1 npx jest golden-fixtures
```

from `packages/@stylexjs/babel-plugin`. A change here is a change to the
behavioral contract, so the diff deserves the same scrutiny as the code that
caused it.

The comparison rules above are themselves tested, independently of any compiler
— run `npm test` in this package.
