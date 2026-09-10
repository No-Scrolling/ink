# Design apps with Ink

Build small, focused apps that feel at home on the Light Phone III. The user should see what they came for and know what to do next.

## Start with the job

- Make the main task available when the app opens. Avoid welcome screens, setup steps and start buttons unless they serve a real purpose.
- Give each screen a clear purpose. Put occasional adjustments in Settings, usually reached through the header.
- Choose useful defaults. Add a preference only for a meaningful choice; keep optional detail out of the main view by default.
- Remove anything that does not help someone understand, decide or act. Simplicity must not hide information they need.

## Let Ink own the interface

- Start with existing screens and components. Let Ink handle typography, appearance, padding, navigation, scrolling and the keyboard.
- Compose with primitives when needed. An app-specific layout does not automatically need a new framework component.
- Group related content and use consistent gaps. Centre a compact, self-contained task; let feeds and longer content flow from the top.
- Do not add padding on top of padding Ink already supplies. Use shared bottom actions and footers rather than positioning your own against the screen edge.
- Use Ink's icons, muted tones and selected states. Avoid decorative containers, borders and status labels.

## Keep interaction direct

- Inputs always belong on their own page. Only hints or errors may appear underneath; do not mix inputs with lists, results or other content.
- Use the appropriate keyboard; its action submits or saves. Do not add a duplicate Save button.
- Use prefix and suffix text for units. Keep instructions only where the input's purpose or constraints would otherwise be unclear.
- Search submits to a results page. Multiline inputs keep Return for new lines and use a header action to save or continue. Conversations use their separate send action.
- Use the header's Back action to leave a page. Do not add a second Cancel action to an ordinary action page.
- Header Back leaves the page in one tap, even with the keyboard open. Dismiss the keyboard as part of navigation.
- Check permissions when a feature needs them. If opening the feature clearly expresses intent, request access directly rather than adding a redundant permission button.

## Keep the screen steady

- Keep useful content visible during refreshes and when returning from another page. Reserve full-screen loading for times when there is nothing useful to show.
- Preserve scroll position, drafts and selections. Let Ink coordinate keyboard dismissal and navigation.
- Use `tabularNumbers` for changing numeric readings. Reserve image space when its dimensions are known.
- Keep the last useful reading through brief signal gaps when appropriate. Indicate stale data when mistaking it for a current reading could mislead.
- Show errors where they affect the task, with a clear way to recover. Avoid status chatter such as “Ready” or “Preparing” that only causes layout changes.

## Finish by subtracting

Use short, plain British English. Name actions directly. Delete explanatory copy that merely describes an obvious control.

Try the app with the keyboard open, long content, missing data and a return from another page. Check both appearances. Use the emulator for layout and the phone for hardware-dependent behaviour.

Before finishing, ask: what can disappear without making the app harder to use?

See [Screens and layout](docs/screens.mdx) for components and [Tuner](examples/tuner) for a focused app example.
