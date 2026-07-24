# Formation Lap

Formation Lap prepares, monitors, and closes the collection of applications
needed for a sim-racing session.

## Language

**Racing Profile**:
A named setup containing exactly one Primary Sim and an ordered collection of
Supporting Applications.
_Avoid_: Preset, configuration, app list

**Primary Sim**:
The single racing game a Racing Profile ultimately launches.
_Avoid_: Game app, primary application

**Supporting Application**:
A long-running program that prepares or augments a Session and normally starts
before the Primary Sim.
_Avoid_: Dependency, helper process, secondary app

**Startup Sequence**:
The ordered preparation of Supporting Applications followed by the Primary Sim.
_Avoid_: Batch, macro, script

**Session**:
One user-initiated run of a Racing Profile, beginning with startup and ending
with Close Session or startup cancellation.
_Avoid_: Run, launch group

**Active Session**:
The one Session Formation Lap currently owns or monitors.
_Avoid_: Current profile, running profile

**Required Application**:
A Supporting Application whose launch failure prevents the Primary Sim from
starting.
_Avoid_: Hard dependency, mandatory app

**Optional Application**:
A Supporting Application whose launch failure is recorded while the Startup
Sequence continues.
_Avoid_: Soft dependency

**Session-owned Process**:
A process started by the Active Session and therefore eligible for automatic
session cleanup.
_Avoid_: Child process, managed app

**Pre-existing Process**:
A matching process that was already running when a Session began and is not
eligible for automatic session cleanup.
_Avoid_: Adopted process, external app

**Keep-running Application**:
A Session-owned Supporting Application deliberately left open by Close Session;
afterward it is no longer Session-owned.
_Avoid_: Persistent app, ignored app

**Close Session**:
The user-visible action that closes the Primary Sim when necessary and then
cleans up eligible Supporting Applications in reverse startup order.
_Avoid_: Kill all, stop profile, exit batch

**Launch Recipe**:
The saved instructions for starting and identifying one Primary Sim or
Supporting Application.
_Avoid_: Command, invocation string

**VR Launch Mode**:
The game-specific virtual-reality path selected when a Racing Profile starts
with VR enabled.
_Avoid_: VR flag

**Formation Rail**:
The ordered visual representation of a Startup Sequence and its current state.
_Avoid_: Progress bar, pipeline

**Curated Catalog**:
Formation Lap's reviewed set of recognized racing sims, Supporting
Applications, launch metadata, compatibility hints, and update sources.
_Avoid_: Library, app store

**Manual Entry**:
A user-defined sim or Supporting Application that is not supplied by the
Curated Catalog.
_Avoid_: Unknown app, custom command

**Race-safe Behavior**:
The suppression of unsolicited notifications, restarts, and disruptive actions
while the Primary Sim is running.
_Avoid_: Do not disturb

**Recovery Offer**:
An explicit choice to resume monitoring a previously recorded Session after
Formation Lap restarts.
_Avoid_: Automatic recovery, session restore

**Update Provider**:
A trusted source that can report whether a known application version is newer,
without installing it.
_Avoid_: Updater, package manager

**Not Responding**:
The state reported when a windowed application repeatedly fails Windows message
responsiveness checks.
_Avoid_: Frozen, crashed
