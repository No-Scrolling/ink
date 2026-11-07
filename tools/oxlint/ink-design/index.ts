import { eslintCompatPlugin } from "@oxlint/plugins";

import { noEmptyFieldRule } from "./rules/no-empty-field.ts";
import { noPageWrapperStackRule } from "./rules/no-page-wrapper-stack.ts";
import { noPendingControlChangesRule } from "./rules/no-pending-control-changes.ts";

/** Rules that keep Ink apps to the structure described in DESIGN.md. */
const inkDesignPlugin = eslintCompatPlugin({
  meta: { name: "ink-design" },
  rules: {
    "no-empty-field": noEmptyFieldRule,
    "no-page-wrapper-stack": noPageWrapperStackRule,
    "no-pending-control-changes": noPendingControlChangesRule,
  },
});

export default inkDesignPlugin;
