"use client";

import { useState, useTransition, type FormEvent } from "react";
import Link from "next/link";
import type { ServerWithAccessKey } from "@/lib/types";
import form from "../community/editor.module.scss";
import { AccessKeyReveal } from "./AccessKeyReveal";
import { DEFAULT_COLOR, Field, MAX_DESCRIPTION, MAX_NAME } from "./ServerForm";
import { registerServer } from "./actions";
import styles from "./servers.module.scss";

/**
 * Registers a server from a name and description, then shows its access key once. The
 * other details can be filled in on the server's page.
 */
export function RegisterServer() {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [created, setCreated] = useState<ServerWithAccessKey | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, startSaving] = useTransition();

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    startSaving(async () => {
      const res = await registerServer({
        name: name.trim(),
        description: description.trim(),
        splash_url: null,
        icon_url: null,
        color: DEFAULT_COLOR,
        connection_address: "",
      });
      if (res.ok) {
        setCreated(res.data);
        setName("");
        setDescription("");
        setError(null);
      } else {
        setError(res.message);
      }
    });
  }

  if (created) {
    return (
      <div className={form.group}>
        <p className={styles.created}>
          Registered <strong>{created.server.name}</strong> (#{created.server.id}).
        </p>
        <AccessKeyReveal accessKey={created.access_key} />
        <div className={styles.inlineActions}>
          <Link href={`/admin/servers/${created.server.id}`} className={`${form.save} ${styles.linkButton}`}>
            Set up {created.server.name}
          </Link>
          <button type="button" className={form.discard} onClick={() => setCreated(null)}>
            Register another
          </button>
        </div>
      </div>
    );
  }

  return (
    <form className={form.group} onSubmit={onSubmit}>
      <Field label="Name">
        <input className={form.input} value={name} onChange={(e) => setName(e.target.value)} maxLength={MAX_NAME} placeholder="DarkRP #1" required />
      </Field>
      <Field label="Description">
        <textarea
          className={`${form.input} ${form.textarea}`}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          maxLength={MAX_DESCRIPTION}
          rows={2}
        />
      </Field>
      <div className={styles.inlineActions}>
        <button type="submit" className={form.save} disabled={saving || !name.trim()}>
          {saving ? "Registering..." : "Register"}
        </button>
        <p className={form.message} role="status">
          {error ? <span className={form.error}>{error}</span> : "You'll get an access key to link the server with."}
        </p>
      </div>
    </form>
  );
}
