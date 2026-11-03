/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import * as React from 'react';
import { useEffect, useRef, useState, type SVGProps } from 'react';
import * as stylex from '@stylexjs/stylex';
import { Menu, Item } from './Menu';
import { ConfirmDialog } from './Dialogs';
import { vars, playgroundVars } from '@/theming/vars.stylex';
import { ChevronDownIcon } from 'lucide-react';

export function Tabs({
  files,
  activeFile,
  getDefaultFilename,
  onSelectFile,
  onCreateFile,
  onDeleteFile,
  onRenameFile,
  onFormat,
  readOnly = false,
  hideFileIcon = false,
  isCollapsed,
  onToggleCollapse,
}: {
  files: string[];
  activeFile: string;
  getDefaultFilename?: (_fileKind: string) => string;
  onSelectFile: (_filename: string) => void;
  onCreateFile?: (_fileKind: 'component' | 'stylex', _name: string) => void;
  onDeleteFile?: (_filename: string) => void;
  onRenameFile?: (_filename: string, _newName: string) => boolean | void;
  onFormat?: () => void;
  readOnly?: boolean;
  hideFileIcon?: boolean;
  isCollapsed?: boolean;
  onToggleCollapse?: () => void;
}) {
  const [renamingFile, setRenamingFile] = useState<string | null>(null);
  const [isOverflowing, setIsOverflowing] = useState(false);
  const scrollableRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = scrollableRef.current;
    if (!el) return;

    const checkOverflow = () => {
      setIsOverflowing(el.scrollWidth > el.clientWidth);
    };

    checkOverflow();
    const observer = new ResizeObserver(checkOverflow);
    observer.observe(el);

    return () => observer.disconnect();
  }, [files]);

  const handleCreateFile = (fileKind: 'component' | 'stylex') => {
    if (!onCreateFile || !getDefaultFilename) return;
    const defaultName = getDefaultFilename(fileKind);
    onCreateFile(fileKind, defaultName);
    onSelectFile(defaultName);
    setRenamingFile(defaultName);
  };

  const showNewFileButton = !readOnly && onCreateFile && getDefaultFilename;

  return (
    <div
      onClick={onToggleCollapse}
      role="tablist"
      {...stylex.props(styles.tabs)}
    >
      <div ref={scrollableRef} {...stylex.props(styles.tabsScrollable)}>
        {files.map((filename, i) => (
          <Tab
            filename={filename}
            hideFileIcon={hideFileIcon}
            immutable={readOnly || i === 0}
            isActive={activeFile === filename}
            key={filename}
            onDelete={readOnly ? undefined : onDeleteFile}
            onRename={readOnly ? undefined : onRenameFile}
            onRenameComplete={() => setRenamingFile(null)}
            onSelect={(e) => {
              e.stopPropagation();
              onSelectFile(filename);
            }}
            readOnly={readOnly}
            startInRenameMode={renamingFile === filename}
          />
        ))}
        {showNewFileButton && !isOverflowing && (
          <NewFileButton onCreateFile={handleCreateFile} />
        )}
      </div>

      <div {...stylex.props(styles.tabsActions)}>
        {showNewFileButton && isOverflowing && (
          <NewFileButton onCreateFile={handleCreateFile} />
        )}
        {!readOnly && onFormat ? (
          <button
            {...stylex.props(styles.tabIconButton)}
            onClick={(e) => {
              e.stopPropagation();
              onFormat();
            }}
            title="Format file"
            type="button"
          >
            <SparklesIcon />
          </button>
        ) : null}
        {!readOnly && <ShareButton />}
        {onToggleCollapse && (
          <div {...stylex.props(styles.collapseIndicator)}>
            <ChevronDownIcon
              {...stylex.props(
                styles.chevron,
                isCollapsed && styles.chevronCollapsed,
              )}
            />
          </div>
        )}
      </div>
    </div>
  );
}

function SparklesIcon() {
  return (
    <svg
      fill="none"
      height="14"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.5"
      viewBox="0 0 24 24"
      width="14"
    >
      <path d="M11.017 2.814a1 1 0 0 1 1.966 0l1.051 5.558a2 2 0 0 0 1.594 1.594l5.558 1.051a1 1 0 0 1 0 1.966l-5.558 1.051a2 2 0 0 0-1.594 1.594l-1.051 5.558a1 1 0 0 1-1.966 0l-1.051-5.558a2 2 0 0 0-1.594-1.594l-5.558-1.051a1 1 0 0 1 0-1.966l5.558-1.051a2 2 0 0 0 1.594-1.594z" />
      <path d="M20 2v4" />
      <path d="M22 4h-4" />
      <circle cx="4" cy="20" r="2" />
    </svg>
  );
}

function NewFileIcon() {
  return (
    <svg
      fill="none"
      height="14"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.5"
      viewBox="0 0 24 24"
      width="14"
    >
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <path d="M14 2v6h6" />
      <path d="M12 11v6" />
      <path d="M9 14h6" />
    </svg>
  );
}

function ShareButton() {
  const [copied, setCopied] = useState(false);

  const handleShare = async (e: React.MouseEvent) => {
    e.stopPropagation();
    const url = window.location.href;
    try {
      await navigator.clipboard.writeText(url);
    } catch {
      const textArea = document.createElement('textarea');
      textArea.value = url;
      textArea.style.cssText = 'position:fixed;left:-9999px';
      document.body.appendChild(textArea);
      textArea.select();
      document.execCommand('copy');
      document.body.removeChild(textArea);
    }
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <button
      {...stylex.props(styles.tabIconButton)}
      onClick={handleShare}
      title={copied ? 'Copied!' : 'Copy link to share'}
      type="button"
    >
      {copied ? <CheckIcon /> : <ShareIcon />}
    </button>
  );
}

function ShareIcon() {
  return (
    <svg
      fill="none"
      height="14"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.5"
      viewBox="0 0 24 24"
      width="14"
    >
      <path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8" />
      <polyline points="16 6 12 2 8 6" />
      <line x1="12" x2="12" y1="2" y2="15" />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg
      fill="none"
      height="14"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.5"
      viewBox="0 0 24 24"
      width="14"
    >
      <polyline points="20 6 9 17 4 12" />
    </svg>
  );
}

function Tab({
  filename,
  isActive,
  onSelect,
  onRename,
  onDelete,
  immutable = false,
  readOnly = false,
  hideFileIcon = false,
  startInRenameMode = false,
  onRenameComplete,
}: {
  filename: string;
  isActive: boolean;
  onSelect: (_e: React.MouseEvent) => void;
  onRename?: (_filename: string, _newName: string) => boolean | void;
  onDelete?: (_filename: string) => void;
  immutable: boolean;
  readOnly?: boolean;
  hideFileIcon?: boolean;
  startInRenameMode?: boolean;
  onRenameComplete?: () => void;
}) {
  const deleteDialogRef = useRef<HTMLDialogElement | null>(null);
  const renameTimerRef = useRef<number | null>(null);

  const [isRenaming, setIsRenaming] = useState(startInRenameMode);
  const [draftName, setDraftName] = useState(filename);

  useEffect(() => {
    setDraftName(filename);
  }, [filename]);

  const cancelRename = () => {
    setIsRenaming(false);
    setDraftName(filename);
    onRenameComplete?.();
  };

  const commitRename = () => {
    if (!onRename) {
      cancelRename();
      return;
    }
    const trimmed = draftName.trim();
    if (!trimmed || trimmed === filename) {
      cancelRename();
      return;
    }
    if (onRename(filename, trimmed) === false) {
      cancelRename();
      return;
    }
    setIsRenaming(false);
    onRenameComplete?.();
  };

  const startLongPress = () => {
    if (immutable || readOnly || !onRename) return;
    renameTimerRef.current = window.setTimeout(() => {
      setIsRenaming(true);
    }, 500);
  };

  const clearLongPress = () => {
    if (renameTimerRef.current != null) {
      clearTimeout(renameTimerRef.current);
      renameTimerRef.current = null;
    }
  };

  return (
    <div {...stylex.props(styles.tab, isActive && styles.tabActive)}>
      <button
        aria-selected={isActive}
        onClick={onSelect}
        onDoubleClick={() =>
          !immutable && !readOnly && onRename && setIsRenaming(true)
        }
        onPointerDown={startLongPress}
        onPointerLeave={clearLongPress}
        onPointerMove={clearLongPress}
        onPointerUp={clearLongPress}
        role="tab"
        title={filename}
        {...stylex.props(styles.tabLabelButton)}
      >
        {!hideFileIcon &&
          (filename.includes('.stylex.') ? (
            <StyleXIcon
              {...stylex.props(
                styles.fileIcon,
                !isActive && styles.fileIconInactive,
              )}
            />
          ) : (
            <ReactIcon
              {...stylex.props(
                styles.fileIcon,
                !isActive && styles.fileIconInactive,
              )}
            />
          ))}

        <span {...stylex.props(styles.filenameBox)}>
          {isRenaming && !readOnly && onRename ? (
            <>
              <span {...stylex.props(styles.renameMirror)}>
                {draftName || ' '}
              </span>
              <input
                autoFocus
                onBlur={commitRename}
                onChange={(e) => setDraftName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    commitRename();
                  } else if (e.key === 'Escape') {
                    e.preventDefault();
                    cancelRename();
                  }
                }}
                value={draftName}
                {...stylex.props(styles.renameInputOverlay)}
              />
            </>
          ) : (
            filename
          )}
        </span>
      </button>

      {!immutable && !readOnly && onDelete && (
        <>
          <button
            {...stylex.props(styles.tabCloseButton)}
            onClick={(e) => {
              e.stopPropagation();
              deleteDialogRef.current?.showModal();
            }}
            title="Close file"
            type="button"
          >
            ×
          </button>
          <ConfirmDialog
            description={`Are you sure you want to delete ${filename}?`}
            onCancel={() => deleteDialogRef.current?.close()}
            onConfirm={() => onDelete(filename)}
            ref={deleteDialogRef}
            title="Delete file"
          />
        </>
      )}
    </div>
  );
}

function NewFileButton({
  onCreateFile,
}: {
  onCreateFile: (_fileKind: 'component' | 'stylex') => void;
}) {
  const id = 'new-file';

  return (
    <>
      <button
        {...stylex.props(styles.tabIconButton)}
        onClick={(e) => e.stopPropagation()}
        popoverTarget={id}
        title="Add file"
        type="button"
      >
        <NewFileIcon />
      </button>
      <Menu id={id}>
        <Item onClick={() => onCreateFile('component')}>Component file</Item>
        <Item onClick={() => onCreateFile('stylex')}>Vars file</Item>
      </Menu>
    </>
  );
}

const styles = stylex.create({
  tabs: {
    position: 'relative',
    display: 'flex',
    alignItems: 'center',
    backgroundColor: playgroundVars['--pg-header-surface'],
    borderBottomColor: playgroundVars['--pg-tabs-border'],
    borderBottomStyle: 'solid',
    borderBottomWidth: 1,
  },

  tabsScrollable: {
    display: 'flex',
    flexGrow: 1,
    flexShrink: 1,
    minWidth: 0,
    paddingInlineStart: 8,
    overflowX: 'auto',
    scrollbarWidth: 'none',
    '::-webkit-scrollbar': {
      display: 'none',
    },
  },

  tabsActions: {
    position: 'relative',
    display: 'flex',
    flexShrink: 0,
    alignItems: 'center',
    backgroundColor: playgroundVars['--pg-header-surface'],
  },

  tab: {
    display: 'flex',
    color: {
      default: vars['--color-fd-muted-foreground'],
      ':hover': `color-mix(in srgb, ${vars['--color-fd-foreground']} 80%, transparent)`,
    },
    backgroundColor: 'transparent',
    borderStyle: 'none',
  },

  tabActive: {
    color: vars['--color-fd-foreground'],
    boxShadow: `0 -2px 0 0 ${vars['--color-fd-primary']} inset`,
  },

  tabLabelButton: {
    display: 'inline-flex',
    gap: 6,
    alignItems: 'center',
    paddingBlock: 15,
    paddingInline: 8,
    fontSize: 16,
    fontStyle: 'inherit',
    fontWeight: 'inherit',
    color: 'inherit',
    whiteSpace: 'nowrap',
    backgroundColor: 'transparent',
    borderStyle: 'none',
  },

  filenameBox: {
    position: 'relative',
    display: 'inline-flex',
    alignItems: 'center',
    whiteSpace: 'pre',
  },

  renameMirror: {
    fontSize: 13,
    lineHeight: 1.4,
    color: 'transparent',
    whiteSpace: 'pre',
    pointerEvents: 'none',
  },

  renameInputOverlay: {
    position: 'absolute',
    inset: 0,
    width: '100%',
    padding: 0,
    fontSize: 13,
    lineHeight: 1.4,
    color: 'inherit',
    outline: 'none',
    backgroundColor: 'transparent',
    borderStyle: 'none',
  },

  fileIcon: {
    display: 'inline-flex',
    minWidth: 12,
  },

  fileIconInactive: {
    opacity: 0.6,
  },

  tabIconButton: {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    width: 42,
    height: 42,
    padding: 4,
    fontSize: 14,
    lineHeight: 1,
    color: vars['--color-fd-foreground'],
    backgroundColor: {
      default: 'transparent',
      ':hover': `color-mix(in srgb, ${vars['--color-fd-foreground']} 8%, transparent)`,
    },
    borderStyle: 'none',
    borderRadius: 4,
  },

  tabCloseButton: {
    paddingBlock: 10,
    paddingRight: 8,
    fontSize: 14,
    lineHeight: 1,
    color: {
      default: vars['--color-fd-muted-foreground'],
      ':hover': vars['--color-fd-foreground'],
    },

    backgroundColor: 'transparent',
    borderStyle: 'none',
  },

  collapseIndicator: {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    width: 28,
    height: 28,
    marginInlineEnd: 4,
    color: vars['--color-fd-foreground'],
  },

  chevron: {
    width: 24,
    height: 24,
    transform: 'rotate(0deg)',
    transitionTimingFunction: 'ease',
    transitionDuration: '0.3s',
    transitionProperty: 'transform',
  },

  chevronCollapsed: {
    transform: 'rotate(180deg)',
  },
});

function ReactIcon(props: SVGProps<SVGSVGElement>) {
  const color = 'light-dark(#0891b2, #61DAFB)';
  return (
    <svg height="14" viewBox="-11.5 -10.232 23 20.463" width="14" {...props}>
      <circle cx="0" cy="0" fill={color} r="2.05" />
      <g fill="none" stroke={color} strokeWidth="1">
        <ellipse rx="11" ry="4.2" />
        <ellipse rx="11" ry="4.2" transform="rotate(60)" />
        <ellipse rx="11" ry="4.2" transform="rotate(120)" />
      </g>
    </svg>
  );
}

function StyleXIcon(props: SVGProps<SVGSVGElement>) {
  return (
    <svg height="14" viewBox="0 0 180 180" width="14" {...props}>
      <defs>
        <linearGradient id="stylex-grad-1" x1="41%" x2="74%" y1="32%" y2="60%">
          <stop offset="0%" stopColor="#5BD3F3" />
          <stop offset="100%" stopColor="#5BD3F3" />
        </linearGradient>
        <linearGradient id="stylex-grad-2" x1="42%" x2="65%" y1="56%" y2="39%">
          <stop offset="0%" stopColor="#D573DE" stopOpacity="0" />
          <stop offset="100%" stopColor="#D573DE" />
        </linearGradient>
      </defs>
      <g fillRule="nonzero">
        <path
          d="M123.054863,93.4254443 C124.041858,95.7626109 128.450105,105.044084 129.355779,107.321152 C123.84289,116.561307 122.549601,118.95899 111.024753,133.60593 C64.1232983,182.705627 27.9371992,190.639891 5.76263041,167.701852 C3.59627766,165.361764 1.67512566,162.319274 0,158.574382 C0.471825684,159.433291 1.09514745,160.379843 1.86996531,161.414039 L2.15025371,161.78256 C2.19772524,161.844746 2.24602235,161.906931 2.29473227,161.969534 L2.59359648,162.349323 L2.90484457,162.735791 L3.22888933,163.12977 L3.56531797,163.530845 L3.91454328,163.939431 L4.27615246,164.355113 L4.65014553,164.77789 L5.03734806,165.208179 L5.23549007,165.426036 L5.64126842,165.867176 L5.84890474,166.090459 L6.2732589,166.542451 L6.49038953,166.771159 C33.8818726,191.84228 61.2048315,170.332834 98.3027967,128.773838 C103.902786,122.190123 112.153337,110.407464 123.054863,93.4254443 Z M137.380118,14.1032604 C154.739423,29.1884191 154.739423,52.5968124 141.717364,86.0295639 C140.719637,83.5713654 136.323774,73.7444144 135.221609,71.226952 C145.472981,42.8320467 145.710752,29.3332399 130.967334,15.8715774 C122.485617,8.12762615 116.462513,7.80876984 104.995043,9.69477985 L104.244168,9.82123726 C104.118678,9.84252217 103.992775,9.86464178 103.866872,9.8867614 L103.107328,10.0236526 L102.342004,10.1663867 L101.956866,10.2402579 L92.9145722,12.0507273 L92.9145722,12.0340333 L93.1139526,11.9605794 C111.260459,5.27670019 126.843916,4.74249067 137.380118,14.1032604 L137.380118,14.1032604 Z"
          fill="url(#stylex-grad-1)"
        />
        <path
          d="M125.890167,63.5141248 C153.449324,115.583313 155.188797,143.75817 146.009025,163.468062 C142.702042,170.570383 134.455253,175.478804 130.907687,177.387749 C122.003636,182.178957 103.568032,179.793293 87.0876824,174.955283 L84.6173661,173.901615 C92.8984649,176.570162 110.89548,180.056296 120.598168,177.387749 C152.463016,168.623747 148.671973,130.669324 116.64467,71.0621007 C84.6173661,11.4548774 49.5757474,-4.8960329 21.9537585,6.3426811 C19.3015581,7.42161421 16.9891503,8.8960871 15,10.7226111 L15.212887,10.4952275 L15.6399012,10.0462588 L15.853615,9.82508786 L16.282696,9.38854448 C19.3635641,6.29215141 22.5576963,3.87542408 25.8493845,2.76294257 C50.8282672,-5.6788289 93.7099159,2.71324123 125.890167,63.5141248 Z"
          fill="url(#stylex-grad-2)"
        />
      </g>
    </svg>
  );
}
