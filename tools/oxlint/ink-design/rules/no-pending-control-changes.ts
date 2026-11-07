import { defineRule } from "@oxlint/plugins";

import { attribute, dependsOnPending, inkElementName, inkImports } from "../ink-elements.ts";

const controls = new Set(["Button", "Field", "Toggle", "TextInput", "SettingsChoices", "Row"]);

/** Reject controls that grey out or change their label while an action is pending. */
export const noPendingControlChangesRule = defineRule({
  meta: {
    type: "suggestion",
    docs: { description: "Keep a control's appearance and label while its action is pending." },
    messages: {
      disabled:
        "Keep this control enabled while the action is pending; `useAction` already ignores repeated calls. Reserve `disabled` for unavailable controls. See node_modules/ink/DESIGN.md.",
      label:
        "Keep this control's label while the action is pending. See node_modules/ink/DESIGN.md.",
    },
  },
  createOnce(context) {
    let imports = new Map<string, string>();
    return {
      Program(program) {
        imports = inkImports(program);
      },
      JSXElement(element) {
        const name = inkElementName(element, imports);
        if (name === null || !controls.has(name)) return;
        const disabled = attribute(element, "disabled")?.value;
        if (
          disabled?.type === "JSXExpressionContainer" &&
          disabled.expression.type !== "JSXEmptyExpression" &&
          dependsOnPending(context.sourceCode, disabled.expression)
        ) {
          context.report({ node: disabled, messageId: "disabled" });
        }
        // A control whose press handler also acts on the pending state, such as Start and Stop, may change its label.
        const onPress = attribute(element, "onPress")?.value;
        if (onPress && /(["'])pending\1/.test(context.sourceCode.getText(onPress))) return;
        for (const child of element.children) {
          if (
            child.type === "JSXExpressionContainer" &&
            child.expression.type === "ConditionalExpression" &&
            dependsOnPending(context.sourceCode, child.expression.test)
          ) {
            context.report({ node: child, messageId: "label" });
          }
        }
      },
    };
  },
});
