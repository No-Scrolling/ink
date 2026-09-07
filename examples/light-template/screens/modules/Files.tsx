import "@ink/network";
import { useEffect, useRef, useState } from "react";
import { files, type FileRef } from "@ink/files";
import { Button, Field, Image, Row, Screen, useAction, useSnapshot } from "ink";
import { attachments } from "../../data/attachments";

export default function Files() {
  const saved = useSnapshot(attachments);
  const [file, setFile] = useState<FileRef | null>(null);
  const [savedFiles, setSavedFiles] = useState<FileRef[]>([]);
  const [readError, setReadError] = useState<Error | null>(null);
  useEffect(() => {
    let active = true;
    if (saved.status === "ready") void Promise.all(saved.data.map(id => files.open(id))).then(values => {
      if (!active) return;
      setSavedFiles(values.filter((value): value is FileRef => value !== null));
      setReadError(null);
    }, error => { if (active) setReadError(error instanceof Error ? error : new Error(String(error))); });
    return () => { active = false; };
  }, [saved]);
  const active = useRef<AbortController | null>(null);
  useEffect(() => () => active.current?.abort(), []);
  const action = useAction(async (run: () => Promise<string>) => run());
  const keep = async (selected: FileRef) => {
    await attachments.update(ids => ids.includes(selected.id) ? ids : [...ids, selected.id]);
    setFile(selected);
    return "Attachment saved";
  };
  const pending = action.status === "pending";
  return (
    <Screen title="Files and media">
      <Button href="/modules/files/media">Choose media</Button>
      <Button disabled={pending} onPress={() => action.run(async () => {
        const selected = await files.pick({ types: ["application/pdf", "text/plain"] });
        return selected ? keep(selected) : "Selection cancelled";
      })}>Choose document</Button>
      {savedFiles.map(savedFile => (
        <Row key={savedFile.id} title={savedFile.name} image={savedFile.mimeType.startsWith("image/") ? savedFile.src : undefined} onPress={() => action.run(async () => {
          const selected = await files.open(savedFile.id);
          if (!selected) {
            await attachments.update(ids => ids.filter(value => value !== savedFile.id));
            return "Attachment has been removed";
          }
          setFile(selected);
          return "Attachment opened";
        })} />
      ))}
      {file && <>
        <Field label="Attachment">{`${file.name} · ${file.size} bytes`}</Field>
        {file.mimeType.startsWith("image/") && <>
          <Image src={file.src} width={280} height={220} />
          <Button disabled={pending} onPress={() => action.run(async () => keep(await files.prepareImage(file, { maxWidth: 1024, maxHeight: 1024 })))}>Prepare smaller image</Button>
        </>}
        <Button disabled={pending} onPress={() => action.run(async () => { await files.save(file); return "Save dialogue closed"; })}>Save a copy</Button>
        <Button disabled={pending} onPress={() => action.run(async () => { await files.share(file); return "Share destinations opened"; })}>Share attachment</Button>
        <Button disabled={pending} onPress={() => action.run(async () => {
          const controller = new AbortController();
          active.current = controller;
          const signal = AbortSignal.any([controller.signal, AbortSignal.timeout(120_000)]);
          try {
            const blob = await (await fetch(file.src, { signal })).blob();
            const form = new FormData();
            form.append("name", file.name);
            form.append("file", blob, file.name);
            const response = await fetch("http://10.0.2.2:8788/upload", { method: "POST", body: form, signal });
            if (!response.ok) throw new Error(`Upload failed (HTTP ${response.status})`);
            return await response.text();
          } finally { if (active.current === controller) active.current = null; }
        })}>Upload attachment</Button>
        {pending && <Button onPress={() => active.current?.abort()}>Cancel upload</Button>}
        <Button disabled={pending} onPress={() => action.run(async () => {
          await files.remove(file.id);
          await attachments.update(ids => ids.filter(id => id !== file.id));
          setFile(null);
          return "Attachment deleted";
        })}>Delete attachment</Button>
      </>}
      {saved.status === "error" && <Field label="Saved attachments">{saved.error.message}</Field>}
      {readError && <Field label="Saved attachments">{readError.message}</Field>}
      <Field label="Result">{action.status === "success" ? action.data : action.status === "error" ? action.error.message : pending ? "Working…" : "Choose an attachment"}</Field>
    </Screen>
  );
}
