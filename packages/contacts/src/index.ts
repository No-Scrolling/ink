import { callNative } from "ink/native";
import { collectContacts, decodeContactDetails } from "./decode";

export type Contact = { id: string; name: string };
export type LabeledValue = { label: string; value: string };
export type ContactDetails = {
  id: string;
  name: string;
  phones: LabeledValue[];
  emails: LabeledValue[];
  addresses: LabeledValue[];
  websites: LabeledValue[];
  events: LabeledValue[];
  organization: string | null;
  note: string | null;
};

// Each page is one bridge message, capped at 1 MiB. Rows are typically 100–200 bytes, so most address books
// take one call; Android also ends a page at 512 KiB, which covers unusually long names and lookup keys.
const PAGE_SIZE = 2000;

export const contacts = {
  /** Every contact with a name, sorted by name. `query` filters names the way Android's contact search does. */
  async list(options: { query?: string; signal?: AbortSignal } = {}): Promise<Contact[]> {
    return collectContacts(offset => callNative("contacts", "list", { query: options.query, offset, limit: PAGE_SIZE }, { timeoutMs: 120_000, signal: options.signal }), options.signal);
  },
  /** Resolves with the contact's details, or null when it no longer exists. */
  async get(id: string, options: { signal?: AbortSignal } = {}): Promise<ContactDetails | null> {
    return decodeContactDetails(await callNative("contacts", "get", { id }, { signal: options.signal }));
  },
};
