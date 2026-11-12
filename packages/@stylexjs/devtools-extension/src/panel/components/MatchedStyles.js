/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import * as React from 'react';
import { useMemo } from 'react';
import * as stylex from '@stylexjs/stylex';

import type {
  OverrideCommand,
  StylexDebugData,
  StylexDeclaration,
} from '../../types';
import { formatCssValue, parseCssValue } from '../../utils/css';
import {
  conditionsAreActive,
  findDisplayedOverride,
  findSuggestion,
  formatConditions,
  getReplacementOverrideIds,
  getSuggestionValues,
  groupDeclarations,
} from '../declarations/model';
import { colors } from '../theme.stylex';
import { ValueEditor } from './ValueEditor';

export function MatchedStyles({
  data,
  onMutate,
}: {
  data: StylexDebugData,
  onMutate: (command: OverrideCommand) => Promise<void>,
}): React.Node {
  const groups = useMemo(
    () => groupDeclarations(data.matched.classes),
    [data.matched.classes],
  );
  if (groups.length === 0) {
    return (
      <div {...stylex.props(styles.muted)}>
        No matching StyleX CSS rules were found.
      </div>
    );
  }

  return (
    <div {...stylex.props(styles.groups)}>
      {groups.map(({ pseudoElement, properties, variableClasses }) => (
        <div
          {...stylex.props(styles.pseudoGroup)}
          key={pseudoElement || 'base'}
        >
          {pseudoElement !== '' ? (
            <div {...stylex.props(styles.pseudoTitle)}>{pseudoElement}</div>
          ) : null}
          {properties.map(({ property, declarations }) => (
            <PropertyGroup
              computedValue={data.computed[pseudoElement]?.[property]}
              data={data}
              declarations={declarations}
              key={property}
              onMutate={onMutate}
              property={property}
            />
          ))}
          {variableClasses.map(
            ({ className, properties: variableProperties }) => (
              <div
                {...stylex.props(styles.variableClassGroup)}
                data-variable-class={className}
                key={className}
              >
                {variableProperties.map(({ property, declarations }, index) => (
                  <PropertyGroup
                    classNameLabel={index === 0 ? className : undefined}
                    computedValue={data.computed[pseudoElement]?.[property]}
                    data={data}
                    declarations={declarations}
                    groupedVariable
                    key={property}
                    onMutate={onMutate}
                    property={property}
                    suppressDeclarationClassName
                  />
                ))}
              </div>
            ),
          )}
        </div>
      ))}
    </div>
  );
}

function PropertyGroup({
  classNameLabel,
  computedValue,
  data,
  declarations,
  groupedVariable = false,
  onMutate,
  property,
  suppressDeclarationClassName = false,
}: {
  classNameLabel?: string,
  computedValue?: string,
  data: StylexDebugData,
  declarations: $ReadOnlyArray<StylexDeclaration>,
  groupedVariable?: boolean,
  onMutate: (command: OverrideCommand) => Promise<void>,
  property: string,
  suppressDeclarationClassName?: boolean,
}): React.Node {
  const propertyPrefix = (
    <span
      {...stylex.props(styles.propertyPrefix)}
      title={computedValue?.trim() || undefined}
    >
      <span {...stylex.props(styles.property)}>{property}</span>:
    </span>
  );

  if (declarations.length === 1) {
    return (
      <div
        {...stylex.props(
          styles.propertyGroup,
          groupedVariable && styles.groupedVariableProperty,
        )}
      >
        <DeclarationRow
          data={data}
          declaration={declarations[0]}
          displayClassName={
            classNameLabel ??
            (suppressDeclarationClassName
              ? undefined
              : declarations[0].className)
          }
          onMutate={onMutate}
          prefix={propertyPrefix}
        />
      </div>
    );
  }

  return (
    <div
      {...stylex.props(
        styles.propertyGroup,
        groupedVariable && styles.groupedVariableProperty,
      )}
    >
      <div {...stylex.props(styles.propertyHeading)}>{propertyPrefix}</div>
      <div {...stylex.props(styles.conditionList)}>
        {declarations.map((declaration, index) => (
          <DeclarationRow
            data={data}
            declaration={declaration}
            displayClassName={
              index === 0 && classNameLabel != null
                ? classNameLabel
                : suppressDeclarationClassName
                  ? undefined
                  : declaration.className
            }
            key={declaration.key}
            onMutate={onMutate}
            prefix={<ConditionPrefix declaration={declaration} />}
          />
        ))}
      </div>
    </div>
  );
}

function ConditionPrefix({
  declaration,
}: {
  declaration: StylexDeclaration,
}): React.Node {
  const conditionLabel = formatConditions(declaration.conditions);
  const conditionState = conditionsAreActive(declaration.conditions);

  return (
    <span
      title={
        conditionState === false
          ? 'This condition is currently inactive.'
          : undefined
      }
    >
      <span
        {...stylex.props(
          styles.condition,
          conditionState === false && styles.inactive,
        )}
      >
        {conditionLabel}
      </span>
      :
    </span>
  );
}

function DeclarationRow({
  data,
  declaration,
  displayClassName,
  onMutate,
  prefix,
}: {
  data: StylexDebugData,
  declaration: StylexDeclaration,
  displayClassName?: string,
  onMutate: (command: OverrideCommand) => Promise<void>,
  prefix: React.Node,
}): React.Node {
  const suggestions = data.suggestions[declaration.contextKey] ?? [];
  const displayedOverride = findDisplayedOverride(declaration, data.overrides);
  const displayValue = formatCssValue(
    displayedOverride?.value ?? declaration.value,
    displayedOverride?.important ?? declaration.important,
  );
  const commit = async (rawValue: string) => {
    const parsed = parseCssValue(rawValue);
    const formatted = formatCssValue(parsed.value, parsed.important);
    const suggestion = findSuggestion(suggestions, formatted);
    const common = {
      selectionId: data.selectionId,
      contextKey: declaration.contextKey,
      property: declaration.property,
      value: parsed.value,
      important: parsed.important,
      conditions: declaration.conditions,
      sourceEntryKey: declaration.key,
      replaceOverrideIds: getReplacementOverrideIds(
        declaration,
        data.overrides,
      ),
    };

    if (suggestion != null) {
      await onMutate({
        ...common,
        type: 'swap-class',
        fromClassName: declaration.className,
        toClassName: suggestion.className,
        ...(declaration.pseudoElement != null
          ? { pseudoElement: declaration.pseudoElement }
          : {}),
      });
    } else if (declaration.pseudoElement != null) {
      await onMutate({
        ...common,
        type: 'set-rule',
        pseudoElement: declaration.pseudoElement,
      });
    } else {
      await onMutate({ ...common, type: 'set-inline' });
    }
  };

  return (
    <div {...stylex.props(styles.row)}>
      <div {...stylex.props(styles.prefixCell)} data-declaration-part="prefix">
        {prefix}
      </div>
      <div {...stylex.props(styles.valueCell)} data-declaration-part="value">
        <ValueEditor
          onCommit={commit}
          resolvedVariables={data.resolvedVariables}
          suggestions={getSuggestionValues(suggestions)}
          value={displayValue}
        />
      </div>
      {displayClassName != null ? (
        <span {...stylex.props(styles.className)} data-declaration-part="class">
          {displayClassName}
        </span>
      ) : null}
    </div>
  );
}

const styles = stylex.create({
  muted: { color: colors.textMuted },
  groups: { containerType: 'inline-size', display: 'grid', gap: 12 },
  pseudoGroup: { display: 'grid', gap: 7 },
  pseudoTitle: {
    color: colors.textMuted,
    fontFamily:
      'ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace',
    fontWeight: 600,
  },
  propertyGroup: {
    borderBottomColor: colors.separator,
    borderBottomStyle: 'solid',
    borderBottomWidth: 1,
    display: 'grid',
    gap: 4,
    paddingBlock: 5,
  },
  variableClassGroup: {
    borderBottomColor: colors.separator,
    borderBottomStyle: 'solid',
    borderBottomWidth: 1,
    display: 'grid',
    gap: 4,
    paddingBlock: 5,
  },
  groupedVariableProperty: {
    borderBottomWidth: 0,
    paddingBlock: 0,
  },
  propertyHeading: {
    fontFamily:
      'ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace',
    minWidth: 0,
    textWrap: 'nowrap',
    whiteSpace: 'nowrap',
  },
  conditionList: { display: 'grid', gap: 2, paddingInlineStart: 12 },
  row: {
    alignItems: 'start',
    columnGap: 6,
    display: 'grid',
    fontFamily:
      'ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace',
    gridTemplateAreas: {
      default: '"prefix value className"',
      '@container (width < 520px)': '"prefix className" "value value"',
    },
    gridTemplateColumns: {
      default: 'max-content minmax(0, 1fr) max-content',
      '@container (width < 520px)': 'minmax(0, 1fr) max-content',
    },
    minWidth: 0,
    rowGap: {
      default: 0,
      '@container (width < 520px)': 2,
    },
  },
  prefixCell: {
    gridArea: 'prefix',
    minWidth: 0,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    textWrap: 'nowrap',
    whiteSpace: 'nowrap',
  },
  propertyPrefix: {
    textWrap: 'nowrap',
    whiteSpace: 'nowrap',
  },
  property: { color: colors.textAccent },
  condition: {
    color: colors.secondaryAccent,
  },
  inactive: { opacity: 0.55, textDecoration: 'line-through' },
  valueCell: { gridArea: 'value', minWidth: 0 },
  className: {
    alignSelf: 'start',
    color: colors.textMuted,
    fontFamily:
      'ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace',
    gridArea: 'className',
    justifySelf: 'end',
    maxWidth: '16ch',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
    '::before': { content: '.' },
  },
});
