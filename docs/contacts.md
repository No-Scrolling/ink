---
title: "Contacts"
description: "List and read the device's contacts."
---

Use `@ink/contacts` to list contacts and read a contact's phone numbers, email addresses and other details. Ink reads contacts only; it does not add, change or delete them.

## Permissions

| Permission | Required for |
| --- | --- |
| `contacts` | Listing and reading contacts |

Ink adds Android's `READ_CONTACTS` permission when your app uses `@ink/contacts`. Request access before reading contacts:

```ts
import { lightos } from "@ink/lightos";

const permission = await lightos.requestPermission("contacts");
```

Contact access uses the LightOS permission screen when the connected Light SDK server is 0.2.7 or later, and the Android prompt otherwise. Without access, `contacts.list()` and `contacts.get()` reject with `NativeError.kind === "permission-denied"`. See [Request a permission](/permissions-guide) for the returned statuses.

## List contacts

```ts
import { contacts } from "@ink/contacts";

const all = await contacts.list();
```

`list()` returns every contact that has a name, as `{ id, name }`, sorted by name for the device language. Contacts without a name are left out. Display the result in a [List](/lists), which only renders the rows on screen.

## Search contacts

```ts
const found = await contacts.list({ query: "al" });
```

`query` uses Android's contact filter. It matches the start of any word in a contact's name, nickname or email address, so `"al"` matches both Alice Smith and Mary Allen. It also matches phone numbers. A blank query returns every contact.

## Read a contact

```ts
const person = await contacts.get(id);
if (!person) {
  // The contact was deleted.
}
```

`get()` returns `null` when the contact no longer exists. Otherwise it returns:

| Field | Type | Contents |
| --- | --- | --- |
| `id` | `string` | The contact's current ID. |
| `name` | `string` | Display name. |
| `phones` | `{ label, value }[]` | Phone numbers. |
| `emails` | `{ label, value }[]` | Email addresses. |
| `addresses` | `{ label, value }[]` | Postal addresses. |
| `websites` | `{ label, value }[]` | Websites. |
| `events` | `{ label, value }[]` | Dates such as birthdays, as stored by the contacts app. |
| `organization` | `string \| null` | Job title, company and department. Multiple organizations appear on separate lines. |
| `note` | `string \| null` | Notes, separated by a blank line. |

Labels are Android's localized type names, such as Mobile or Work, or the custom label the user entered. Website labels are in English, as in LightOS. Ink also matches LightOS by skipping blank values and removing duplicates.

Treat contact IDs as opaque strings. They stay valid when the device syncs and merges or splits contacts, so you can store one and pass it to `get()` later. The returned `id` can differ from the one you passed after such a change.

To call a number, pass it to `lightos.openDialler({ phoneNumber })`. See [LightOS](/light-sdk).

## Cancel a request

Both methods accept an `AbortSignal`:

```ts
const controller = new AbortController();
const all = await contacts.list({ signal: controller.signal });
```

Contacts load in the background, so a large address book does not block the interface.
