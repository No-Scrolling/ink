# Navigation

The `navigation` integration target uses a fixture compiled by Ink, including its generated file router. Real QuickJS and React produce commits which the production `ReactTree` and `Engine` consume. Interactions originate from native hit testing and native back, rather than calling event handlers directly.

Navigation contracts cover decoded route parameters, retained page state after back, replacement history, independent tab state, tab visits which do not add history, selecting an existing tab from a covering detail, static route precedence, notification navigation which clears obsolete history, and recovery from a deleted notification destination. Expected labels and history outcomes are independently specified application behaviour.

These tests check scene content and interaction, not pixels, GPU rendering or Android activity lifetime. They require the `navigation` scene fixture to be prepared before direct Cargo runs.
