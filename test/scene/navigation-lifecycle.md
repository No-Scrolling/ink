# Navigation and native-view lifetime

The `navigation` and `lifecycle` integration targets use fixtures compiled by Ink, including its generated file router. Real QuickJS and React produce commits which the production `ReactTree` and `Engine` consume. Interactions originate from native hit testing and native back, rather than calling event handlers directly.

Navigation contracts cover decoded route parameters, retained page state after back, replacement history, independent tab state, tab visits which do not add history, selecting an existing tab from a covering detail, static route precedence, notification navigation which clears obsolete history, and recovery from a deleted notification destination. Expected labels and history outcomes are independently specified application behaviour.

Native-view lifetime contracts cover subscription disposal while an Activity is hidden, reconnection without rebuilding the view, delivery after reconnection, deferred value and collection edits, disposal after removal, and rejection of an action captured before unmount. The subscription is the genuine `onNativeMessage` subscription; counters record application callbacks rather than replacing the renderer or native-view implementation. A report barrier consumes real commits as well as messages, so an unexpected hidden update cannot be discarded by the harness.

These tests check scene content and interaction, not pixels, GPU rendering or Android activity lifetime. They require the `navigation` and `lifecycle` scene fixtures to be prepared before direct Cargo runs.
