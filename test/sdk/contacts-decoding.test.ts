import { expect, test } from "bun:test";
import { collectContacts, decodeContactDetails, decodeContactPage } from "../../packages/contacts/src/decode";
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

test("contact decoding preserves pages, details and the deleted-contact null result", () => {
  const contacts = [{ id: "1:a", name: "Ada" }, { id: "2:b", name: "Bob" }];
  expect(decodeContactPage(JSON.stringify({ contacts, next: 2 }), 0)).toEqual({ contacts, next: 2 });
  expect(decodeContactPage(JSON.stringify({ contacts: [], next: null }), 2000)).toEqual({ contacts: [], next: null });
  expect(decodeContactDetails(JSON.stringify(details))).toEqual(details);
  expect(decodeContactDetails("null")).toBeNull();
});

test.each([
  null, {}, [], "contacts", { contacts: [] }, { next: null }, { contacts: {}, next: null },
  { contacts: [{ id: 1, name: "Ada" }], next: null }, { contacts: [{ id: "1:a" }], next: null },
  { contacts: [{ id: "1:a", name: null }], next: null }, { contacts: [null], next: null },
  { contacts: [], next: "2000" }, { contacts: [], next: 2000.5 }, { contacts: [], next: Number.MAX_SAFE_INTEGER + 1 },
].map(value => ({ value })))("contact pages reject malformed native values: %j", ({ value }) => {
  rejects(() => decodeContactPage(JSON.stringify(value), 0));
});

function pages(...pages: { contacts: { id: string; name: string }[]; next: number | null }[]) {
  const offsets: number[] = [];
  const load = async (offset: number) => {
    offsets.push(offset);
    return JSON.stringify(pages[offsets.length - 1]);
  };
  return { offsets, load };
}

test("contact pages join in order from the offsets native returns", async () => {
  const { offsets, load } = pages(
    { contacts: [{ id: "1:a", name: "Ada" }, { id: "2:b", name: "Bob" }], next: 3 },
    { contacts: [{ id: "3:c", name: "Cy" }], next: 4 },
    { contacts: [{ id: "4:d", name: "Di" }], next: null },
  );
  expect(await collectContacts(load)).toEqual([
    { id: "1:a", name: "Ada" }, { id: "2:b", name: "Bob" }, { id: "3:c", name: "Cy" }, { id: "4:d", name: "Di" },
  ]);
  expect(offsets).toEqual([0, 3, 4]);
});

test("a contact pushed onto the next page by an insertion is listed once", async () => {
  const { load } = pages(
    { contacts: [{ id: "1:a", name: "Ada" }, { id: "2:b", name: "Bob" }], next: 2 },
    { contacts: [{ id: "2:b", name: "Bob" }, { id: "3:c", name: "Cy" }], next: null },
  );
  expect(await collectContacts(load)).toEqual([{ id: "1:a", name: "Ada" }, { id: "2:b", name: "Bob" }, { id: "3:c", name: "Cy" }]);
});

test("aborting between pages stops before the next request", async () => {
  const controller = new AbortController();
  const reason = new Error("left the screen");
  const { offsets, load } = pages(
    { contacts: [{ id: "1:a", name: "Ada" }], next: 1 },
    { contacts: [{ id: "2:b", name: "Bob" }], next: null },
  );
  await expect(collectContacts(async offset => {
    const page = await load(offset);
    controller.abort(reason);
    return page;
  }, controller.signal)).rejects.toBe(reason);
  expect(offsets).toEqual([0]);
});

test.each([1, 0])("a page whose next offset does not advance past %i is a protocol error", async next => {
  const { offsets, load } = pages(
    { contacts: [{ id: "1:a", name: "Ada" }], next: 1 },
    { contacts: [{ id: "1:a", name: "Ada" }], next },
  );
  const result = collectContacts(load);
  await expect(result).rejects.toBeInstanceOf(NativeError);
  await expect(result).rejects.toHaveProperty("kind", "protocol");
  expect(offsets).toEqual([0, 1]);
});

test.each([
  { ...details, id: 12 }, { ...details, name: undefined }, { ...details, phones: undefined },
  { ...details, emails: [{ label: "Home" }] }, { ...details, addresses: [{ label: 1, value: "London" }] },
  { ...details, websites: "https://example.com" }, { ...details, events: [null] },
  { ...details, organization: undefined }, { ...details, note: 7 }, [], "contact",
].map(value => ({ value })))("contact details reject malformed native values: %j", ({ value }) => {
  rejects(() => decodeContactDetails(JSON.stringify(value)));
});
