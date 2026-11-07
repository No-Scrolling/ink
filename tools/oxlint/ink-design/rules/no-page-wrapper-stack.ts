import { defineRule, type ESTree } from "@oxlint/plugins";

import { attribute, inkElementName, inkImports, meaningfulChildren, stringAttribute } from "../ink-elements.ts";

const controls = new Set(["Button", "Field", "Toggle", "Row", "TextInput", "List", "ReorderList", "SettingsChoices"]);

/** Elements a child renders, looking through fragments, `&&` and conditional expressions. */
function renderedElements(child: ESTree.JSXChild | ESTree.Expression): ESTree.JSXElement[] {
  switch (child.type) {
    case "JSXElement":
      return [child];
    case "JSXFragment":
      return child.children.flatMap(renderedElements);
    case "JSXExpressionContainer":
      return child.expression.type === "JSXEmptyExpression" ? [] : renderedElements(child.expression);
    case "LogicalExpression":
      return renderedElements(child.right);
    case "ConditionalExpression":
      return [...renderedElements(child.consequent), ...renderedElements(child.alternate)];
    case "ParenthesizedExpression":
      return renderedElements(child.expression);
    default:
      return [];
  }
}

/** Reject a vertical Stack that wraps a Screen's controls and replaces its spacing. */
export const noPageWrapperStackRule = defineRule({
  meta: {
    type: "suggestion",
    docs: { description: "Place page content directly inside Screen, which already spaces its children." },
    messages: {
      wrapper:
        "Put these components directly inside `Screen`; it already spaces its children. Use `Stack` only for a small group, or `justify=\"space-between\"` to pin a footer. See node_modules/ink/DESIGN.md.",
    },
  },
  createOnce(context) {
    let imports = new Map<string, string>();
    return {
      Program(program) {
        imports = inkImports(program);
      },
      JSXElement(element) {
        if (inkElementName(element, imports) !== "Screen" || attribute(element, "centered") !== null) return;
        const children = meaningfulChildren(element);
        const [only] = children;
        if (children.length !== 1 || only?.type !== "JSXElement" || inkElementName(only, imports) !== "Stack") return;
        if (stringAttribute(only, "axis") === "horizontal" || stringAttribute(only, "justify") === "space-between") return;
        if (!only.children.flatMap(renderedElements).some(child => controls.has(inkElementName(child, imports) ?? ""))) return;
        context.report({ node: only.openingElement, messageId: "wrapper" });
      },
    };
  },
});
