# Known differences from the Babel plugin

Every place where oxs output is allowed to differ from `@stylexjs/babel-plugin` (main b39a8c4d).
Anything not listed here is a bug. Add an entry with a repro when you make a difference on
purpose, and say why.

## Babel crashes

| case | Babel | oxs |
|---|---|---|
| `unstable_createThemeNested` with a key that an empty `unstable_defineVarsNested({})` group lacks (fuzz 1gkw8nw) | `TypeError: Cannot read properties of undefined (reading 'slice')` in `styleXCreateTheme` | throws the same message |

| `firstThatWorks("var(--a)", 10.25)`, a number after a variable (fuzz aslp10) | `TypeError: varName.startsWith is not a function` | `var(--a,10.25)`. Babel's reduce calls `startsWith` on the number. oxs does not copy the crash |

The fuzzer files Babel TypeErrors as `babel-crash` and does not count them against oxs, even
when oxs matches.

## Formatting that the fuzzer and apps treat as the same

Through `js/index.js` the output is spliced text, not Babel-printed. Quoted vs unquoted keys,
comments, whitespace and generated `_temp` names can differ. `tools/fuzz/compare.cjs` compares a
canonical print (`format-only`). Through `js/babel.js` Babel prints the program, so these vanish.

## Syntax errors in cross-file imports

With `unstable_moduleResolution: {type: 'experimental_crossFileParsing'}`, an imported file that
does not parse throws Babel's `unknown: <message> (<line>:<column>)` and the frame of that file.

| case | Babel | oxs |
|---|---|---|
| `import { x } from './broken.stylex'` where the file is `export const x = ;` | the error has `loc` `{line: 1, column: 17}`, a position in the imported file | same message and frame, no `loc`. A `loc` would make the Babel adapter add a second frame from the importing file |
| the imported file is `export const x = 1 2;` | `unknown: Missing semicolon. (1:18)` | `unknown: Expected a semicolon or an implicit semicolon after a statement, but found none (1:18)`. Same location and frame. Only unexpected token, a missing expected token and an unterminated string use Babel's wording. Other syntax errors keep oxc's text |

## Syntax errors

oxs raises the early errors Babel's parser raises (redeclarations, `break` outside a loop,
`super()` outside a constructor, strict-mode rules and the rest) with Babel's message and
position, in every file and in cross-file imports (`crates/stylex_oxc/src/early.rs`,
`tests/early/run.cjs`). What still differs:

| case | Babel | oxs |
|---|---|---|
| `if (x) let y = 1;`, `a?.b = 1;`, `let = ;` | `Lexical declaration cannot appear in a single-statement context.`, `Support for the experimental syntax 'optionalChainingAssign' isn't currently enabled`, `Unexpected reserved word 'let'.` | throws oxc's parse error text. Generic parse errors use Babel's wording only for unexpected and missing tokens |
| a Flow file with runtime syntax (`match`, `component`, `hook`) that exports a name twice, repeats a regex flag, or uses `#x in o` | compiles (hermes-parser does not check these) | throws. Flow's or oxc's parser rejects the file before lowering |
| a file with no StyleX import, sx prop or atoms import that does not parse | throws | returns the file unchanged without parsing it. The bundler or Babel parses every module itself and reports the error |

## Not differences

- `tools/fuzz` `cssproc-diff` used to show for programs with imported `.stylex` files. That was the
  check feeding only the file's own rules to the candidate. Fixed in the tool.
- Source maps through `js/babel.js`. oxs's map becomes the generator's input map (core and
  direct-generator callers like Next.js), so Babel-printed maps point at the source.
  `tools/smcheck.cjs` (`oxs-babel`, `oxs-babel-gen`) matches or beats Babel's own maps.
- A block comment right before a replaced StyleX call. Babel prints it on its own line because
  the replacement has no location. `js/babel.js` drops the location of such a node too.
