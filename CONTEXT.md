# Ink

Ink is a framework for creating Light Phone III apps using ordinary TypeScript and React. It provides the phone interface and device APIs so you can focus on app content and behaviour.

## Language

**Ink app**:
A Light Phone III application built with Ink. It uses ordinary React semantics. React Native native modules are not supported.

**Ink screen**:
A page of app content presented within Ink's shared phone interface. The shared interface includes typography, spacing, scrolling, navigation controls and keyboard handling.

**Native capability**:
A device or platform facility available to an Ink app, such as camera capture or audio playback. Its availability is separate from whether the user has granted permission to use it.

**LightOS integration**:
An app's interaction with the LightOS host, including the preferences, permissions and services the host exposes. Integration does not imply official distribution approval.

**App appearance**:
An Ink app's choice of light or dark interface colours. It is distinct from the user's LightOS invert-colours preference, which the inspected SDK does not expose.
