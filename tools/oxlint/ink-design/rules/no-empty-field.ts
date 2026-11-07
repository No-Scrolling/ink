import { defineRule } from "@oxlint/plugins";

import { inkElementName, inkImports, meaningfulChildren } from "../ink-elements.ts";

/** Reject Fields without a value, which should be Buttons that open a page. */
export const noEmptyFieldRule = defineRule({
  meta: {
    type: "suggestion",
    docs: { description: "Show a value in every Field; use Button to open a group of settings." },
    messages: {
      empty:
        "A `Field` shows a labelled value. Use `<Button href=\"…\">` to open a page without a value. See node_modules/ink/DESIGN.md.",
    },
  },
  createOnce(context) {
    let imports = new Map<string, string>();
    return {
      Program(program) {
        imports = inkImports(program);
      },
      JSXElement(element) {
        if (inkElementName(element, imports) !== "Field") return;
        const children = meaningfulChildren(element);
        const [only] = children;
        const empty =
          children.length === 0 ||
          (children.length === 1 &&
            only?.type === "JSXExpressionContainer" &&
            only.expression.type === "Literal" &&
            only.expression.value === "");
        if (empty) context.report({ node: element.openingElement, messageId: "empty" });
      },
    };
  },
});
