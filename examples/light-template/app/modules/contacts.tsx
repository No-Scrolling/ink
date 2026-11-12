import { contacts, type Contact } from "@ink/contacts";
import { lightos } from "@ink/lightos";
import { useEffect, useState } from "react";
import { EmptyState, ErrorState, List, LoadingState, Row, Screen, TextInput } from "ink";

export default function Contacts() {
  const [query, setQuery] = useState("");
  const [attempt, setAttempt] = useState(0);
  const [state, setState] = useState<{ contacts: Contact[] } | { error: Error } | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    void (async () => {
      const permission = await lightos.requestPermission("contacts", { signal: controller.signal });
      if (permission !== "granted") {
        throw new Error(
          permission === "blocked"
            ? "Allow contacts access in app settings."
            : "Contacts access was not granted.",
        );
      }
      return contacts.list({ query, signal: controller.signal });
    })().then(
      (list) => setState({ contacts: list }),
      (error) => {
        if (!controller.signal.aborted)
          setState({ error: error instanceof Error ? error : new Error(String(error)) });
      },
    );
    return () => controller.abort();
  }, [query, attempt]);
  return (
    <Screen title="Contacts">
      <TextInput value={query} onChange={setQuery} placeholder="Search" action="search" />
      {state === null ? (
        <LoadingState />
      ) : "error" in state ? (
        <ErrorState
          message={state.error.message}
          onRetry={() => setAttempt((value) => value + 1)}
        />
      ) : state.contacts.length === 0 ? (
        <EmptyState title={query ? "No matches" : "No contacts"} />
      ) : (
        <List
          items={state.contacts}
          keyExtractor={(contact) => contact.id}
          renderItem={(contact) => (
            <Row
              title={contact.name}
              href={{ path: "/modules/contacts/contact", params: { id: contact.id } }}
            />
          )}
        />
      )}
    </Screen>
  );
}
