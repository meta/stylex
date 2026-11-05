export type Rule = [
  string,
  {
    ltr: string;
    rtl: string | null;
    constKey?: string;
    constVal?: string | number;
  },
  number,
];
export type ModuleResolution =
  | { type: 'haste'; themeFileExtension?: string }
  | {
      type: 'commonJS' | 'experimental_crossFileParsing';
      rootDir?: string;
      themeFileExtension?: string;
    }
  | {
      type: 'custom';
      themeFileExtension?: string;
      filePathResolver(
        importPath: string,
        sourceFilePath: string,
        aliases?: Record<string, readonly string[]>,
      ): string | void;
      getCanonicalFilePath(filePath: string): string;
    };
export interface CompilerOptions {
  filename?: string;
  sourceMaps?: boolean;
  classNamePrefix?: string;
  dev?: boolean;
  debug?: boolean;
  enableDebugDataProp?: boolean;
  enableDevClassNames?: boolean;
  enableInlinedConditionalMerge?: boolean;
  enableMediaQueryOrder?: boolean;
  enableLegacyValueFlipping?: boolean;
  enableLogicalStylesPolyfill?: boolean;
  enableFontSizePxToRem?: boolean;
  enableMinifiedKeys?: boolean;
  sxPropName?: string | false;
  styleResolution?:
    | 'property-specificity'
    | 'application-order'
    | 'legacy-expand-shorthands';
  propertyValidationMode?: 'throw' | 'warn' | 'silent';
  env?: Readonly<Record<string, unknown>>;
  aliases?: Record<string, string | readonly string[]>;
  importSources?: readonly (string | { from: string; as: string })[];
  runtimeInjection?: boolean | string | { from: string; as: string };
  rewriteAliases?: boolean;
  treeshakeCompensation?: boolean;
  unstable_moduleResolution?: ModuleResolution;
  debugFilePath?: (filename: string) => string;
  test?: boolean;
  [option: string]: unknown;
}
export interface SourceMap {
  version: 3;
  sources: string[];
  sourcesContent: (string | null)[];
  names: string[];
  mappings: string;
}
export interface TransformResult {
  code: string;
  map: SourceMap | null;
  metadata: { stylex: Rule[] };
  warnings: string[];
  errors: string[];
}
export interface ProcessOptions {
  useLayers?:
    | boolean
    | {
        before?: readonly string[];
        after?: readonly string[];
        prefix?: string;
      };
  enableLTRRTLComments?: boolean;
  legacyDisableLayers?: boolean;
  useLegacyClassnamesSort?: boolean;
}
export function transformSync(
  source: string,
  options?: CompilerOptions,
): TransformResult;
export function transform(
  source: string,
  options?: CompilerOptions,
): Promise<TransformResult>;
export function processStylexRules(
  rules: readonly Rule[],
  options?: ProcessOptions | boolean,
): string;
