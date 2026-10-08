# Design apps with Ink

Use Ink’s components and defaults to build Light Phone III apps.

## Screens and defaults

- Open the app on its main task. Add welcome screens, setup steps or start buttons only when required.
- Use a separate screen for each task. Put occasional adjustments in Settings, usually reached through the header.
- Choose defaults that work without setup. Add settings for choices users need to make. Hide optional details by default.
- Remove unrelated content and controls. Keep information needed to complete the task.

## Components and layout

- Use existing screens and components for typography, appearance, padding, navigation, scrolling and the keyboard.
- Combine primitives for custom layouts before adding framework components.
- Group related content and use consistent gaps. Centre short content; align feeds and longer content to the top.
- Do not add padding on top of padding Ink already supplies. Use shared bottom actions and footers rather than positioning your own against the screen edge.
- Use Ink's icons, muted tones and selected states. Avoid decorative containers, borders and status labels.

## Inputs and navigation

- Place inputs on a separate page, with only hints or errors underneath. Do not mix them with lists, results or other content.
- Use the appropriate keyboard; its action submits or saves. Do not add a duplicate Save button.
- Use prefix and suffix text for units. Add instructions only to explain the input’s purpose or limits.
- Search submits to a results page. Multiline inputs keep Return for new lines and use a header action to save or continue. Conversations use their separate send action.
- Use the header’s Back action to leave a page. Do not add a duplicate Cancel action.
- Header Back leaves the page in one tap, even with the keyboard open. Dismiss the keyboard as part of navigation.
- Request permission when the user opens the feature that needs it. Do not add a separate permission button.

## Loading and updates

- Keep existing content visible during refreshes and when returning from another page. Use full-screen loading when no content is available.
- Keep controls’ normal appearance while an action is pending. Prevent duplicate actions without setting `disabled` or flashing controls grey. Reserve disabled styling for controls that are unavailable for use.
- Preserve scroll position, drafts and selections. Use Ink’s keyboard dismissal and navigation behaviour.
- Use `tabularNumbers` for changing numeric readings. Reserve image space when its dimensions are known.
- Keep the last reading visible through brief signal gaps. Mark stale readings when the distinction matters.
- Show errors beside the affected content or control, with a recovery action. Omit “Ready” or “Preparing” labels that only cause layout changes.

## Review

Use plain British English and direct action names. Remove text that repeats a control’s label or explains an obvious action.

Check the app with the keyboard open, long content, missing data and navigation back from another page. Check light and dark modes. Use the emulator for layout and the phone for hardware features.

See [Screens and layout](docs/screens.mdx) for components and [Light Template](examples/light-template) for an example app.
