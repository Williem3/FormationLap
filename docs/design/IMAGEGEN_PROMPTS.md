# Formation Lap concept prompts

Mode: built-in `image_gen` tool.

These are the final prompt specifications used to create the implementation
references in [`concepts/`](concepts/).

## UI kit

```text
Use case: ui-mockup
Asset type: high-fidelity desktop application UI-kit board for a Windows app
Primary request: Create a polished, production-minded visual design system board
for an application named "Formation Lap", a sim-racing session launcher that
starts support apps in order, monitors process health, and launches one racing
sim. The design must feel like an Apple-style professional utility adapted to
Windows, not like a gaming launcher.
Scene/backdrop: clean landscape design-spec canvas on Paddock White #F4F5F7 with
Fog Panel #E8EBEF surfaces
Subject: a coherent UI kit showing color swatches, typography hierarchy,
buttons, segmented controls, VR toggle, text fields, select menus, compact
application rows, status indicators for "Running", "Pre-existing", "Not
Responding", "Stopped", confirmation dialog, empty state, sidebar profile item,
and the distinctive Formation Rail component connecting ordered app nodes to a
final sim node
Style/medium: realistic shippable desktop product UI, crisp vector-like
interface rendering, Apple-inspired calm utility aesthetic with native-Windows
practicality; generous but disciplined whitespace, thin separators, soft inset
panels, precise 10–16px corner radii, subtle shadows only where hierarchy
requires them
Composition/framing: 16:9 landscape component board, neatly organized modular
grid, readable at a glance, one light-theme system with a compact dark-theme
palette preview
Lighting/mood: quiet, trustworthy, engineered, premium without luxury theatrics
Color palette: Paddock White #F4F5F7, Fog Panel #E8EBEF, Carbon Ink #15171B,
Telemetry Gray #68717D, Lap Blue #3478F6, Tire Warmth #E89B3C; reserve green and
red only for process state
Typography: Apple-style system sans similar to SF Pro for interface copy;
compact utility data uses the same family with tabular numerals
Text (verbatim): "Formation Lap", "Start session", "Close session", "Running",
"Pre-existing", "Not Responding", "Stopped", "VR", "Required", "Optional"
Signature: the Formation Rail is a thin continuous route through ordered
circular app nodes into a larger sim node; it warms from blue to amber as
startup progresses and becomes ready green, encoding actual startup order
rather than decoration
Constraints: native Windows window controls are not part of this kit board; no
copyrighted game art; no logos other than the plain Formation Lap wordmark;
practical accessible hierarchy; all state uses icon plus text, never color
alone; visible keyboard focus examples; no watermark
Avoid: checkerboard motifs, speedometers, tachometers, neon gamer styling,
black-and-red esports palette, glossy glassmorphism, giant hero cards,
promotional game launcher art, excessive gradients, excessive blur, ornamental
racing stripes, fake 3D controls
```

## Dashboard

```text
Use case: ui-mockup
Asset type: high-fidelity desktop application concept page
Input images: Image 1 is the approved Formation Lap UI-kit style reference;
preserve its palette, typography, component language, spacing, status semantics,
and Formation Rail signature
Primary request: Create the main Dashboard screen for "Formation Lap", a
sim-racing session launcher. This is a realistic shippable Windows desktop app
screen, not a presentation board.
Scene/backdrop: complete app window with a real restrained Windows title bar and
standard minimize, maximize, and close controls; light theme
Style/medium: Apple-inspired professional utility adapted to Windows; calm,
compact, precise; use the UI kit exactly; native-feeling title bar,
translucent-feeling Fog Panel sidebar, Paddock White content canvas, soft inset
grouped surfaces
Composition/framing: 16:10 desktop window around 1440x900. Left sidebar about
230px wide. Main content uses a quiet toolbar, compact header, Formation Rail,
primary sim summary, and ordered application table without oversized hero cards.
Sidebar content: plain wordmark "Formation Lap" at top; section label "Profiles";
profile items "Le Mans Ultimate", "iRacing", and "Assetto Corsa Competizione"
with small locally sourced generic icons; "Le Mans Ultimate" selected; button
"New profile"; bottom navigation items "Settings" and "Diagnostics". No user
avatar and no account UI.
Main header: title "Le Mans Ultimate"; small caption "5 applications · Game
launches last"; binary VR toggle switched on with label "VR"; one prominent blue
button "Start session"; subtle "Edit profile" action.
Formation Rail: a thin continuous ordered route through five labeled nodes:
"Crew Chief", "SimHub", "LMUFFB", "Desktop switcher", "Le Mans Ultimate". This
is pre-session, so nodes are quiet gray with a Lap Blue outline on the final sim;
show small numeric order labels and make the sim node slightly larger. It must
look like a functional startup sequence, not a diagram illustration.
Application section: heading "Applications" and compact rows for Crew Chief,
SimHub, LMUFFB, and VirtualDesktopSwitcher. Each row includes drag handle, local
icon, app name, Required or Optional chip, status text "Stopped", a small update
indicator where appropriate, and unobtrusive Start and overflow controls. Show a
small line of supporting detail for VirtualDesktopSwitcher: "Console · Hidden ·
Stops with session".
Sim section: compact final row/card for "Le Mans Ultimate" tagged "Game · Steam"
with status "Stopped" and preferred VR mode "OpenVR".
Footer/status: quiet local-only note "All systems ready to start" with a check
icon; no cloud/account content.
Color palette: exactly match Image 1: Paddock White #F4F5F7, Fog Panel #E8EBEF,
Carbon Ink #15171B, Telemetry Gray #68717D, Lap Blue #3478F6, Tire Warmth
#E89B3C; reserve green/red for process state only
Constraints: practical accessible desktop layout; visible keyboard focus on
Start session; states use icon plus text; no copyrighted game artwork; no
account/avatar; no watermark
Avoid: checkerboards, gauges, tachometers, neon, gaming artwork, colorful
marketing tiles, giant hero panel, excessive cards, excessive blur, custom fake
macOS traffic-light window controls, decorative graphs, dense microtext
```

## Profiles

```text
Use case: ui-mockup
Asset type: high-fidelity desktop application concept page
Input images: Image 1 is the Formation Lap UI-kit reference; Image 2 is the
approved Formation Lap Dashboard reference. Preserve their exact design
language, palette, typography, sidebar structure, native Windows frame, spacing,
controls, and density.
Primary request: Create the Profiles management and profile editor screen for
"Formation Lap". It must look like another real screen in the same shipped app,
not a design board.
Scene/backdrop: complete light-theme Windows desktop app window with the same
native title bar, Fog Panel sidebar, and Paddock White main canvas as the
references
Style/medium: realistic shippable React/Tauri product UI; Apple-inspired
professional utility adapted to Windows; calm grouped surfaces, restrained
shadows, precise spacing
Composition/framing: 16:10 desktop window around 1440x900. Reuse the exact left
sidebar from the Dashboard with wordmark, profile items, New profile, Settings,
Diagnostics. Main area is a practical profile editor with a compact top toolbar
and two-column content.
Main toolbar: breadcrumb-like eyebrow "Profiles"; large title "Le Mans
Ultimate"; status caption "Saved locally"; actions "Duplicate", "Export", and
one prominent blue "Save changes" button.
Left main column: grouped inset section titled "Profile" with fields "Profile
name" set to "Le Mans Ultimate", "Racing sim" set to "Le Mans Ultimate", and
source segmented control with "Steam" selected and "Executable" unselected.
Group titled "Launch" with "Default to VR" toggle on, select "Preferred VR mode"
set to "OpenVR", read-only Steam App ID "2399420", button "Test game launch",
and helper copy "Runs the game only and records diagnostics". Group titled
"Session close" with toggle "Close when game exits" on and toggle "Stop SteamVR"
off.
Right main column: heading "Startup order" with caption "The game always
launches last" and a compact vertical ordered list. Rows: "Crew Chief" Required,
"SimHub" Required, "LMUFFB" Required, "VirtualDesktopSwitcher" Optional with
small sublabel "Console · Hidden", then a visually separated locked final row
"Le Mans Ultimate" with label "Game · Always last". Each support-app row has
drag handle, local generic icon, required/optional selector, overflow button,
and subtle remove control. Add a quiet outlined button "Add application" below.
Bottom right group: heading "On session close" with a small application row for
a telemetry viewer "Go Fast" and toggle/chip "Keep running" to demonstrate the
exception behavior.
Constraints: practical form layout; labels above controls; visible keyboard
focus example; app state uses text plus icon; no account UI; no copyrighted
artwork; no watermark
Avoid: dashboard Formation Rail on this page, giant cards, marketing art,
checkerboards, gauges, neon, fake macOS titlebar controls, overly colorful
icons, excessive blur, dense tiny text, decorative graphs
```

## Settings

```text
Use case: ui-mockup
Asset type: high-fidelity desktop application concept page
Input images: Images 1–3 are the approved Formation Lap UI kit, Dashboard, and
Profiles editor references. Preserve their exact visual language, palette,
typography, native Windows frame, left sidebar, spacing, control shapes, and
information density.
Primary request: Create the Settings screen for "Formation Lap" as a realistic
shippable screen in the same application.
Scene/backdrop: complete light-theme Windows desktop app window with the same
native title bar; Fog Panel left sidebar; Paddock White main canvas
Style/medium: Apple-inspired professional utility adapted to Windows; precise
grouped settings, calm whitespace, no promotional content
Composition/framing: 16:10 desktop window around 1440x900. Reuse the Dashboard
sidebar exactly, but highlight "Settings" at the bottom with the same selected
treatment. Main content begins with title "Settings" and caption "Formation Lap
0.1.0". Use a balanced two-column grid of grouped inset settings panels, aligned
control labels, generous whitespace, and concise helper text.
Main panel 1 titled "General": toggle "Start with Windows" off with helper "Open
minimized in the system tray"; toggle "Keep running in tray" on; select "Startup
timeout" set to "30 seconds".
Main panel 2 titled "Appearance": segmented control "System" selected, "Light",
"Dark"; small live color theme preview; select "Interface density" set to
"Comfortable"; toggle "Reduce motion" off.
Main panel 3 titled "Updates": row "Formation Lap updates" with channel chip
"Stable" and button "Check now"; toggle "Check automatically" on with helper "At
most once per day, never during a session"; select "Update channel" set to
"Stable"; toggle "Check application updates" on with helper "Notifications only
— Formation Lap never installs third-party updates".
Main panel 4 titled "Race-safe behavior": a clear locked-on setting "Suppress
notifications while the sim is running" with check icon; helper "Events appear
in the session summary afterward"; select "Not Responding threshold" set to "2
checks · about 6 seconds".
Main panel 5 titled "Data & privacy": prominent sentence "Everything stays on
this PC"; rows "Profiles and settings" with button "Open folder", "Backups" with
status "Enabled", and "Diagnostic logs" with button "Export"; helper "No
account, analytics, or cloud storage".
Main panel 6 titled "Advanced": button "Manage update providers"; button "Reset
app discovery"; quiet destructive text button "Reset all settings" separated at
bottom.
Color palette: match all references exactly: Paddock White #F4F5F7, Fog Panel
#E8EBEF, Carbon Ink #15171B, Telemetry Gray #68717D, Lap Blue #3478F6, Tire
Warmth #E89B3C; state colors only for status
Constraints: practical accessible settings page; visible keyboard focus on one
control; labels and helper text clear; no account UI; no copyrighted artwork; no
watermark
Avoid: giant cards, promotional art, Formation Rail on settings, checkerboards,
gauges, neon, fake macOS traffic-light controls, excessive blur, decorative
charts, long paragraphs, dense tiny text
```
