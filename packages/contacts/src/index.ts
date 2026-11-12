import { callNative } from "ink/native";
import { decodeContactDetails, decodeContacts } from "./decode";

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

export const contacts = {
  /** Every contact with a name, sorted by name. `query` filters names the way Android's contact search does. */
  async list(options: { query?: string; signal?: AbortSignal } = {}): Promise<Contact[]> {
    return decodeContacts(await callNative("contacts", "list", { query: options.query }, { timeoutMs: 120_000, signal: options.signal }));
  },
  /** Resolves with the contact's details, or null when it no longer exists. */
  async get(id: string, options: { signal?: AbortSignal } = {}): Promise<ContactDetails | null> {
    return decodeContactDetails(await callNative("contacts", "get", { id }, { signal: options.signal }));
  },
};
