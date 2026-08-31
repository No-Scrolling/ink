# How Ink works

Ink turns a focused TypeScript and TSX app into a native Android package. App code is compiled before the APK is built; it is not evaluated on the phone.

```text
TypeScript and TSX
        ↓
Ink compiler
        ↓
app.ink + required Android capabilities
        ↓
Rust runtime + Vulkan renderer
        ↓
Android APK
```

## Build input

An app contains:

- `ink.toml` for its name, Android package, version and optional signing settings.
- `App.tsx` as the composition root.
- Local screen modules, bundled assets, and installed Ink packages imported by the app.

A minimal configuration is:

```toml
name = "Weather"
package = "com.example.weather"
version = "0.1.0"
version_code = 1
```

Ink supplies its runtime, Public Sans, Android project and generated resources. Apps do not need to configure source paths, generated Rust files, fonts or Android resource directories.

## Compilation

The compiler resolves the complete screen and package graph from `App.tsx`. It parses TypeScript and TSX, validates the supported language, checks routes and typed data, and writes a compact `app.ink` definition.

The definition contains the app's:

- screens, layout, and navigation;
- initial, shared, and persisted state declarations;
- state-to-UI dependencies;
- typed resource and native action declarations;
- referenced text, icons, images and other assets;
- required native capabilities.

Changing ordinary app UI or data rewrites this definition. It does not generate app-specific Rust source or relink the shared native runtime.

## Native packaging

The capability list produced by the compiler controls what enters the APK. For example:

- `TextInput` adds the Ink keyboard;
- `@ink/audio` playback adds the audio player;
- detached playback also adds the Android media session;
- `@ink/camera` scanning adds CameraX and code decoding;
- modules add only the Android permissions and components they use.

An app that does not use a capability does not carry its native implementation. `ink info` shows the capability list and its estimated native cost before a release build.

## Runtime

The Rust runtime reads `app.ink` and owns state, navigation, layout, scrolling, hit testing and async resource states. It retains the current UI tree and updates affected branches when state changes.

Fixed-height vertical lists are virtualised. Text and image geometry is reused while unchanged, and scrolling updates retained content rather than rebuilding the complete screen. Ink submits no frames while the app is visually idle.

The renderer turns Ink's display list into Vulkan commands. Android supplies the window, lifecycle, pointer input and native device services through a small adapter.

## Native actions and resources

Network responses, location fixes, permissions and other native results return through typed resource states. The runtime:

- activates screen-scoped work only while its screen is visible;
- cancels work when the screen leaves or a request reloads;
- ignores late results from cancelled or replaced requests;
- applies timeouts and prevents duplicate in-flight reads;
- schedules the resulting UI change on the next display frame.

Explicit mutations and native actions remain separate from reads, so an app controls when work that changes external state begins.

## Platform conventions

Ink targets portrait Android apps for the Light Phone III on API 34 or later.

- Layout uses LP3 logical units rather than Android display-density units.
- Public Sans Regular is the bundled text face.
- Material Symbols are included by reference; unused symbols are omitted.
- Text is fully opaque. Bold, italic and arbitrary fonts are not supported.
- Local PNG and HTTPS images are supported.
- The Android splash is blank and black.
- Launcher artwork is generated from the app name.
- Apps contain no JavaScript engine and cannot execute arbitrary JavaScript packages.

Read [Core Ink](ink.md) for the authoring API and [Benchmarks](../benchmarks/README.md) for the measured LP3 results and reproduction method.
