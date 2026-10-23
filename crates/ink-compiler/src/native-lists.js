// Compile only declarative rows. Unknown components and executable render logic
// keep the ordinary React path, including hooks and component lifetimes.
export function compileNativeLists(ts, path, contents) {
  if (!contents.includes("renderItem")) return contents;
  const source = ts.createSourceFile(path, contents, ts.ScriptTarget.Latest, true);
  const host = { getSourceFile: name => name === path ? source : undefined, getDefaultLibFileName: () => "",
    writeFile() {}, getCurrentDirectory: () => "", getDirectories: () => [], fileExists: name => name === path,
    readFile: name => name === path ? contents : undefined, getCanonicalFileName: name => name,
    useCaseSensitiveFileNames: () => true, getNewLine: () => "\n" };
  const checker = ts.createProgram([path], { noLib: true, noResolve: true }, host).getTypeChecker();
  const imports = new Map();
  const wrapperReferences = new Map();
  for (const statement of source.statements) {
    if (!ts.isImportDeclaration(statement) || statement.moduleSpecifier.text !== "ink" || statement.importClause?.isTypeOnly) continue;
    const bindings = statement.importClause?.namedBindings;
    if (!bindings || !ts.isNamedImports(bindings)) continue;
    for (const binding of bindings.elements) if (!binding.isTypeOnly) imports.set(checker.getSymbolAtLocation(binding.name), (binding.propertyName ?? binding.name).text);
  }
  let prefix = "__inkList";
  while (contents.includes(prefix)) prefix += "_";
  const edits = [];
  const text = node => node.getText(source);
  const unwrap = node => ts.isParenthesizedExpression(node) ? unwrap(node.expression) : node;
  function pure(node, action = false) {
    node = unwrap(node);
    if (action && ts.isArrowFunction(node)) return true;
    if (ts.isIdentifier(node) || ts.isLiteralExpression(node) || [ts.SyntaxKind.TrueKeyword, ts.SyntaxKind.FalseKeyword, ts.SyntaxKind.NullKeyword].includes(node.kind)) return true;
    if (ts.isPropertyAccessExpression(node)) return pure(node.expression);
    if (ts.isElementAccessExpression(node)) return pure(node.expression) && pure(node.argumentExpression);
    if (ts.isTemplateExpression(node)) return node.templateSpans.every(span => pure(span.expression));
    if (ts.isBinaryExpression(node)) return !ts.isAssignmentOperator(node.operatorToken.kind) && pure(node.left) && pure(node.right);
    if (ts.isConditionalExpression(node)) return pure(node.condition) && pure(node.whenTrue) && pure(node.whenFalse);
    if (ts.isObjectLiteralExpression(node)) return node.properties.every(p => ts.isPropertyAssignment(p) && !ts.isComputedPropertyName(p.name) && pure(p.initializer));
    if (ts.isPrefixUnaryExpression(node)) return ![ts.SyntaxKind.PlusPlusToken, ts.SyntaxKind.MinusMinusToken].includes(node.operator) && pure(node.operand);
    return false;
  }
  function compile(callback) {
    callback = unwrap(callback);
    if (!ts.isArrowFunction(callback) || callback.modifiers?.length || ts.isBlock(callback.body)
      || callback.parameters.some(p => !ts.isIdentifier(p.name) || p.initializer || p.dotDotDotToken)) return;
    const identity = `${path}:${callback.getStart(source)}`;
    const fields = [], captures = new Set(), preludes = [];
    const aliases = new Map(), wrappers = new Set(); let next = 1;
    function text(node) {
      const replacements = [];
      function visit(current) {
        const alias = ts.isIdentifier(current) && aliases.get(checker.getSymbolAtLocation(current));
        if (alias) replacements.push([current.getStart(source), current.end, alias.code]);
        else ts.forEachChild(current, visit);
      }
      visit(node);
      let result = node.getText(source);
      const offset = node.getStart(source);
      for (const [start, end, value] of replacements.sort((a,b) => b[0]-a[0])) {
        result = result.slice(0,start-offset) + value + result.slice(end-offset);
      }
      return result;
    }
    const parameters = new Set(callback.parameters.map(p => checker.getSymbolAtLocation(p.name)));
    const indexParameter = callback.parameters[1] && checker.getSymbolAtLocation(callback.parameters[1].name);
    let usesIndex = false;
    function capture(expression, caller = false) {
      // Event arrows run through the latest projector at dispatch, not when
      // preparing displayed values. Their bodies do not affect move reuse.
      if (ts.isArrowFunction(expression)) return;
      if (ts.isPropertyAccessExpression(expression)) return capture(expression.expression, caller);
      if (ts.isPropertyAssignment(expression)) return capture(expression.initializer, caller);
      if (ts.isIdentifier(expression)) {
        const symbol = checker.getSymbolAtLocation(expression);
        const alias = aliases.get(symbol);
        if (alias) { if (alias.source) capture(alias.source, true); return; }
        if (indexParameter && symbol === indexParameter) usesIndex = true;
        if (!parameters.has(symbol)) {
          if (wrappers.size && !caller) throw Error("wrapper captures need React");
          captures.add(text(expression));
        }
        return;
      }
      ts.forEachChild(expression, child => capture(child, caller));
    }
    const bind = expression => { const slot = fields.push(expression) - 1; return { $value: 0, path: ["values", slot] }; };
    function node(element) {
      if (next > 4096) throw Error("native row template is too large");
      element = unwrap(element);
      const opening = ts.isJsxElement(element) ? element.openingElement : ts.isJsxSelfClosingElement(element) ? element : null;
      if (!opening) throw Error("not a fixed JSX row");
      const symbol = checker.getSymbolAtLocation(opening.tagName);
      const kind = imports.get(symbol);
      if (!kind) {
        const declaration = symbol?.valueDeclaration;
        if (!declaration || !ts.isVariableDeclaration(declaration)
          || !(declaration.parent.flags & ts.NodeFlags.Const)
          || declaration.parent.parent.parent !== source
          || declaration.parent.parent.modifiers?.some(modifier => modifier.kind === ts.SyntaxKind.ExportKeyword)) throw Error("component needs React");
        if (!wrapperReferences.has(symbol)) {
          let direct = true;
          function references(current) {
            if (ts.isIdentifier(current) && checker.getSymbolAtLocation(current) === symbol
              && current !== declaration.name
              && !(ts.isJsxOpeningElement(current.parent) || ts.isJsxSelfClosingElement(current.parent)
                || ts.isJsxClosingElement(current.parent))) direct = false;
            ts.forEachChild(current, references);
          }
          references(source);
          wrapperReferences.set(symbol, direct);
        }
        if (!wrapperReferences.get(symbol)) throw Error("wrapper references need React");
        const wrapper = declaration.initializer;
        if (!wrapper || !ts.isArrowFunction(wrapper) || wrapper.modifiers?.length || wrapper.parameters.length !== 1
          || !ts.isObjectBindingPattern(wrapper.parameters[0].name) || wrapper.parameters[0].initializer
          || wrapper.parameters[0].dotDotDotToken
          || ts.isBlock(wrapper.body) || wrappers.has(symbol) || wrappers.size >= 4) throw Error("wrapper needs React");
        if (ts.isJsxElement(element) && element.children.some(child => !ts.isJsxText(child) || child.text.trim())) throw Error("wrapper children need React");
        const props = [], arguments_ = new Map();
        for (const attribute of opening.attributes.properties) {
          if (!ts.isJsxAttribute(attribute) || ["ref", "key", "children"].includes(attribute.name.text)) throw Error("wrapper properties need React");
          const name = attribute.name.text, value = attribute.initializer;
          if (!value) props.push(`${JSON.stringify(name)}:true`);
          else if (ts.isStringLiteral(value) && !value.text.includes("&")) props.push(`${JSON.stringify(name)}:${JSON.stringify(value.text)}`);
          else if (ts.isJsxExpression(value) && value.expression && pure(value.expression, name.startsWith("on"))) {
            props.push(`${JSON.stringify(name)}:${text(value.expression)}`); capture(value.expression);
            arguments_.set(name, value.expression);
          } else throw Error("wrapper property needs React");
        }
        const temporary = `${prefix}Props${preludes.length}`;
        preludes.push([fields.length, `const ${temporary}={${props.join(",")}};`]);
        const added = [];
        for (const binding of wrapper.parameters[0].name.elements) {
          if (!ts.isIdentifier(binding.name) || binding.initializer || binding.dotDotDotToken
            || binding.propertyName && !ts.isIdentifier(binding.propertyName)) throw Error("wrapper binding needs React");
          const name = (binding.propertyName ?? binding.name).text;
          const symbol = checker.getSymbolAtLocation(binding.name);
          aliases.set(symbol, { code:`${temporary}.${name}`, source:arguments_.get(name) }); added.push(symbol);
        }
        wrappers.add(symbol);
        try { return node(wrapper.body); }
        finally { wrappers.delete(symbol); for (const symbol of added) aliases.delete(symbol); }
      }
      if (!["Text", "Stack", "Row", "Image", "Icon", "Avatar", "Toggle", "Button", "Field"].includes(kind)) throw Error("component needs React");
      if (kind === "Row") {
        const props = [];
        for (const attribute of opening.attributes.properties) {
          if (!ts.isJsxAttribute(attribute) || !["image","title","titleMaxLines","titleIcon","subtitle","subtitleIcon","href","url","onPress","onLongPress"].includes(attribute.name.text)) throw Error("unsupported Row property");
          const name = attribute.name.text, value = attribute.initializer;
          if (value && ts.isStringLiteral(value) && !value.text.includes("&")) { props.push(`${name}:${JSON.stringify(value.text)}`); }
          else if (value && ts.isJsxExpression(value) && value.expression && pure(value.expression, name.startsWith("on"))) {
            if (wrappers.size && ts.isArrowFunction(unwrap(value.expression))) throw Error("wrapper actions need React");
            props.push(`${name}:${text(value.expression)}`); capture(value.expression);
          }
          else throw Error("Row property needs React");
        }
        if (ts.isJsxElement(element) && element.children.some(child => !ts.isJsxText(child) || child.text.trim())) throw Error("Row children need React");
        const slot = fields.push(`${prefix}Fields({${props.join(",")}})`) - 1;
        const field = name => ({ $value: 0, path: ["values", slot, name] });
        const host = (type, props, children = []) => ({ id: next++, type, props, children });
        return host("RowContent", Object.fromEntries(
          ["image", "title", "titleMaxLines", "titleIcon", "subtitle", "subtitleIcon", "onPress", "onLongPress"].map(name => [name, field(name)])));
      }
      const result = { id: next++, type: kind, props: {}, children: [] };
      for (const attribute of opening.attributes.properties) {
        if (!ts.isJsxAttribute(attribute)) throw Error("spread properties need React");
        const name = attribute.name.text;
        if (["ref", "key", "href", "url", "children"].includes(name)) throw Error("special property needs React");
        const value = attribute.initializer;
        if (!value) result.props[name] = true;
        else if (ts.isStringLiteral(value) && !value.text.includes("&")) result.props[name] = value.text;
        else if (ts.isJsxExpression(value) && value.expression && pure(value.expression, name.startsWith("on"))) {
          if (wrappers.size && ts.isArrowFunction(unwrap(value.expression))) throw Error("wrapper actions need React");
          result.props[name] = bind(text(value.expression)); capture(value.expression);
        }
        else throw Error("property needs React");
      }
      const children = ts.isJsxElement(element) ? element.children : [];
      if (["Text", "Button", "Field"].includes(kind)) {
        const parts = [];
        for (const child of children) {
          if (ts.isJsxText(child)) {
            // Let React retain responsibility for multiline JSX whitespace/entities.
            if (/[\r\n&]/.test(child.text)) { if (child.text.trim()) throw Error("complex JSX text"); }
            else parts.push(JSON.stringify(child.text));
          } else if (ts.isJsxExpression(child) && child.expression && pure(child.expression)) { parts.push(text(child.expression)); capture(child.expression); }
          else if (ts.isJsxExpression(child) && !child.expression) continue;
          else throw Error("rich text needs React");
        }
        const content = bind(`${prefix}Text([${parts.join(",")}])`);
        if (kind === "Text") result.props.text = content;
        else result.children.push({ id: next++, type: "RawText", props: { text: content }, children: [] });
      } else {
        for (const child of children) {
          if (ts.isJsxText(child) && !child.text.trim()) continue;
          result.children.push(node(child));
        }
      }
      return result;
    }
    try {
      const template = { id: 0, type: "Stack", props: { gap: 0 }, children: [node(callback.body)] };
      const params = callback.parameters.map(text).join(",");
      let projection = `[${fields.join(",")}]`;
      if (preludes.length) {
        const values = fields.map((_, index) => `${prefix}Value${index}`);
        const statements = [];
        let prelude = 0;
        for (let slot = 0; slot <= fields.length; slot++) {
          while (prelude < preludes.length && preludes[prelude][0] === slot) statements.push(preludes[prelude++][1]);
          if (slot < fields.length) statements.push(`const ${values[slot]}=${fields[slot]};`);
        }
        projection = `{${statements.join("")}return [${values.join(",")}];}`;
      }
      return `${prefix}Row(${text(callback)}, (${params}) => ${projection}, ${JSON.stringify(JSON.stringify(template))}, {identity:${JSON.stringify(identity)},values:() => [${[...captures].join(",")}],usesIndex:${usesIndex}})`;
    } catch { return; }
  }
  function visit(node) {
    if ((ts.isJsxSelfClosingElement(node) || ts.isJsxOpeningElement(node)) && imports.get(checker.getSymbolAtLocation(node.tagName)) === "List") {
      const attr = node.attributes.properties.find(p => ts.isJsxAttribute(p) && p.name.text === "renderItem");
      const expression = attr?.initializer;
      if (expression && ts.isJsxExpression(expression) && expression.expression) {
        const replacement = compile(expression.expression);
        if (replacement) {
          edits.push([expression.expression.getStart(source), expression.expression.end, replacement]);
          const key = node.attributes.properties.find(p => ts.isJsxAttribute(p) && p.name.text === "keyExtractor")?.initializer;
          const extract = key && ts.isJsxExpression(key) && key.expression && unwrap(key.expression);
          if (extract && ts.isArrowFunction(extract) && extract.parameters.length === 1
            && ts.isIdentifier(extract.parameters[0].name) && !extract.parameters[0].initializer && !extract.parameters[0].dotDotDotToken) {
            const body = unwrap(extract.body);
            if (ts.isPropertyAccessExpression(body) && ts.isIdentifier(body.expression)
              && checker.getSymbolAtLocation(body.expression) === checker.getSymbolAtLocation(extract.parameters[0].name)) {
              edits.push([key.expression.getStart(source), key.expression.end,
                `${prefix}Key(${text(key.expression)}, ${JSON.stringify(`${path}:${key.expression.getStart(source)}`)})`]);
            }
          }
          return;
        }
      }
    }
    ts.forEachChild(node, visit);
  }
  visit(source);
  if (!edits.length) return contents;
  for (const [start, end, replacement] of edits.sort((a,b) => b[0]-a[0])) contents = contents.slice(0,start)+replacement+contents.slice(end);
  return `import { nativeListRow as ${prefix}Row, nativeListKey as ${prefix}Key, listText as ${prefix}Text, listRowFields as ${prefix}Fields } from "ink/internal/list";\n` + contents;
}
