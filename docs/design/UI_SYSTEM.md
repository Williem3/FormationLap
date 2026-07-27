# Formation Lap UI system

This document is the implementation contract for Formation Lap's interface. The
concept images are visual references, not pixel-perfect specifications. When an
image and this document disagree, this document wins.

## Product character

Formation Lap is a calm, trustworthy utility for preparing a sim-racing
session. It borrows the clarity and restraint of Apple utility software while
remaining a well-behaved Windows application with native window chrome.

The interface must feel operational rather than promotional:

- Show the next useful action and current process state.
- Keep configuration separate from the session dashboard.
- Use color for meaning, never decoration alone.
- Avoid gamer conventions such as neon, tachometers, checkers, racing stripes,
  large game artwork, and black-and-red esports palettes.
- Keep all nonessential motion quiet while a session is active.

## Signature: Formation Rail

The Formation Rail is the product's one expressive visual device. It represents
the actual startup sequence: supporting applications form ordered nodes that
lead to a larger final game node.

- Stopped and failed nodes use the danger tone; pending and in-transition nodes
  use Tire Warmth; both Session-owned and Pre-existing running nodes use
  Running Green.
- Nodes use solid, compact start-light markers rather than ordinal labels; the
  Startup Sequence is communicated left to right.
- A failed required app interrupts the rail before the game node.
- Every node includes an icon and accessible text; color is never the only state
  signal.
- The rail is functional UI. Do not use it on Settings or unrelated screens.

## Color tokens

### Light

| Token     | Value     | Use                                                      |
| --------- | --------- | -------------------------------------------------------- |
| `canvas`  | `#F4F5F7` | Main window background ("Paddock White")                 |
| `panel`   | `#E8EBEF` | Sidebar and quiet grouped surfaces ("Fog Panel")         |
| `ink`     | `#15171B` | Primary text and icons ("Carbon Ink")                    |
| `muted`   | `#68717D` | Secondary text and stopped state ("Telemetry Gray")      |
| `accent`  | `#3478F6` | Primary actions, focus, selected navigation ("Lap Blue") |
| `warm`    | `#E89B3C` | Startup progress and warning ("Tire Warmth")             |
| `running` | `#1CA34A` | Running and ready state                                  |
| `danger`  | `#D94747` | Failure and destructive actions                          |
| `surface` | `#FFFFFF` | Inputs and raised content groups                         |
| `border`  | `#D5DAE1` | Dividers and control outlines                            |

### Dark

| Token     | Value     |
| --------- | --------- |
| `canvas`  | `#1C1F24` |
| `surface` | `#23272D` |
| `panel`   | `#272B31` |
| `border`  | `#2F343C` |
| `ink`     | `#E6E9EE` |
| `muted`   | `#9AA3AE` |
| `accent`  | `#3A82F7` |
| `warm`    | `#F0A243` |

State tokens retain their semantic roles in both themes and must meet WCAG AA
contrast when paired with text or icons.

The base state colors are also used for borders, fills, and the Formation Rail.
Small text and filled-action labels use the theme-aware derived tokens
`accent-ink`, `warm-ink`, `running-ink`, `danger-ink`, `accent-action`,
`danger-action`, and `action-ink`. These preserve each semantic hue while
meeting at least 4.5:1 against the applicable surface. Do not use a base state
token directly as small text simply because its non-text contrast passes.

## Typography

Use an Apple-style system stack without bundling proprietary fonts:

- Prefer **SF Pro** through `-apple-system` and `BlinkMacSystemFont` where the
  platform supplies it.
- On Windows, use **Segoe UI Variable** / **Segoe UI** as the native fallback.
- Use the same stack for headings, controls, copy, and compact utility data;
  use monospaced system fonts only for code and diagnostic output.

| Role            | Size / line height | Weight |
| --------------- | ------------------ | ------ |
| Display         | `28 / 36`          | 600    |
| Page heading    | `24 / 32`          | 600    |
| Section heading | `20 / 28`          | 600    |
| Row title       | `16 / 24`          | 600    |
| Body            | `14 / 20`          | 400    |
| Caption         | `12 / 16`          | 400    |
| Utility data    | `12 / 16`          | 500    |

Use sentence case. Labels describe what users control, not implementation
details.

## Geometry

- Spacing scale: `4, 8, 12, 16, 24, 32, 40`.
- Control radius: `8px`.
- Row and small-group radius: `12px`.
- Large panel and dialog radius: `16px`.
- Sidebar width: `248px` at the reference desktop size.
- Main content maximum width: `1280px`.
- Minimum supported window: `1040 × 680`.
- Borders are one physical pixel where possible.
- Shadows are limited to dialogs, menus, and temporary overlays.

The window uses native Windows title-bar controls. The web content must not draw
fake macOS traffic lights or replace native snap and resize behavior.

## Core components

### Profile sidebar item

Shows a local app icon, profile name, optional compact state, and selected
indicator. Profiles are navigation, never user accounts.

### Application row

Contains, in order:

1. Drag handle when editing.
2. Local executable icon or generic fallback.
3. Application name and one concise supporting line.
4. Required/Optional classification centered in its own fixed-width column.
5. Status indicator with icon and text in a separate fixed-width column.
6. Output action in the lifecycle-action column: `View Output` when native
   output is captured, otherwise a disabled `No Output` placeholder.
7. Update state when known.
8. Contextual lifecycle action.
9. Overflow menu.

Rows must remain legible without app icons.

### Status indicator

Supported labels include Starting, Running, Running (pre-existing), Not
Responding, Stopping, Stopped, and Failed. Every state pairs an icon with text.

### Buttons

- Primary: one per visible task area.
- Secondary: bordered surface.
- Tertiary: text or icon action.
- Destructive: danger color and explicit verb.
- Disabled controls retain readable labels.

Keyboard focus uses a `2px` Lap Blue ring with a visible offset. Hover is never
the only indication that a control is interactive.

### Grouped settings

Settings use inset groups with a heading, concise rows, and helper text only
where it prevents ambiguity. Controls align to a consistent right edge.

## Screen contracts

### Dashboard

- Profile sidebar remains visible.
- Header contains profile name, VR toggle, Edit Profile, and the single primary
  Start Session or Close Session action.
- Formation Rail follows the header.
- The Formation Rail heading reads `Driver's Start Your Engines!` until every
  node is running, then reads `And Away we go!`.
- Supporting applications appear before a visually separated game row.
- Session-level state occupies a quiet footer area.

### Profiles

- Launch and session behavior occupy the left column.
- Startup order occupies the wider right column.
- The game is a locked final row.
- Keep-running exceptions are explicit.
- Save Changes is the only primary action for approved profiles.
- Needs Review profiles show a native-quarantine panel and replace Save Changes
  with Save and Approve. The panel requires one complete configuration review
  confirmation plus one explicit confirmation for every elevated or custom-stop
  entry. Start Session remains disabled until native approval succeeds.

### Settings

- Settings is selected in the shared sidebar.
- General, Appearance, Updates, Race-safe behavior, Data & privacy, and Advanced
  are separate groups.
- The local-data privacy model is stated plainly without implying that
  explicitly requested online checks are local-only.
- First-run and Settings copy names GitHub Releases, Winget, and SimHub's
  official site as possible provider contacts.
- A manual check that finds a Formation Lap update exposes its verified install
  action in Settings beside Check now.
- The Dashboard update banner is reserved for automatic checks and remains
  non-blocking and race-safe.
- The quiet footer reads `Local data · Online checks on/off` from persisted
  settings.
- Reset actions are visually separated from routine settings.

## Interaction and motion

- Standard transitions last `120–180ms`.
- Formation Rail progress may animate during startup, but it must stop when the
  session reaches a stable state.
- Respect `prefers-reduced-motion` and the in-app Reduce Motion setting.
- Do not show unsolicited dialogs, toasts, or animated alerts while the sim is
  running.
- Destructive and force-termination confirmations use native-feeling modal
  dialogs with explicit targets.

## Asset rules

- Extract game and application icons from local executables or Steam metadata.
- Do not bundle or download copyrighted game artwork.
- Use a monochrome generic fallback when no icon is available.
- The colorful icons in the concepts are placeholders, not catalog assets.

## Accessibility acceptance criteria

- WCAG AA text contrast in light and dark themes.
- Complete keyboard access with visible focus.
- Status never communicated by color alone.
- Controls have accessible names and descriptions.
- Layout remains usable at 125%, 150%, and 200% Windows scaling.
- Reduced-motion preference is honored.

## Concept references

- [`concepts/ui-kit.png`](concepts/ui-kit.png)
- [`concepts/dashboard.png`](concepts/dashboard.png)
- [`concepts/profiles.png`](concepts/profiles.png)
- [`concepts/settings.png`](concepts/settings.png)

The concepts were generated with the built-in image generation tool from this
contract's visual direction and then reviewed for cross-screen consistency.
