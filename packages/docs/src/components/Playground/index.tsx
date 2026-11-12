/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import * as React from 'react';
import { useEffect, useState, useRef, useMemo, useCallback } from 'react';

import * as stylex from '@stylexjs/stylex';
// @ts-ignore - CJS module
import { transform } from '@babel/standalone';
import { loadSandpackClient } from '@codesandbox/sandpack-client';
import Editor, { useMonaco } from '@monaco-editor/react';
// @ts-ignore - CJS module
import path from 'path-browserify';
// import { useColorMode } from '@docusaurus/theme-common';
import {
  useQueryParam,
  JsonParam,
  withDefault,
  StringParam,
  // @ts-ignore
} from 'use-query-params';
import { Tabs } from './Tabs';
import prettier from 'prettier';
import * as babelPlugin from 'prettier/plugins/babel.js';
import * as estreePlugin from 'prettier/plugins/estree.js';
import {
  INITIAL_INPUT_FILES,
  INITIAL_BUNDLER_FILES,
  CSS_PRELUDE,
} from './demoConstants';
import {
  compressToEncodedURIComponent,
  decompressFromEncodedURIComponent,
} from 'lz-string';

// @ts-ignore - CJS module
import * as stylexPluginModule from '@stylexjs/babel-plugin';
import { vars, playgroundVars } from '@/theming/vars.stylex';
import { ChevronDown } from 'lucide-react';
const stylexPlugin: typeof import('@stylexjs/babel-plugin').default =
  // @ts-ignore - handle CJS default export
  stylexPluginModule.default ?? stylexPluginModule;

declare const STYLEX_TYPES: Record<string, string>;
declare const REACT_TYPES: string;
declare const REACT_JSX_RUNTIME_TYPES: string;

const LIGHT_EDITOR_THEME = 'stylex-light';
const DARK_EDITOR_THEME = 'stylex-dark';
const OUTPUT_TABS = [
  { key: 'js', label: 'JS output' },
  { key: 'css', label: 'CSS output' },
] as const;

const defineMonacoThemes = (monacoInstance: any) => {
  if (!monacoInstance?.editor?.defineTheme) {
    return;
  }

  const colors = {
    purpleLight: '#9b6ad4',
    purpleDark: '#c9a0f0',
    pinkLight: '#b35cc6',
    pinkDark: '#d9a0e8',
    blueLight: '#3966b8',
    blueDark: '#92b8f8',
    commentLight: '#6b6b7a',
    commentDark: '#6b7280',
    foregroundLight: '#000000',
    foregroundDark: '#e4e4e7',
  };

  monacoInstance.editor.defineTheme(LIGHT_EDITOR_THEME, {
    base: 'vs',
    inherit: true,
    rules: [
      {
        token: 'keyword',
        foreground: colors.purpleLight.slice(1),
        fontStyle: '',
      },
      { token: 'keyword.control', foreground: colors.purpleLight.slice(1) },
      { token: 'type', foreground: colors.pinkLight.slice(1) },
      { token: 'type.identifier', foreground: colors.pinkLight.slice(1) },
      { token: 'support.type', foreground: colors.pinkLight.slice(1) },
      { token: 'entity.name.function', foreground: colors.blueLight.slice(1) },
      { token: 'support.function', foreground: colors.blueLight.slice(1) },
      { token: 'function', foreground: colors.blueLight.slice(1) },
      { token: 'string', foreground: colors.blueLight.slice(1) },
      { token: 'string.quoted', foreground: colors.blueLight.slice(1) },
      { token: 'number', foreground: colors.blueLight.slice(1) },
      { token: 'constant.numeric', foreground: colors.blueLight.slice(1) },
      {
        token: 'comment',
        foreground: colors.commentLight.slice(1),
        fontStyle: 'italic',
      },
      { token: 'variable', foreground: colors.foregroundLight.slice(1) },
      { token: 'identifier', foreground: colors.foregroundLight.slice(1) },
      { token: 'tag', foreground: colors.pinkLight.slice(1) },
      { token: 'tag.html', foreground: colors.pinkLight.slice(1) },
      { token: 'metatag', foreground: colors.pinkLight.slice(1) },
      { token: 'attribute.name', foreground: colors.blueLight.slice(1) },
      { token: 'attribute.value', foreground: colors.blueLight.slice(1) },
      { token: 'delimiter', foreground: colors.commentLight.slice(1) },
      { token: 'delimiter.bracket', foreground: colors.purpleLight.slice(1) },
      { token: 'operator', foreground: colors.blueLight.slice(1) },
      { token: 'constant', foreground: colors.blueLight.slice(1) },
      { token: 'annotation', foreground: colors.pinkLight.slice(1) },
    ],
    colors: {
      'editor.background': '#ffffff',
      'editor.foreground': colors.foregroundLight,
      'editorLineNumber.foreground': '#a0a0b0',
      'editorLineNumber.activeForeground': colors.purpleLight,
      'editor.selectionBackground': '#ddd6fe40',
      'editor.lineHighlightBackground': '#f8f5ff',
      'editorCursor.foreground': colors.purpleLight,
      'editorIndentGuide.background': '#e5e5f0',
      'editorIndentGuide.activeBackground': colors.purpleLight + '40',
    },
  });

  monacoInstance.editor.defineTheme(DARK_EDITOR_THEME, {
    base: 'vs-dark',
    inherit: true,
    rules: [
      {
        token: 'keyword',
        foreground: colors.purpleDark.slice(1),
        fontStyle: '',
      },
      { token: 'keyword.control', foreground: colors.purpleDark.slice(1) },
      { token: 'type', foreground: colors.pinkDark.slice(1) },
      { token: 'type.identifier', foreground: colors.pinkDark.slice(1) },
      { token: 'support.type', foreground: colors.pinkDark.slice(1) },
      { token: 'entity.name.function', foreground: colors.blueDark.slice(1) },
      { token: 'support.function', foreground: colors.blueDark.slice(1) },
      { token: 'function', foreground: colors.blueDark.slice(1) },
      { token: 'string', foreground: colors.blueDark.slice(1) },
      { token: 'string.quoted', foreground: colors.blueDark.slice(1) },
      { token: 'number', foreground: colors.blueDark.slice(1) },
      { token: 'constant.numeric', foreground: colors.blueDark.slice(1) },
      {
        token: 'comment',
        foreground: colors.commentDark.slice(1),
        fontStyle: 'italic',
      },
      { token: 'variable', foreground: colors.foregroundDark.slice(1) },
      { token: 'identifier', foreground: colors.foregroundDark.slice(1) },
      { token: 'tag', foreground: colors.pinkDark.slice(1) },
      { token: 'tag.html', foreground: colors.pinkDark.slice(1) },
      { token: 'metatag', foreground: colors.pinkDark.slice(1) },
      { token: 'attribute.name', foreground: colors.blueDark.slice(1) },
      { token: 'attribute.value', foreground: colors.blueDark.slice(1) },
      { token: 'delimiter', foreground: colors.commentDark.slice(1) },
      { token: 'delimiter.bracket', foreground: colors.purpleDark.slice(1) },
      { token: 'operator', foreground: colors.blueDark.slice(1) },
      { token: 'constant', foreground: colors.blueDark.slice(1) },
      { token: 'annotation', foreground: colors.pinkDark.slice(1) },
    ],
    colors: {
      'editor.background': '#1e1e1e',
      'editor.foreground': colors.foregroundDark,
      'editorLineNumber.foreground': '#5a5a6a',
      'editorLineNumber.activeForeground': colors.purpleDark,
      'editor.selectionBackground': '#7c3aed30',
      'editor.lineHighlightBackground': '#252530',
      'editorCursor.foreground': colors.purpleDark,
      'editorIndentGuide.background': '#333340',
      'editorIndentGuide.activeBackground': colors.purpleDark + '40',
    },
  });
};

function transformSourceFiles(sourceFiles: Record<string, string>) {
  const stylexRules = [];
  const transformedFiles: Record<string, string> = {};

  const sourceFilePaths = Object.keys(sourceFiles).map(
    (filename) => `/${filename}`,
  );

  for (const [filename, code] of Object.entries(sourceFiles)) {
    const isTSX = filename.endsWith('.tsx');
    const isTS = isTSX || filename.endsWith('.ts');
    const result = transform(code, {
      filename,
      plugins: [
        isTS && ['transform-typescript', { isTSX }],
        'syntax-jsx',
        [
          stylexPlugin,
          {
            dev: false,
            unstable_moduleResolution: {
              type: 'custom',
              filePathResolver(importPath: string, sourceFilePath: string) {
                if (importPath.startsWith('.')) {
                  const result = path.resolve(
                    path.dirname(sourceFilePath),
                    importPath,
                  );
                  if (sourceFilePaths.includes(result)) {
                    return result;
                  }
                  const matchingPrefix = sourceFilePaths.find((fullPath) =>
                    fullPath.startsWith(result),
                  );
                  if (matchingPrefix) {
                    return matchingPrefix;
                  }
                }
                return undefined;
              },
              getCanonicalFilePath(filePath: string) {
                return filePath;
              },
            },
          },
        ],
      ].filter(Boolean),
    });
    transformedFiles[filename] = result.code;
    if (result.metadata.stylex) {
      stylexRules.push(...result.metadata.stylex);
    }
  }

  const generatedCSS = stylexPlugin.processStylexRules(stylexRules, {
    useLayers: true,
  });

  return { transformedFiles, generatedCSS };
}

const decodeObjKeysOld = (obj: Record<string, string>) => {
  return Object.fromEntries(
    Object.entries(obj).map(([key, value]) => [key, decodeURIComponent(value)]),
  );
};

const encodeObjKeys = (obj: Record<string, string>) => {
  return compressToEncodedURIComponent(JSON.stringify(obj));
};

const decodeObjKeys = (string: string) => {
  return JSON.parse(decompressFromEncodedURIComponent(string));
};

const getColorMode = () => {
  if (typeof document !== 'undefined') {
    const root = document.documentElement;
    if (root.dataset.theme === 'dark' || root.classList.contains('dark')) {
      return 'dark';
    }
  }
  if (
    typeof window !== 'undefined' &&
    window.matchMedia('(prefers-color-scheme: dark)').matches
  ) {
    return 'dark';
  }
  return 'light';
};

export default function PlaygroundNew() {
  const [_inputFilesOld, _setInputFilesOld] = useQueryParam(
    'inputFiles',
    JsonParam,
  );

  let initialValue: Record<string, string> = INITIAL_INPUT_FILES;
  if (_inputFilesOld != null && Object.keys(_inputFilesOld).length > 0) {
    initialValue = decodeObjKeysOld(_inputFilesOld);
  }

  const [_inputFiles, _setInputFiles] = useQueryParam(
    'input',
    withDefault(StringParam, encodeObjKeys(initialValue)),
  );
  const inputFiles: Record<string, string> = useMemo(
    () => decodeObjKeys(_inputFiles),
    [_inputFiles],
  );
  const [colorMode, setColorMode] = useState<'light' | 'dark'>(() =>
    getColorMode(),
  );

  const [activeInputFile, setActiveInputFile] = useState<string>((): string => {
    const keys: string[] = Object.keys(inputFiles);
    return keys.includes('App.tsx') ? 'App.tsx' : keys[0]!;
  });
  const [transformedFiles, setTransformedFiles] = useState<
    Record<string, string>
  >({});
  const [cssOutput, setCssOutput] = useState('');
  const [sandpackInitialized, setSandpackInitialized] = useState(false);
  const iframeRef = useRef<HTMLIFrameElement | null>(null);
  const sandpackClientRef = useRef<any>(null);
  const [error, setError] = useState<Error | null>(null);
  const monaco = useMonaco();
  const editorRef = useRef<any>(null);
  const [activeOutputTab, setActiveOutputTab] = useState<'js' | 'css'>('js');
  const [outputCollapsed, setOutputCollapsed] = useState(true);

  const setInputFiles = useCallback(
    (updatedInputFiles: Record<string, string>, replace: boolean = true) => {
      _setInputFiles(
        encodeObjKeys(updatedInputFiles),
        replace ? 'replaceIn' : 'push',
      );
    },
    [],
  );

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const update = () => setColorMode(getColorMode());
    update();
    media.addEventListener('change', update);
    const observer =
      typeof MutationObserver !== 'undefined'
        ? new MutationObserver(update)
        : null;
    if (observer) {
      observer.observe(document.documentElement, {
        attributes: true,
        attributeFilter: ['data-theme', 'class'],
      });
    }
    return () => {
      media.removeEventListener('change', update);
      observer?.disconnect();
    };
  }, []);

  useEffect(() => {
    if (_inputFilesOld == null || Object.keys(_inputFilesOld).length === 0) {
      return;
    }
    _setInputFilesOld(null);
    setInputFiles(initialValue);
  }, []);

  const getUniqueFilename = useCallback(
    (baseName: string) => {
      if (!inputFiles[baseName]) {
        return baseName;
      }
      const [base, ...rest] = baseName.split('.');
      let counter = 2;
      let candidate = [`${base}-${counter}`, ...rest].join('.');
      while (inputFiles[candidate]) {
        counter += 1;
        candidate = [`${base}-${counter}`, ...rest].join('.');
      }
      return candidate;
    },
    [inputFiles],
  );

  const toComponentName = useCallback((filename: string) => {
    return (
      filename
        .replace(/\.[^/.]+$/, '')
        .split(/[^a-zA-Z0-9]+/)
        .filter(Boolean)
        .map((chunk: string) => chunk?.[0]?.toUpperCase() + chunk?.slice(1))
        .join('') || 'Component'
    );
  }, []);

  const createComponentTemplate = useCallback(
    (filename: string) => {
      const componentName = toComponentName(filename);
      return `import * as stylex from '@stylexjs/stylex';

const styles = stylex.create({
  root: {

  },
});

export default function ${componentName}() {
  return <div {...stylex.props(styles.root)}>${componentName} component</div>;
}
`;
    },
    [toComponentName],
  );

  const createStylexVarsTemplate = useCallback(() => {
    return `import * as stylex from '@stylexjs/stylex';

export const vars = stylex.defineVars({
  primary: '#ff6b6b',
  secondary: '#1e90ff',
  accent: '#fbbf24',
});
`;
  }, []);

  const getDefaultFilename = useCallback(
    (fileKind: string) =>
      fileKind === 'component'
        ? getUniqueFilename('Component.tsx')
        : getUniqueFilename('tokens.stylex.ts'),
    [getUniqueFilename],
  );

  const ensureMonacoModel = useCallback(
    (monacoInstance: any, filename: string, content: string) => {
      const uri = monacoInstance.Uri.file(`/${filename}`);
      const existingModel = monacoInstance.editor.getModel(uri);
      if (existingModel) {
        if (existingModel.getValue() !== content) {
          existingModel.setValue(content);
        }
        return existingModel;
      }
      return monacoInstance.editor.createModel(content, 'typescript', uri);
    },
    [],
  );

  function updateSandpack(updatedInputFiles: Record<string, string>) {
    try {
      const { transformedFiles, generatedCSS } =
        transformSourceFiles(updatedInputFiles);

      setTransformedFiles(transformedFiles as Record<string, string>);
      setCssOutput(generatedCSS);
      setError(null);

      if (sandpackClientRef.current) {
        const dynamicFiles = Object.keys(transformedFiles).reduce(
          (acc, filename) => ({
            ...acc,
            [`/${filename.replace(/\.tsx?$/, '.js')}`]: {
              code: transformedFiles[filename],
            },
          }),
          {},
        );
        sandpackClientRef.current.updateSandbox({
          files: {
            ...INITIAL_BUNDLER_FILES,
            ...dynamicFiles,
            '/styles.css': {
              code: CSS_PRELUDE + generatedCSS,
            },
          },
        });
      }
      return true;
    } catch (error: unknown) {
      if (error instanceof Error) {
        setError(error);
      } else {
        setError(new Error(String(error)));
      }
      return false;
    }
  }

  function handleEditorChange(value: string | undefined) {
    const updatedInputFiles = {
      ...inputFiles,
      [activeInputFile as string]: value || '',
    };

    const success = updateSandpack(updatedInputFiles);
    setInputFiles(updatedInputFiles, success);
  }

  const createFile = (fileKind: 'component' | 'stylex', name: string) => {
    const trimmedName = name.trim();
    if (!trimmedName || inputFiles[trimmedName]) {
      return;
    }
    const template =
      fileKind === 'component'
        ? createComponentTemplate(trimmedName)
        : createStylexVarsTemplate();
    const updatedInputFiles = {
      ...inputFiles,
      [trimmedName]: template,
    };

    const success = updateSandpack(updatedInputFiles);
    setInputFiles(updatedInputFiles, success);

    ensureMonacoModel(monaco, trimmedName, template);
  };

  const renameFile = (oldName: string, newName: string) => {
    const trimmedName = newName.trim();
    if (!trimmedName || trimmedName === oldName || inputFiles[trimmedName]) {
      return false;
    }
    const updatedInputFiles = Object.fromEntries(
      Object.entries(inputFiles as Record<string, string>).map(
        ([key, value]) =>
          key === oldName ? [trimmedName, value] : [key, value],
      ),
    );
    const success = updateSandpack(updatedInputFiles);
    setInputFiles(updatedInputFiles, success);

    ensureMonacoModel(
      monaco,
      trimmedName,
      updatedInputFiles[trimmedName] ?? '',
    );

    if (activeInputFile === oldName) {
      setActiveInputFile(trimmedName);
    }

    const oldModel = monaco.editor.getModel(monaco.Uri.file(`/${oldName}`));
    if (oldModel) {
      oldModel.dispose();
    }

    return true;
  };

  const deleteFile = (filename: string) => {
    if (Object.keys(inputFiles).length <= 1) {
      return false;
    }
    const updatedInputFiles = { ...inputFiles };
    delete updatedInputFiles[filename];
    setInputFiles(updatedInputFiles, false);
    if (activeInputFile === filename) {
      const nextActive = Object.keys(updatedInputFiles)[0];
      if (nextActive) {
        setActiveInputFile(nextActive);
      }
    }
    updateSandpack(updatedInputFiles);

    const oldModel = monaco.editor.getModel(monaco.Uri.file(`/${filename}`));
    if (oldModel) {
      oldModel.dispose();
    }

    return true;
  };

  useEffect(() => {
    let mounted = true;

    // The initial files come from the URL, so they are not guaranteed to
    // compile. Surface the failure the same way editing does, instead of
    // letting it escape the effect and take down the page. Sandpack still
    // has to load, otherwise the editor stays read only and there is no way
    // to correct the code.
    let transformedFiles: Record<string, string> = {};
    let generatedCSS = '';
    try {
      ({ transformedFiles, generatedCSS } = transformSourceFiles(inputFiles));
    } catch (err: unknown) {
      setError(err instanceof Error ? err : new Error(String(err)));
    }

    setTransformedFiles(transformedFiles);
    setCssOutput(generatedCSS);
    loadSandpackClient(
      iframeRef.current!,
      {
        files: {
          ...INITIAL_BUNDLER_FILES,
          ...Object.keys(transformedFiles).reduce(
            (acc, filename) => ({
              ...acc,
              [`/${filename.replace(/\.tsx?$/, '.js')}`]: {
                code: transformedFiles[filename],
              },
            }),
            {},
          ),
          '/styles.css': {
            code: CSS_PRELUDE + generatedCSS,
          },
        },
        template: 'react' as any,
      },
      {
        showOpenInCodeSandbox: false,
      },
    ).then((sandpackClient: any) => {
      if (!mounted) {
        sandpackClient.destroy();
        return;
      }
      sandpackClientRef.current = sandpackClient;
      const unsubscribe = sandpackClient.listen((msg: any) => {
        if (msg.type === 'done') {
          setSandpackInitialized(true);
          unsubscribe();
        }
      });
    });

    return () => {
      mounted = false;
      if (sandpackClientRef.current) {
        sandpackClientRef.current.destroy();
      }
    };
  }, []);

  useEffect(() => {
    if (monaco) {
      defineMonacoThemes(monaco);
    }
  }, [monaco]);

  const handleFormat = useCallback(async () => {
    if (!editorRef.current) return;

    const model = editorRef.current.getModel();
    if (!model) return;

    const formatted = await prettier.format(model.getValue(), {
      parser: 'babel',
      plugins: [estreePlugin as any, babelPlugin],
    });

    editorRef.current.executeEdits('format', [
      {
        range: model.getFullModelRange(),
        text: formatted,
      },
    ]);

    const updatedInputFiles = {
      ...inputFiles,
      [activeInputFile]: formatted,
    };

    setInputFiles(updatedInputFiles, updateSandpack(updatedInputFiles));
  }, [monaco, inputFiles, activeInputFile, setInputFiles]);

  return (
    <div {...stylex.props(styles.container)}>
      <div {...stylex.props(styles.frame)}>
        <div {...stylex.props(styles.row)}>
          <div {...stylex.props(styles.column)}>
            <Panel header="">
              <Tabs
                activeFile={activeInputFile as string}
                files={Object.keys(inputFiles)}
                getDefaultFilename={getDefaultFilename}
                onCreateFile={createFile}
                onDeleteFile={deleteFile}
                onFormat={handleFormat}
                onRenameFile={renameFile}
                onSelectFile={(filename) => setActiveInputFile(filename)}
              />
              <div {...stylex.props(styles.editorShell)}>
                <Editor
                  beforeMount={(monaco: any) => {
                    defineMonacoThemes(monaco);
                    monaco.languages.typescript.typescriptDefaults.setEagerModelSync(
                      true,
                    );

                    for (const [filename, content] of Object.entries(
                      inputFiles,
                    )) {
                      ensureMonacoModel(monaco, filename, content);
                    }

                    monaco.languages.typescript.typescriptDefaults.setCompilerOptions(
                      {
                        ...monaco.languages.typescript.typescriptDefaults.getCompilerOptions(),
                        jsx: monaco.languages.typescript.JsxEmit.ReactJSX,
                        moduleResolution:
                          monaco.languages.typescript.ModuleResolutionKind
                            .NodeJs,
                        paths: {
                          '@stylexjs/stylex': [
                            'file:///node_modules/@stylexjs/stylex/stylex.d.ts',
                          ],
                        },
                      },
                    );

                    for (const [file, content] of Object.entries(
                      STYLEX_TYPES,
                    )) {
                      monaco.languages.typescript.typescriptDefaults.addExtraLib(
                        content,
                        file,
                      );
                    }
                    monaco.languages.typescript.typescriptDefaults.addExtraLib(
                      REACT_TYPES,
                      'file:///node_modules/@types/react/index.d.ts',
                    );
                    monaco.languages.typescript.typescriptDefaults.addExtraLib(
                      REACT_JSX_RUNTIME_TYPES,
                      'file:///node_modules/@types/react/jsx-runtime.d.ts',
                    );
                    monaco.languages.typescript.typescriptDefaults.addExtraLib(
                      `import type { StyleXArray, CompiledStyles, InlineStyles } from '@stylexjs/stylex';
declare module 'react' {
  interface DOMAttributes<T> {
    sx?: StyleXArray<
      | (null | undefined | CompiledStyles)
      | boolean
      | Readonly<[CompiledStyles, InlineStyles]>
    >;
  }
}`,
                      'file:///react-augmentation.d.ts',
                    );
                  }}
                  defaultLanguage="typescript"
                  height="100%"
                  onChange={handleEditorChange}
                  onMount={(editor: any) => {
                    editorRef.current = editor;
                    editor.onKeyDown((e: any) => {
                      if (e.browserEvent.key === '/') {
                        e.browserEvent.stopPropagation();
                      }
                    });
                  }}
                  options={{
                    minimap: { enabled: false },
                    scrollBeyondLastLine: true,
                    contextmenu: false,
                    readOnly: !sandpackInitialized,
                    fontSize: 14,
                    lineHeight: 22,
                  }}
                  path={`/${activeInputFile}`}
                  theme={
                    colorMode === 'dark'
                      ? DARK_EDITOR_THEME
                      : LIGHT_EDITOR_THEME
                  }
                />
              </div>
              {error != null && (
                <div {...stylex.props(styles.error)}>{error.message}</div>
              )}
            </Panel>
            <div
              {...stylex.props(
                styles.panel,
                outputCollapsed && styles.panelClosed,
              )}
            >
              <div {...stylex.props(styles.outputContent)}>
                <Tabs
                  activeFile={
                    OUTPUT_TABS.find((t) => t.key === activeOutputTab)?.label ??
                    OUTPUT_TABS[0].label
                  }
                  files={OUTPUT_TABS.map((t) => t.label)}
                  hideFileIcon
                  isCollapsed={outputCollapsed}
                  onSelectFile={(label) => {
                    const found = OUTPUT_TABS.find((t) => t.label === label);
                    if (found) {
                      setActiveOutputTab(found.key);
                      setOutputCollapsed(false);
                    }
                  }}
                  onToggleCollapse={() => setOutputCollapsed((c) => !c)}
                  readOnly
                />
                {!outputCollapsed && (
                  <div {...stylex.props(styles.outputEditor)}>
                    <Editor
                      beforeMount={defineMonacoThemes}
                      defaultLanguage={
                        activeOutputTab === 'css' ? 'css' : 'javascript'
                      }
                      options={{
                        minimap: { enabled: false },
                        scrollBeyondLastLine: false,
                        contextmenu: false,
                        readOnly: true,
                        fontSize: 14,
                        lineHeight: 22,
                      }}
                      theme={
                        colorMode === 'dark'
                          ? DARK_EDITOR_THEME
                          : LIGHT_EDITOR_THEME
                      }
                      value={
                        activeOutputTab === 'css'
                          ? cssOutput
                          : transformedFiles[activeInputFile] || ''
                      }
                    />
                  </div>
                )}
              </div>
            </div>
          </div>
          <div {...stylex.props(styles.column)}>
            <CollapsiblePanel
              alwaysOpen={true}
              header="Preview"
              style={styles.previewPanel}
            >
              <iframe
                ref={iframeRef}
                title="StyleX Playground Preview"
                {...stylex.props(
                  styles.iframe,
                  !sandpackInitialized && styles.iframeHidden,
                )}
              />
            </CollapsiblePanel>
          </div>
        </div>
      </div>
    </div>
  );
}

function Panel({
  header,
  children,
  style,
}: {
  header: string;
  children: React.ReactNode;
  style?: stylex.StyleXStyles;
}) {
  return (
    <div {...stylex.props(styles.panel, style)}>
      {header && (
        <div {...stylex.props(styles.panelHeader)}>
          <span>{header}</span>
        </div>
      )}
      {children}
    </div>
  );
}

function CollapsiblePanel({
  header,
  children,
  style,
  alwaysOpen = false,
}: {
  header: string;
  children: React.ReactNode;
  style?: stylex.StyleXStyles;
  alwaysOpen?: boolean;
}) {
  const [open, setOpen] = useState(alwaysOpen || !header);

  return (
    <div
      {...stylex.props(
        styles.panel,
        !open && styles.panelClosed,
        alwaysOpen && !open && styles.panelClosedAlwaysOpen,
        style,
      )}
    >
      {header && (
        <button
          {...stylex.props(
            styles.panelHeader,
            alwaysOpen && styles.panelHeaderButtonAlwaysOpen,
          )}
          onClick={() => setOpen((open) => !open)}
        >
          {header}
          <ChevronDown
            {...stylex.props(
              styles.panelChevron,
              open && styles.panelChevronOpen,
            )}
          />
        </button>
      )}
      <div
        {...stylex.props(
          styles.panelContent,
          !open && styles.hidden,
          alwaysOpen && !open && styles.panelContentAlwaysOpen,
        )}
      >
        <div {...stylex.props(styles.panelContentInner)}>{children}</div>
      </div>
    </div>
  );
}

const styles = stylex.create({
  container: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    width: '100%',
    height: 'calc(100dvh - 64px)',
    padding: 10,
    containerType: 'inline-size',
  },
  frame: {
    boxSizing: 'border-box',
    width: '100%',
    height: '100%',
    padding: 0,
    overflow: 'hidden',
    backgroundColor: 'transparent',
    borderColor: playgroundVars['--pg-border'],
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: 20,
    cornerShape: 'squircle',
  },
  row: {
    display: 'flex',
    flexDirection: { default: 'row', '@container (width < 768px)': 'column' },
    gap: 0,
    width: '100%',
    height: '100%',
  },
  column: {
    display: { default: 'flex', '@container (width < 768px)': 'contents' },
    flexGrow: 1,
    flexShrink: 1,
    flexDirection: 'column',
    gap: 0,
    width: '50%',
    minWidth: 0,
  },
  panel: {
    position: 'relative',
    display: 'flex',
    flexGrow: 1,
    flexShrink: 0,
    flexBasis: 42,
    flexDirection: 'column',
    overflow: 'hidden',
    backgroundColor: playgroundVars['--pg-panel-surface'],
    borderColor: playgroundVars['--pg-border'],
    borderStyle: 'solid',
    borderWidth: 0,
    borderRadius: 0,
    boxShadow: `0 0 0 1px ${playgroundVars['--pg-panel-shadow']}`,
  },
  panelClosed: {
    flexGrow: 0,
  },
  panelClosedAlwaysOpen: {
    flexGrow: {
      default: 1,
      '@container (width < 768px)': 0,
    },
  },
  panelHeader: {
    position: 'relative',
    display: 'block',
    flexShrink: 0,
    width: '100%',
    height: 46,
    paddingBlock: 8,
    paddingInline: 16,
    fontSize: 16,
    fontWeight: 500,
    color: vars['--color-fd-foreground'],
    textAlign: 'start',
    appearance: 'none',
    backgroundColor: playgroundVars['--pg-header-surface'],
    borderStyle: 'none',
    borderBottomColor: vars['--color-fd-border'],
    borderBottomStyle: 'solid',
    borderBottomWidth: 1,
  },
  panelHeaderButtonAlwaysOpen: {
    display: {
      default: 'none',
      '@container (width < 768px)': 'block',
    },
  },
  panelChevron: {
    position: 'absolute',
    insetInlineEnd: 4,
    top: 0,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    height: '100%',
    aspectRatio: 1,
    paddingBottom: 4,
    transformOrigin: 'center',
    rotate: '-90deg',
    transitionTimingFunction: 'ease-in-out',
    transitionDuration: '0.3s',
    transitionProperty: 'rotate',
  },
  panelChevronOpen: {
    rotate: '0deg',
  },
  panelContent: {
    display: 'grid',
    gridTemplateRows: '1fr',
    height: '100%',
  },
  panelContentInner: {
    height: '100%',
    overflow: 'hidden',
  },
  editorShell: {
    display: 'flex',
    flexGrow: 1,
    minHeight: 0,
  },
  panelContentAlwaysOpen: {
    display: {
      default: 'block',
      '@container (width < 768px)': 'none',
    },
  },
  previewPanel: {
    backgroundColor: playgroundVars['--pg-preview'],
  },
  hidden: {
    gridTemplateRows: '0fr',
  },
  iframe: {
    flexGrow: 1,
    flexShrink: 1,
    width: '100%',
    height: '100%',
    color: 'var(--fg1)',
    outline: 'none',
    backgroundColor: playgroundVars['--pg-preview'],
    opacity: 1,
    transitionDuration: '0.2s',
    transitionProperty: 'opacity',
  },
  iframeHidden: {
    opacity: 0,
  },
  error: {
    position: 'absolute',
    right: 0,
    bottom: 0,
    left: 0,
    zIndex: 1,
    maxHeight: 'min(160px, 20vh)',
    padding: 8,
    overflow: 'auto',
    color: 'white',
    backgroundColor: 'red',
  },
  outputContent: {
    display: 'flex',
    flexDirection: 'column',
    height: '100%',
    boxShadow: `inset 0 0 12px color-mix(in srgb, ${vars['--color-fd-primary']} 25%, transparent)`,
  },
  outputEditor: {
    flexGrow: 1,
    flexShrink: 1,
    minHeight: 0,
  },
});
