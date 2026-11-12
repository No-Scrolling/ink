import { contacts, type ContactDetails } from "@ink/contacts";
import { lightos } from "@ink/lightos";
import { useEffect, useState } from "react";
import * as v from "valibot";
import {
  EmptyState,
  ErrorState,
  Field,
  LoadingState,
  Screen,
  Text,
  useAction,
  useRouteParams,
} from "ink";

export default function Contact() {
  const { id } = useRouteParams(v.parser(v.object({ id: v.string() })));
  const [state, setState] = useState<{ contact: ContactDetails | null } | { error: Error } | null>(
    null,
  );
  const [attempt, setAttempt] = useState(0);
  const dial = useAction((phoneNumber: string) => lightos.openDialler({ phoneNumber }));
  useEffect(() => {
    const controller = new AbortController();
    contacts.get(id, { signal: controller.signal }).then(
      (contact) => setState({ contact }),
      (error) => {
        if (!controller.signal.aborted)
          setState({ error: error instanceof Error ? error : new Error(String(error)) });
      },
    );
    return () => controller.abort();
  }, [id, attempt]);
  if (state === null)
    return (
      <Screen title="Contact">
        <LoadingState />
      </Screen>
    );
  if ("error" in state)
    return (
      <Screen title="Contact">
        <ErrorState
          message={state.error.message}
          onRetry={() => setAttempt((value) => value + 1)}
        />
      </Screen>
    );
  const { contact } = state;
  if (!contact)
    return (
      <Screen title="Contact">
        <EmptyState title="Contact deleted" />
      </Screen>
    );
  return (
    <Screen title={contact.name}>
      {contact.phones.map((phone) => (
        <Field
          key={`phone:${phone.label}:${phone.value}`}
          label={phone.label}
          onPress={() => dial.run(phone.value)}
        >
          {phone.value}
        </Field>
      ))}
      {[...contact.emails, ...contact.addresses, ...contact.websites, ...contact.events].map(
        (item) => (
          <Field key={`${item.label}:${item.value}`} label={item.label}>
            {item.value}
          </Field>
        ),
      )}
      {contact.organization && <Field label="Organization">{contact.organization}</Field>}
      {contact.note && <Field label="Note">{contact.note}</Field>}
      {dial.status === "error" && <Text>{dial.error.message}</Text>}
    </Screen>
  );
}
