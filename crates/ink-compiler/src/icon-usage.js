// Only shrink assets whose uses have a known size. Passing an icon through user
// code keeps the default resolution rather than guessing what that code renders.
export async function collectIconSizes(ts, path, contents, materialIcons, resolveSvg) {
  const source = ts.createSourceFile(path, contents, ts.ScriptTarget.Latest, true,
    /\.[jt]sx$/.test(path) ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
  const host = {
    getSourceFile: name => name === path ? source : undefined,
    getDefaultLibFileName: () => "", writeFile() {}, getCurrentDirectory: () => "",
    getDirectories: () => [], fileExists: name => name === path,
    readFile: name => name === path ? contents : undefined,
    getCanonicalFileName: name => name, useCaseSensitiveFileNames: () => true,
    getNewLine: () => "\n",
  };
  const checker = ts.createProgram([path], { noLib: true, noResolve: true }, host).getTypeChecker();
  const icons = new Map();
  const components = new Map();
  const sizes = new Map();
  for (const statement of source.statements) {
    if (ts.isExportDeclaration(statement) && statement.moduleSpecifier) {
      const module = statement.moduleSpecifier.text;
      if (module === "ink/icons") {
        for (const reference of materialIcons.values()) sizes.set(reference, 56);
      } else if (module.endsWith(".svg")) sizes.set(await resolveSvg(module), 56);
    }
    if (!ts.isImportDeclaration(statement) || !statement.importClause) continue;
    const module = statement.moduleSpecifier.text;
    const clause = statement.importClause;
    if (clause.isTypeOnly) continue;
    if (module.endsWith(".svg") && clause.name) {
      icons.set(checker.getSymbolAtLocation(clause.name), await resolveSvg(module));
    }
    const bindings = clause.namedBindings;
    if (module === "ink/icons" && bindings && ts.isNamespaceImport(bindings)) {
      for (const reference of materialIcons.values()) sizes.set(reference, 56);
    }
    if (!bindings || !ts.isNamedImports(bindings)) continue;
    for (const binding of bindings.elements) {
      if (binding.isTypeOnly) continue;
      const name = (binding.propertyName ?? binding.name).text;
      const symbol = checker.getSymbolAtLocation(binding.name);
      if (module === "ink") components.set(symbol, name);
      if (module === "ink/icons") icons.set(symbol, materialIcons.get(name));
    }
  }
  function sizeAt(node) {
    const expression = node.parent;
    const attribute = expression?.parent;
    if (!expression || !ts.isJsxExpression(expression) || !attribute || !ts.isJsxAttribute(attribute)) return 56;
    const attributes = attribute.parent;
    const opening = attributes.parent;
    const component = components.get(checker.getSymbolAtLocation(opening.tagName));
    if (attributes.properties.some(ts.isJsxSpreadAttribute)) return 56;
    if (component === "Row" && attribute.name.text === "subtitleIcon") return 16;
    if (component === "Message" && attribute.name.text === "statusIcon") return 14;
    if (component === "Button" && attribute.name.text === "icon") return 40;
    if ((component === "Icon" || component === "CanvasIcon") && attribute.name.text === "name") {
      const size = attributes.properties.find(prop => ts.isJsxAttribute(prop) && prop.name.text === "size");
      if (!size) return 28;
      const value = size.initializer;
      return value && ts.isJsxExpression(value) && value.expression && ts.isNumericLiteral(value.expression)
        ? Number(value.expression.text) : 56;
    }
    return 56;
  }
  function visit(node) {
    if (ts.isImportDeclaration(node)) return;
    if (ts.isIdentifier(node)) {
      const reference = icons.get(checker.getSymbolAtLocation(node));
      if (reference) sizes.set(reference, Math.max(sizes.get(reference) ?? 0, sizeAt(node)));
    }
    ts.forEachChild(node, visit);
  }
  visit(source);
  return sizes;
}
