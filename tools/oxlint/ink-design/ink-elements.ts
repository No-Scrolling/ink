import type { ESTree, Scope, SourceCode } from "@oxlint/plugins";

/** Maps local JSX names to the components they import from `ink`. */
export function inkImports(program: ESTree.Program): Map<string, string> {
  const names = new Map<string, string>();
  for (const statement of program.body) {
    if (statement.type !== "ImportDeclaration" || statement.source.value !== "ink") continue;
    for (const specifier of statement.specifiers) {
      if (specifier.type === "ImportSpecifier" && specifier.imported.type === "Identifier") {
        names.set(specifier.local.name, specifier.imported.name);
      }
    }
  }
  return names;
}

/** Returns the Ink component an element renders, or null for other elements. */
export function inkElementName(element: ESTree.JSXElement, imports: Map<string, string>): string | null {
  const name = element.openingElement.name;
  return name.type === "JSXIdentifier" ? (imports.get(name.name) ?? null) : null;
}

export function attribute(element: ESTree.JSXElement, name: string): ESTree.JSXAttribute | null {
  for (const item of element.openingElement.attributes) {
    if (item.type === "JSXAttribute" && item.name.type === "JSXIdentifier" && item.name.name === name) return item;
  }
  return null;
}

export function stringAttribute(element: ESTree.JSXElement, name: string): string | null {
  const value = attribute(element, name)?.value;
  return value?.type === "Literal" && typeof value.value === "string" ? value.value : null;
}

/** Children that render something, without the whitespace between JSX tags. */
export function meaningfulChildren(element: ESTree.JSXElement): ESTree.JSXChild[] {
  return element.children.filter(child =>
    !(child.type === "JSXText" && child.value.trim() === "") &&
    !(child.type === "JSXExpressionContainer" && child.expression.type === "JSXEmptyExpression"));
}

function constInitialiser(sourceCode: SourceCode, identifier: ESTree.IdentifierReference): ESTree.Expression | null {
  let scope: Scope | null = sourceCode.getScope(identifier);
  while (scope !== null) {
    const variable = scope.set.get(identifier.name);
    if (variable !== undefined) {
      const [definition] = variable.defs;
      if (variable.defs.length !== 1 || definition?.type !== "Variable") return null;
      const declarator = definition.node;
      return declarator.type === "VariableDeclarator" && definition.parent.kind === "const" ? declarator.init : null;
    }
    scope = scope.upper;
  }
  return null;
}

/** Whether an expression depends on an action or resource being pending. */
export function dependsOnPending(sourceCode: SourceCode, expression: ESTree.Expression): boolean {
  switch (expression.type) {
    case "ParenthesizedExpression":
      return dependsOnPending(sourceCode, expression.expression);
    case "UnaryExpression":
      return dependsOnPending(sourceCode, expression.argument);
    case "LogicalExpression":
      return dependsOnPending(sourceCode, expression.left) || dependsOnPending(sourceCode, expression.right);
    case "BinaryExpression":
      return [expression.left, expression.right].some(side => side.type === "Literal" && side.value === "pending");
    case "Identifier": {
      const initialiser = constInitialiser(sourceCode, expression);
      return initialiser !== null && dependsOnPending(sourceCode, initialiser);
    }
    default:
      return false;
  }
}
