import { RECOMMENDED_RULES } from "oxlint-plugin-react-doctor";

export default {
  jsPlugins: [
    { name: "anti-slop", specifier: "./tools/oxlint/anti-slop/index.ts" },
    { name: "react-doctor", specifier: "oxlint-plugin-react-doctor" },
  ],
  settings: { "react-doctor": { portedRuleMode: "curated" } },
  rules: {
    ...RECOMMENDED_RULES,
    "react-doctor/only-export-components": "off",
    "react-doctor/context-provider-value-from-unmemoized-local-literal": "off",
    "anti-slop/no-chained-type-assertions": "error",
    "anti-slop/no-conditional-empty-object-spread": "error",
    "anti-slop/no-known-value-widening": "error",
    "anti-slop/no-module-mocking": "error",
    "anti-slop/no-object-parameters": "error",
    "anti-slop/no-reflect-apply": "error",
    "anti-slop/no-reflect-get": "error",
    "anti-slop/no-runtime-typeof": "error",
    "anti-slop/no-shape-in-symbol-names": "error",
    "anti-slop/no-unknown-parameters": "error",
    "anti-slop/no-unknown-returns": "error",
    "anti-slop/no-unknown-type-aliases": "error",
    "anti-slop/no-unsafe-dictionary-type": "error",
    "anti-slop/no-widen-then-assert": "error",
    "anti-slop/require-safety-comment-for-type-assertion": "error",
  },
};
