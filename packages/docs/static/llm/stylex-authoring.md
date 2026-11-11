# StyleX Authoring Guide

This document provides guidance on authoring styles with StyleX.

## Writing styles

Styles must be created using `stylex.create()`. Define styles as an object with namespaces containing CSS properties.

```tsx
import * as stylex from '@stylexjs/stylex';

const styles = stylex.create({
  container: {
    display: 'flex',
    alignItems: 'center',
    padding: 16,
  },
  title: {
    fontSize: 24,
    fontWeight: 'bold',
    color: 'navy',
  },
});
```

**IMPORTANT**
- Use longhand properties and single-value shorthands over multi-value shorthands.
- Use `null` to unset properties.
- Length properties are in pixels by default.

---

## Applying styles

Convert StyleX style objects to props using `stylex.props()`:

```tsx
function Component() {
  return (
    <div {...stylex.props(styles.container)}>
      <h1 {...stylex.props(styles.title)}>Hello</h1>
    </div>
  );
}
```

### Merging styles

Pass multiple styles to merge them. The last style wins for conflicting properties:

```tsx
// styles.highlighted overrides conflicting properties from styles.base
<div {...stylex.props(styles.base, styles.highlighted)} />

// Or passed in as arrays
<div {...stylex.props([styles.base, styles.highlighted])} />
```

### Conditional styles

Use JavaScript expressions for conditional styling:

```tsx
<div
  {...stylex.props(
    styles.base,
    isActive && styles.active,
    isDisabled && styles.disabled,
    variant === 'primary' ? styles.primary : styles.secondary,
  )}
/>
```

### Passing styles as props

Accept styles from parent components:

```tsx
import type { StyleXStyles } from '@stylexjs/stylex';

type Props = {
  children: React.ReactNode;
  style?: StyleXStyles;
};

const styles = stylex.create({
  card: {
    padding: 16,
    borderRadius: 8,
  },
});

function Card({ children, style }: Props) {
  // Local styles first, then prop styles (so props can override)
  return <div {...stylex.props(styles.card, style)}>{children}</div>;
}
```

### Unsetting styles

Use `null` to remove a style property:

```tsx
const styles = stylex.create({
  base: { margin: 16, padding: 16 },
  reset: { margin: null, padding: null }, // Removes margin and padding
});

<div {...stylex.props(styles.base, styles.reset)} />
```

---

## Pseudo-classes

Nest pseudo-classes within property values using an object with `default` and pseudo-class keys:

```tsx
const styles = stylex.create({
  button: {
    backgroundColor: {
      default: 'lightblue',
      ':hover': 'blue',
      ':active': 'darkblue',
      ':focus-visible': 'royalblue',
      ':disabled': 'gray',
    },
    cursor: {
      default: 'pointer',
      ':disabled': 'not-allowed',
    },
  },
});
```

Recommended pseudo-classes include:
- `:hover`, `:active`, `:focus`, `:focus-visible`, `:focus-within`

**IMPORTANT: Prefer JS changes over `:first-child` and `:nth-child` pseudo-elements. This reduces CSS bundle size.**

---

## Pseudo-elements

Define pseudo-elements as top-level keys within a style namespace:

```tsx
const styles = stylex.create({
  input: {
    color: 'black',
    '::placeholder': {
      color: 'gray',
      fontStyle: 'italic',
    },
    '::selection': {
      backgroundColor: 'yellow',
    },
  },
});
```

**IMPORTANT: Prefer actual HTML elements over `::before` and `::after` pseudo-elements. This reduces CSS bundle size and improves accessibility.**

---

## Media queries and @-rules

Nest media queries within property values:

```tsx
const styles = stylex.create({
  container: {
    flexDirection: {
      default: 'column',
      '@media (min-width: 768px)': 'row',
    },
    padding: {
      default: 8,
      '@media (min-width: 768px)': 16,
      '@media (min-width: 1024px)': 24,
    },
  },
});
```

**For app-wide breakpoints, use `stylex.defineConsts()` to define shareable media query constants:**

Other supported @-rules include `@supports` and `@container` queries.

**IMPORTANT: The `default` key is required when using nested conditions. Use `null` when no style should apply for the default case.**

---

## Dynamic styles

Use arrow functions for runtime values:

```tsx
const styles = stylex.create({
  bar: (width: number) => ({
    width,
  }),
  positioned: (x: number, y: number) => ({
    transform: `translate(${x}px, ${y}px)`,
  }),
});

<div {...stylex.props(styles.bar(100))} />
<div {...stylex.props(styles.positioned(mouseX, mouseY))} />
```

---

## Defining constants

Use `stylex.defineConsts()` for shareable media queries and static values like animations, colors, and font sizes that aren't themed.

**IMPORTANT: Use `defineConsts` over `defineVars` when values don't need to be themed or overridden at runtime.**

```tsx
// constants.stylex.ts
import * as stylex from '@stylexjs/stylex';

export const breakpoints = stylex.defineConsts({
  small: '@media (max-width: 600px)',
  medium: '@media (min-width: 601px) and (max-width: 1024px)',
  large: '@media (min-width: 1025px)',
});

export const zIndices = stylex.defineConsts({
  modal: '1000',
  tooltip: '1100',
  toast: '1200',
});
```

---

## Defining variables

Use `stylex.defineVars()` when values need theming or runtime overrides. Must be in `.stylex.ts` files:

```tsx
// tokens.stylex.ts
import * as stylex from '@stylexjs/stylex';

export const colors = stylex.defineVars({
  primary: 'blue',
  secondary: 'gray',
  text: 'black',
  background: 'white',
});

export const spacing = stylex.defineVars({
  small: '8px',
  medium: '16px',
  large: '24px',
});
```

---

## Using variables and constants

Import and use variables and constants in your styles:

```tsx
import * as stylex from '@stylexjs/stylex';
import { colors, spacing } from './tokens.stylex';

const styles = stylex.create({
  container: {
    backgroundColor: colors.background,
    color: colors.text,
    padding: spacing.medium,
  },
});
```

**IMPORTANT: For `defineConsts` and `defineVars`:**
- Must be in `.stylex.ts` or `.stylex.js` files
- Must be named exports (not default exports)
- No other exports allowed in the file

---

## Creating themes

Override variable values for DOM sub-trees using `stylex.createTheme()`:

```tsx
import * as stylex from '@stylexjs/stylex';
import { colors } from './tokens.stylex';

export const darkTheme = stylex.createTheme(colors, {
  primary: 'lightblue',
  text: 'white',
  background: '#1a1a1a',
});

// Apply theme to a container
function App({ isDark, children }) {
  return (
    <div {...stylex.props(isDark && darkTheme)}>
      {children} {/* All descendants use theme values */}
    </div>
  );
}
```

Unlike `defineVars`, themes can be created anywhere and passed across files/components.

---

## Relational selectors

Style elements based on the state of ancestors, descendants, or siblings using `stylex.when.*` selectors: `stylex.when.ancestor()`, `stylex.when.descendant()`, `stylex.when.anySibling()`, `stylex.when.siblingBefore()`, `stylex.when.siblingAfter()`.

Mark the observed element with `stylex.defaultMarker()` or create custom markers using `stylex.defineMarker()`.

```tsx
const styles = stylex.create({
  card: {
    transform: {
      default: 'translateX(0)',
      [stylex.when.ancestor(':hover')]: 'translateX(10px)',
    },
  },
});

<div {...stylex.props(stylex.defaultMarker())}>
  <div {...stylex.props(styles.card)}>Hover the parent to move me</div>
</div>
```

---

## Fallback styles

Use `stylex.firstThatWorks()` for browser compatibility fallbacks:

```tsx
const styles = stylex.create({
  header: {
    position: stylex.firstThatWorks('sticky', '-webkit-sticky', 'fixed'),
    display: stylex.firstThatWorks('grid', 'flex'),
  },
});
```

---

## Keyframe animations

Define animations with `stylex.keyframes()`:

```tsx
const fadeIn = stylex.keyframes({
  from: { opacity: 0 },
  to: { opacity: 1 },
});

const slideIn = stylex.keyframes({
  '0%': { transform: 'translateX(-100%)' },
  '100%': { transform: 'translateX(0)' },
});

const styles = stylex.create({
  animated: {
    animationName: fadeIn,
    animationDuration: '0.3s',
    animationTimingFunction: 'ease-out',
  },
});
```

---

## View transitions

Use `stylex.viewTransitionClass()` to customize View Transition API animations:

```tsx
import * as stylex from '@stylexjs/stylex';
import { ViewTransition } from 'react';

const fadeInUp = stylex.keyframes({
  from: { opacity: 0, transform: 'translateY(-30px)' },
  to: { opacity: 1, transform: 'translateY(0)' },
});

const transitionClass = stylex.viewTransitionClass({
  group: { /* ::view-transition-group styles */ },
  imagePair: { /* ::view-transition-image-pair styles */ },
  old: { animationDuration: '2s' },
  new: { animationName: fadeInUp },
});

<ViewTransition default={transitionClass}>{/* ... */}</ViewTransition>
```

---

## Anchor positioning

Use `stylex.positionTry()` to define CSS anchor positioning fallbacks:

```tsx
const fallback = stylex.positionTry({
  positionAnchor: '--anchor',
  top: '0',
  left: '0',
  width: '100px',
  height: '100px',
});

const styles = stylex.create({
  tooltip: {
    positionTryFallbacks: fallback,
  },
});
```

---

## TypeScript integration

Use `StyleXStyles` and `StyleXStylesWithout` over `StaticStyles` and `StaticStylesWithout` for type-safe style objects.

### StyleXStyles

Accept any StyleX styles:

```tsx
import type { StyleXStyles } from '@stylexjs/stylex';

type Props = {
  style?: StyleXStyles;
};
```

Constrain to specific properties:

```tsx
type Props = {
  style?: StyleXStyles<{
    color?: string;
    backgroundColor?: string;
  }>;
};
```

### StyleXStylesWithout

Exclude specific properties:

```tsx
import type { StyleXStylesWithout } from '@stylexjs/stylex';

type Props = {
  // Allow all styles except layout properties
  style?: StyleXStylesWithout<{
    margin: unknown;
    padding: unknown;
    width: unknown;
    height: unknown;
  }>;
};
```

### VarGroup

Types for variable groups:

```tsx
import type { VarGroup } from '@stylexjs/stylex';
import { colors } from './tokens.stylex';

function ThemeProvider({ theme }: { theme: VarGroup<typeof colors> }) {
  return <div {...stylex.props(theme)}>{children}</div>;
}
```

---

## Common antipatterns

**IMPORTANT: Avoid these common mistakes:**

### Don't import non-StyleX values

```tsx
// invalid: imported non-StyleX variable
import { PADDING } from './constants';
const styles = stylex.create({
  container: { padding: PADDING },
});

// valid: use StyleX constants or variables
import { spacing } from './tokens.stylex';
const styles = stylex.create({
  container: { padding: spacing.medium },
});
```

### Don't use `style` or `className` props

Do not apply `style` or `className` props on an element with a `stylex.props()` spread.

```tsx
// invalid: no `classname` and `style` prop usage
<div className="m-10" style={style} {...stylex.props(styles.container)} />

// valid
<div {...stylex.props(styles.container)} />
```

### Don't use media queries or pseudo-classes at the top level

Media queries and pseudo-classes must be nested inside property values, not at the top level of a style object.

```tsx
// invalid: media query at top level
const styles = stylex.create({
  container: {
    '@media (min-width: 768px)': {
      padding: 16,
    },
  },
});

// invalid: pseudo-class at top level
const styles = stylex.create({
  button: {
    ':hover': {
      backgroundColor: 'blue',
    },
  },
});

// valid: nest inside property values
const styles = stylex.create({
  container: {
    padding: {
      default: 8,
      '@media (min-width: 768px)': 16,
    },
  },
  button: {
    backgroundColor: {
      default: 'lightblue',
      ':hover': 'blue',
    },
  },
});
```

---

## More resources

- Official documentation: https://stylexjs.com
- API reference: https://stylexjs.com/docs/api
- GitHub repository: https://github.com/facebook/stylex
