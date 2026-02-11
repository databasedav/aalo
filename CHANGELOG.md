all notable changes to this project will be documented in this file

the format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Common Changelog](https://common-changelog.org/), and this project vaguely adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

## unreleased

# 0.2.0 (2026-02-10)

### changed

- upgrade to bevy 0.17
- upgrade to haalka 0.6, still using deprecated futures-signals backend

# 0.1.0 (2025-11-30)

### changed

- upgrade to bevy 0.16
- upgrade to haalka 0.5
- text inputs use bevy_ui_text_input instead of bevy_cosmic_edit
- justfile + CI migrated to centralized kaaj package
- aalo no longer piggybacks off an available camera and just uses its own all the time

### added

- haalka's debug ui available in deployed examples, press F1 to activate the debug overlay

### fixed

- aalo header text no longer flashes in the middle of the screen at startup
- resizing the window no longer breaks the aalo header text position
- search and targeting windows no longer leak pointer events to elements underneath

# 0.0.5 (2025-04-22)

### added

- inspector root inherits `RenderLayers` of its `TargetCamera`

### changed

- **breaking:** renamed `Inspector::unnest_children` to `.flatten_descendants`
- searching ignores case

### fixed

- inspector `Visibility` inherited by all descendants

# 0.0.4 (2025-04-12)

### added

- visibility toggle in world example

### fixed

- nested entity fields (e.g. of `Children`) get populated with name
- toggling inspector visibility propagates to aalo text

# 0.0.3 (2025-04-09)

### fixed

- setting search inspector target root snaps to root header
- deeply nested headers don't disappear behind inspector
- numeric fields clickable on web

# 0.0.2 (2025-04-09)

### added

- newly added objects respect active search

### changed

- increase default scroll pixels from 15 to 20

### fixed

- clicking/downing resize borders selects inspector
- flipping between inspection target roots does not leave root headers in wacky places
- flipping between inspection target roots does not result in partially scrolled root
- inspection targets are correctly retargeted when flipping through roots

# 0.0.1 (2025-04-09)

### added
- initial release
