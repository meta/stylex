/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
const postcss = require('postcss');
const createBuilder = require('./builder');
const {
  getEffectiveBabelConfig,
  resolveExclude,
  resolveImportSourcesWithMetadata,
  resolveIncludeWithMetadata,
} = require('./discovery');

function isDebugEnabled() {
  const value = String(process.env.STYLEX_POSTCSS_DEBUG ?? '').toLowerCase();
  return value === '1' || value === 'true' || value === 'yes';
}

module.exports = function createPlugin() {
  const PLUGIN_NAME = '@stylexjs/postcss-plugin';

  const builder = createBuilder();

  const isDev = process.env.NODE_ENV === 'development';

  const plugin = ({
    cwd = process.cwd(),
    // By default reuses the Babel configuration from the project root.
    // Use `babelrc: false` to disable this behavior.
    babelConfig = {},
    include,
    exclude,
    useCSSLayers = false,
    styleResolution = 'application-order',
    importSources,
  }) => {
    const effectiveBabelConfig = getEffectiveBabelConfig({
      babelConfig,
      cwd,
    });

    const importSourcesResolution = resolveImportSourcesWithMetadata({
      importSources,
      babelConfig: effectiveBabelConfig,
      cwd,
    });
    const effectiveImportSources = importSourcesResolution.importSources;

    const includeResolution = resolveIncludeWithMetadata({
      cwd,
      include,
      importSources: effectiveImportSources,
    });
    const effectiveInclude = includeResolution.include;

    const effectiveExclude = resolveExclude({
      include,
      exclude,
    });

    const excludeWithDefaults = [
      // Exclude type declaration files by default because it never contains any CSS rules.
      '**/*.d.ts',
      '**/*.flow',
      ...effectiveExclude,
    ];

    if (isDebugEnabled()) {
      console.info(
        `[${PLUGIN_NAME}] Auto-discovery details:\n${JSON.stringify(
          {
            cwd,
            importSourcesSource: importSourcesResolution.source,
            importSources: effectiveImportSources,
            include: effectiveInclude,
            includeWasExplicit: includeResolution.hasExplicitInclude,
            discoveredDependencyDirectories:
              includeResolution.discoveredDependencyDirectories,
            exclude: excludeWithDefaults,
          },
          null,
          2,
        )}`,
      );
    }

    // Whether to skip the error when transforming StyleX rules.
    // Useful in watch mode where Fast Refresh can recover from errors.
    // Initial transform will still throw errors in watch mode to surface issues early.
    let shouldSkipTransformError = false;

    return {
      postcssPlugin: PLUGIN_NAME,
      plugins: [
        // Processes the PostCSS root node to find and transform StyleX at-rules.
        async function (root, result) {
          const fileName = result.opts.from;

          // Configure the builder with the provided options
          await builder.configure({
            include: effectiveInclude,
            exclude: excludeWithDefaults,
            cwd,
            babelConfig: effectiveBabelConfig,
            useCSSLayers,
            styleResolution,
            importSources: effectiveImportSources,
            isDev,
          });

          // Find the "@stylex" at-rule
          const styleXAtRule = builder.findStyleXAtRule(root);
          if (styleXAtRule == null) {
            return;
          }

          // Get dependencies to be watched for changes
          const dependencies = builder.getDependencies();

          // Add each dependency to the PostCSS result messages.
          // This watches the entire "./src" directory for "./src/**/*.{ts,tsx}"
          // to handle new files and deletions reliably in watch mode.
          for (const dependency of dependencies) {
            result.messages.push({
              plugin: PLUGIN_NAME,
              parent: fileName,
              ...dependency,
            });
          }

          // Build and parse the CSS from collected StyleX rules
          const css = await builder.build({
            shouldSkipTransformError,
          });
          const parsed = await postcss.parse(css, {
            from: fileName,
          });

          // Replace the "@stylex" rule with the generated CSS
          styleXAtRule.replaceWith(parsed);

          result.root = root;

          if (!shouldSkipTransformError) {
            // Build was successful, subsequent builds are for watch mode
            shouldSkipTransformError = true;
          }
        },
      ],
    };
  };

  plugin.postcss = true;

  return plugin;
};
