# Matching counter emulator check

Checked on 6 September 2026 on emulator-5554 at 1080 × 1240, density 480. These are appearance and interaction checks, not performance results.

The active fixtures now contain only Ink, Expo and Light SDK counters. Each shows a standard framework header titled Counter, Public Sans count text, and an Increase action. Expo uses light-template's Header and StyledButton. Light SDK uses LightTopBar and LightText with lightClickable; its fixture bundles the same Public Sans font as Ink and Expo and sets matching typography through LightTheme. Framework text rasterisation, header geometry and line metrics retain small differences; these are not pixel-identical screenshots.

All three started at Count: 0 and reached Count: 100 after 100 taps at the shared coordinate (540, 760). OCR through scripts/agent-tools confirmed the final count and labels in each screenshot. No runtime errors were found in the inspected recent logcat output. Release builds succeeded for Expo and Light SDK. Ink used isolated experiment experiment-20260906-183629-53f3cf96, re-signed with the emulator's existing development key for installation without deleting its app data.

| App | Initial screen | After 100 taps |
| --- | --- | --- |
| Ink | [Initial](ink.png) | [100](ink-100.png) |
| Expo | [Initial](expo.png) | [100](expo-100.png) |
| Light SDK | [Initial](light-sdk.png) | [100](light-sdk-100.png) |

The measurement harness now uses the same tap position for all three counters. Runtime and build scripts no longer require scrolling projects. Bun bundling and shell syntax checks passed; no tests were written. Previous physical-device benchmark figures remain historical and need a fresh run with the aligned fixtures.
