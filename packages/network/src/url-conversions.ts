/// <reference lib="es2024.string" />
// The Web IDL conversions used by whatwg-url 14's URL and URLSearchParams.
// Keep string-hint coercion and the caller's realm and error context.
interface Options {
  globals?: { String: StringConstructor; TypeError: TypeErrorConstructor };
  context?: string;
  treatNullAsEmptyString?: boolean;
}

export function USVString(value: unknown, options: Options = {}): string {
  if (options.treatNullAsEmptyString && value === null) return "";
  if (typeof value === "symbol") {
    const ErrorType = options.globals ? options.globals.TypeError : TypeError;
    throw new ErrorType(`${options.context ? options.context : "Value"} is a symbol, which cannot be converted to a string.`);
  }
  const StringType = options.globals ? options.globals.String : String;
  return StringType(value).toWellFormed();
}

export function any(value: unknown): unknown { return value; }
