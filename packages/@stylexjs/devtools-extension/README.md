# @stylexjs/devtools-extension

DevTools extension for inspecting StyleX styles in Chrome, Firefox, and Safari.

## Build

```sh
yarn run build -w @stylexjs/devtools-extension
```

The build produces clean browser-specific directories with byte-identical panel,
inspected runtime, and UI assets:

- `dist/chrome`
- `dist/firefox`
- `dist/safari`

The extension currently uses `@stylexjs/stylex@0.19.0`, the latest published
StyleX release.

## Load In Chrome

1. Open `chrome://extensions` in Chrome 148 or newer.
2. Enable **Developer mode** and click **Load unpacked**.
3. Select `packages/@stylexjs/devtools-extension/dist/chrome`.

## Load In Firefox

1. Open `about:debugging#/runtime/this-firefox` in Firefox 153 or newer.
2. Click **Load Temporary Add-on**.
3. Select `packages/@stylexjs/devtools-extension/dist/firefox/manifest.json`.

Firefox does not expose the Chrome resource APIs used for source previews and
opening files. The Firefox build shows source metadata and provides a copy
location command instead.

## Load In Safari

The Safari build must be packaged by a separate Safari Web Extension host app;
the Xcode project is intentionally not part of this repository. Build this
package, copy `dist/safari` into the host app's extension resources, and build
the host app in Xcode. Safari uses a full Web Inspector tab because it does not
support Elements sidebar extensions. Grant StyleX DevTools access to the sites
you inspect when Safari asks. The Safari-only HTTP/HTTPS host permission is
required because WebKit checks site access before allowing
`devtools.inspectedWindow.eval()`.

Browser capabilities are intentionally separate:

- Chrome uses the Elements sidebar, source previews, and source navigation.
- Firefox uses the Elements sidebar and copy-only source locations.
- Safari uses a full Inspector tab, copy-only source locations, selection
  polling, and an instance-scoped DevTools-page bridge for inspected-page calls.

## Verify And Package

```sh
yarn test -w @stylexjs/devtools-extension
yarn run verify:build -w @stylexjs/devtools-extension
yarn run web-ext:lint -w @stylexjs/devtools-extension
yarn run web-ext:build -w @stylexjs/devtools-extension
```

Packaging validates an explicit file allowlist before writing deterministic
store archives to `artifacts/chrome` and `artifacts/firefox`.

Firefox lint also verifies that the only accepted warnings are React DOM's two
generated compatibility assignments; any extension-owned or new warning fails
the check.
