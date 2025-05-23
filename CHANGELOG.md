all notable changes to this project will be documented in this file

the format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Common Changelog](https://common-changelog.org/), and this project vaguely adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

## unreleased

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
