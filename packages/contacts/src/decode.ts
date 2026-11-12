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

export function decodeContacts(json: string): Contact[] {
  const contacts: unknown = JSON.parse(json);
  if (!Array.isArray(contacts) || !contacts.every(isContact)) throw new NativeError("protocol", "Invalid contact list");
  return contacts;
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
