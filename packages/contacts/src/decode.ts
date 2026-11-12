import { NativeError } from "ink/native";
import type { Contact, ContactDetails, LabeledValue } from "./index";

const LISTS = ["phones", "emails", "addresses", "websites", "events"] as const;

function isContact(value: unknown): value is Contact {
  return typeof value === "object" && value !== null
    && typeof Reflect.get(value, "id") === "string" && typeof Reflect.get(value, "name") === "string";
}

function isLabeledValue(value: unknown): value is LabeledValue {
  return typeof value === "object" && value !== null
    && typeof Reflect.get(value, "label") === "string" && typeof Reflect.get(value, "value") === "string";
}

export function decodeContactPage(json: string, offset: number): { contacts: Contact[]; next: number | null } {
  const page: unknown = JSON.parse(json);
  if (typeof page !== "object" || page === null) throw new NativeError("protocol", "Invalid contact list");
  const contacts: unknown = Reflect.get(page, "contacts");
  const next: unknown = Reflect.get(page, "next");
  if (!Array.isArray(contacts) || !contacts.every(isContact) || !(next === null || typeof next === "number" && Number.isSafeInteger(next))) {
    throw new NativeError("protocol", "Invalid contact list");
  }
  if (next !== null && next <= offset) throw new NativeError("protocol", "Contact pagination did not advance");
  return { contacts, next };
}

/** Joins pages into one list. A contact that moves between pages while they load is kept once, where it first appeared. */
export async function collectContacts(load: (offset: number) => Promise<string>, signal?: AbortSignal): Promise<Contact[]> {
  const contacts = new Map<string, Contact>();
  for (let offset: number | null = 0; offset !== null;) {
    signal?.throwIfAborted();
    const page = decodeContactPage(await load(offset), offset);
    for (const contact of page.contacts) contacts.set(contact.id, contact);
    offset = page.next;
  }
  return [...contacts.values()];
}

export function decodeContactDetails(json: string): ContactDetails | null {
  const contact: unknown = JSON.parse(json);
  if (contact === null) return null;
  if (!isContact(contact) || !LISTS.every(key => {
    const list: unknown = Reflect.get(contact, key);
    return Array.isArray(list) && list.every(isLabeledValue);
  }) || !["organization", "note"].every(key => {
    const text: unknown = Reflect.get(contact, key);
    return text === null || typeof text === "string";
  })) throw new NativeError("protocol", "Invalid contact details");
  return contact as ContactDetails;
}
