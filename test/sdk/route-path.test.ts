import { expect, test } from "bun:test";
import { comparePaths, matchPath, parameterNames } from "../../packages/ink/src/route-path";

test("route matching respects whole segments and decodes each captured parameter once", () => {
  expect(matchPath("/notes/[id]/[section]", "/notes/a%2Fb/%2520")).toEqual({ id: "a/b", section: "%20" });
  expect(matchPath("/literal.+/[id]", "/literal.+/42")).toEqual({ id: "42" });
  expect(matchPath("/literal.+/[id]", "/literalXX/42")).toBeUndefined();
  expect(matchPath("/notes/[id]", "/notes/")).toBeUndefined();
  expect(matchPath("/notes/[id]", "/notes/42/more")).toBeUndefined();
  expect(matchPath("/notes/[id]", "/notes/%E0%A4%A")).toBeUndefined();
  expect(matchPath("/", "/")).toEqual({});
  expect(parameterNames("/notes/[note_id]/[section2]")).toEqual(["note_id", "section2"]);
});

test("specificity compares each depth rather than counting all parameters", () => {
  const routes = ["/[root]/about", "/notes/[id]", "/notes/new", "/[root]/[leaf]"];
  expect(routes.sort(comparePaths)).toEqual(["/notes/new", "/notes/[id]", "/[root]/about", "/[root]/[leaf]"]);
  expect(comparePaths("/notes/[id]", "/notes/[id]")).toBe(0);
});

test("all accepted parameter names remain own properties in matched route data", () => {
  expect(parameterNames("/[__proto__]/[constructor]")).toEqual(["__proto__", "constructor"]);
  const params = matchPath("/[__proto__]/[constructor]", "/note/section");
  expect(Object.hasOwn(params!, "__proto__")).toBe(true);
  expect(params!["__proto__"]).toBe("note");
  expect(params!.constructor).toBe("section");
  expect(Object.getPrototypeOf(params)).toBe(Object.prototype);
  expect(JSON.stringify(params)).toBe('{"__proto__":"note","constructor":"section"}');
});
