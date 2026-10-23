// Recognise only fixed field operations. Runtime guards retain responsibility
// for data descriptors, builtins and the original method's observable behaviour.
export function compileDataViews(ts, path, contents) {
  if (!/\.(?:filter|sort)\s*\(/.test(contents)) return contents;
  const source = ts.createSourceFile(path, contents, ts.ScriptTarget.Latest, true);
  if (source.parseDiagnostics.length) return contents;
  const host = { getSourceFile: name => name === path ? source : undefined, getDefaultLibFileName: () => "",
    writeFile() {}, getCurrentDirectory: () => "", getDirectories: () => [], fileExists: name => name === path,
    readFile: name => name === path ? contents : undefined, getCanonicalFileName: name => name,
    useCaseSensitiveFileNames: () => true, getNewLine: () => "\n" };
  const checker = ts.createProgram([path], { noLib: true, noResolve: true }, host).getTypeChecker();
  const unwrap = node => ts.isParenthesizedExpression(node) ? unwrap(node.expression) : node;
  const text = node => node.getText(source);
  const identifiers = new Set();
  let dynamicScope = false;
  function inspect(node) {
    if (ts.isIdentifier(node)) identifiers.add(node.text);
    // Direct eval and with can introduce bindings invisible to the checker.
    if (ts.isWithStatement(node) || ts.isCallExpression(node)
      && ts.isIdentifier(unwrap(node.expression)) && unwrap(node.expression).text === "eval") dynamicScope = true;
    ts.forEachChild(node, inspect);
  }
  inspect(source);
  if (dynamicScope) return contents;
  let prefix = "__inkData";
  while ([...identifiers].some(name => name.startsWith(prefix))) prefix += "_";
  const replacements = new Map(), helpers = new Set();
  const optional = node => !!(node.questionDotToken || node.flags & ts.NodeFlags.OptionalChain);
  const string = node => ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node);
  const binary = (node, operator) => ts.isBinaryExpression(node) && node.operatorToken.kind === operator;
  function method(node, name, arity) {
    node = unwrap(node);
    if (!ts.isCallExpression(node) || optional(node) || node.typeArguments?.length
      || node.arguments.length !== arity || node.arguments.some(ts.isSpreadElement)) return;
    const callee = unwrap(node.expression);
    if (ts.isPropertyAccessExpression(callee) && !optional(callee) && callee.name.text === name) return callee.expression;
  }
  function callback(node, arity) {
    node = unwrap(node);
    if (!ts.isArrowFunction(node) || node.modifiers?.length || node.typeParameters?.length
      || ts.isBlock(node.body) || node.parameters.length !== arity
      || node.parameters.some(parameter => !ts.isIdentifier(parameter.name) || parameter.initializer
        || parameter.dotDotDotToken || parameter.questionToken || parameter.modifiers?.length)) return;
    const symbols = node.parameters.map(parameter => checker.getSymbolAtLocation(parameter.name));
    if (symbols.some(symbol => !symbol) || new Set(symbols).size !== arity) return;
    return { body: unwrap(node.body), symbols };
  }
  function field(node, symbols) {
    node = unwrap(node);
    if (!ts.isPropertyAccessExpression(node) || optional(node) || !ts.isIdentifier(node.name)) return;
    const receiver = unwrap(node.expression);
    if (!ts.isIdentifier(receiver)) return;
    const symbol = checker.getSymbolAtLocation(receiver);
    if (symbols.includes(symbol)) return { name: node.name.text, symbol };
  }
  function localQuery(node, symbols) {
    node = unwrap(node);
    if (!ts.isIdentifier(node)) return;
    const symbol = checker.getSymbolAtLocation(node);
    if (!symbol || symbols.includes(symbol) || symbol.flags & ts.SymbolFlags.Alias) return;
    let declaration = symbol.valueDeclaration;
    if (!declaration || ![ts.SyntaxKind.VariableDeclaration, ts.SyntaxKind.Parameter, ts.SyntaxKind.BindingElement].includes(declaration.kind)) return;
    let local = ts.isExternalModule(source);
    // Ambient/global properties and namespace members are not lexical captures.
    while (declaration && !ts.isSourceFile(declaration)) {
      if (ts.isModuleDeclaration(declaration) || declaration.modifiers?.some(modifier => modifier.kind === ts.SyntaxKind.DeclareKeyword)) return;
      if (ts.isFunctionLike(declaration) || ts.isVariableDeclarationList(declaration)
        && declaration.flags & ts.NodeFlags.BlockScoped) local = true;
      declaration = declaration.parent;
    }
    if (local) return text(node);
  }
  function search(argument) {
    const arrow = callback(argument, 1);
    if (!arrow) return;
    const lowered = method(arrow.body, "includes", 1);
    if (!lowered) return;
    const value = method(lowered, "toLowerCase", 0);
    if (!value) return;
    const joined = method(value, "join", 1);
    if (!joined || !ts.isArrayLiteralExpression(unwrap(joined))) return;
    const join = unwrap(value), delimiter = unwrap(join.arguments[0]);
    if (!string(delimiter)) return;
    const elements = unwrap(joined).elements.map(element => field(element, arrow.symbols));
    if (elements.length < 2 || elements.some(element => !element)) return;
    const fields = elements.map(element => element.name), separator = delimiter.text;
    const query = unwrap(arrow.body.arguments[0]);
    let capture, queryLower = false;
    if (string(query)) capture = text(query);
    else {
      const raw = method(query, "toLowerCase", 0);
      capture = localQuery(raw ?? query, arrow.symbols);
      if (raw) queryLower = true;
    }
    if (capture === undefined) return;
    return { plan: { fields, separator, ...(queryLower ? { queryLower: true } : {}) }, capture };
  }
  function dateKey(node, symbols) {
    node = unwrap(node);
    if (!binary(node, ts.SyntaxKind.BarBarToken)) return;
    const zero = unwrap(node.right);
    if (!ts.isNumericLiteral(zero) || Number(zero.text) !== 0) return;
    const parse = unwrap(node.left), date = method(parse, "parse", 1);
    if (!date || !ts.isIdentifier(unwrap(date)) || unwrap(date).text !== "Date"
      || checker.getSymbolAtLocation(unwrap(date))) return;
    const value = unwrap(parse.arguments[0]);
    if (!binary(value, ts.SyntaxKind.QuestionQuestionToken)) return;
    const empty = unwrap(value.right);
    if (!string(empty) || empty.text !== "") return;
    return field(value.left, symbols);
  }
  function sort(argument) {
    const arrow = callback(argument, 2);
    if (!arrow) return;
    let difference = arrow.body, tie;
    if (binary(difference, ts.SyntaxKind.BarBarToken)) {
      const compare = unwrap(difference.right), left = method(compare, "localeCompare", 1);
      if (!left) return;
      const first = field(left, arrow.symbols), second = field(compare.arguments[0], arrow.symbols);
      if (!first || !second || first.symbol !== arrow.symbols[0] || second.symbol !== arrow.symbols[1]
        || first.name !== second.name) return;
      tie = first.name;
      difference = unwrap(difference.left);
    }
    if (!binary(difference, ts.SyntaxKind.MinusToken)) return;
    const left = dateKey(difference.left, arrow.symbols), right = dateKey(difference.right, arrow.symbols);
    if (!left || !right || left.name !== right.name || left.symbol === right.symbol) return;
    return { date: left.name, direction: left.symbol === arrow.symbols[0] ? 1 : -1, ...(tie === undefined ? {} : { tie }) };
  }
  function render(node) {
    const edits = [];
    function collect(child) {
      if (replacements.has(child)) edits.push([child.getStart(source), child.end, replacements.get(child)]);
      else ts.forEachChild(child, collect);
    }
    collect(node);
    let result = text(node);
    const offset = node.getStart(source);
    for (const [start, end, replacement] of edits.sort((a, b) => b[0] - a[0])) {
      result = result.slice(0, start - offset) + replacement + result.slice(end - offset);
    }
    return result;
  }
  function visit(node) {
    ts.forEachChild(node, visit);
    if (!ts.isCallExpression(node)) return;
    for (const [name, recognise, helper] of [["filter", search, "Search"], ["sort", sort, "Sort"]]) {
      const receiver = method(node, name, 1);
      if (!receiver || unwrap(receiver).kind === ts.SyntaxKind.SuperKeyword) continue;
      const plan = recognise(node.arguments[0]);
      if (!plan) return;
      const arguments_ = [render(receiver), JSON.stringify(JSON.stringify(helper === "Search" ? plan.plan : plan))];
      if (helper === "Search") arguments_.push(`() => ${plan.capture}`);
      // A unique parameter keeps every original callback capture in its scope.
      arguments_.push(`${prefix}Items => ${prefix}Items.${name}(${text(node.arguments[0])})`);
      replacements.set(node, `${prefix}${helper}(${arguments_.join(", ")})`);
      helpers.add(helper);
      return;
    }
  }
  visit(source);
  if (!replacements.size) return contents;
  // Preserve leading comments and hashbangs as well as untouched callback text.
  const transformed = contents.slice(0, source.getStart(source)) + render(source);
  const imports = `import { ${[...helpers].map(helper => `compiled${helper} as ${prefix}${helper}`).join(", ")} } from "ink/internal/data";\n`;
  let start = transformed.startsWith("#!") ? transformed.indexOf("\n") + 1 : 0;
  for (const statement of source.statements) {
    if (!ts.isExpressionStatement(statement) || !ts.isStringLiteral(statement.expression)) break;
    start = statement.end;
  }
  const newline = start && transformed[start - 1] !== "\n" ? "\n" : "";
  return transformed.slice(0, start) + newline + imports + transformed.slice(start);
}
