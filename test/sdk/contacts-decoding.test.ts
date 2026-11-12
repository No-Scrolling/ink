import { expect, test } from "bun:test";
import { decodeContactDetails, decodeContacts } from "../../packages/contacts/src/decode";
import { NativeError } from "../../packages/ink/src/native";

const details = {
  id: "12:0r12-ABC",
  name: "Ada Lovelace",
  phones: [{ label: "Mobile", value: "+44 20 7946 0000" }],
  emails: [],
  addresses: [{ label: "Home", value: "12 St James's Square, London" }],
  websites: [],
  events: [{ label: "Birthday", value: "1815-12-10" }],
  organization: null,
  note: "Met at the Analytical Society",
};

function rejects(decode: () => unknown) {
  try {
    decode();
    throw Error("Expected a protocol failure");
  } catch (error) {
    expect(error).toBeInstanceOf(NativeError);
    expect((error as NativeError).kind).toBe("protocol");
  }
}

test("contact decoding preserves lists, details and the deleted-contact null result", () => {
  const list = [{ id: "1:a", name: "Ada" }, { id: "2:b", name: "Bob" }];
  expect(decodeContacts(JSON.stringify(list))).toEqual(list);
  expect(decodeContacts("[]")).toEqual([]);
  expect(decodeContactDetails(JSON.stringify(details))).toEqual(details);
  expect(decodeContactDetails("null")).toBeNull();
});

test.each([
  null, {}, [{ id: 1, name: "Ada" }], [{ id: "1:a" }], [{ id: "1:a", name: null }], [null], "contacts",
].map(value => ({ value })))("contact lists reject malformed native values: %j", ({ value }) => {
  rejects(() => decodeContacts(JSON.stringify(value)));
});

test.each([
  { ...details, id: 12 }, { ...details, name: undefined }, { ...details, phones: undefined },
  { ...details, emails: [{ label: "Home" }] }, { ...details, addresses: [{ label: 1, value: "London" }] },
  { ...details, websites: "https://example.com" }, { ...details, events: [null] },
  { ...details, organization: undefined }, { ...details, note: 7 }, [], "contact",
].map(value => ({ value })))("contact details reject malformed native values: %j", ({ value }) => {
  rejects(() => decodeContactDetails(JSON.stringify(value)));
});
