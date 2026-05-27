import { createHash } from "crypto";
import { cookies, headers } from "next/headers";

const EMAIL_COOKIE = "spillio_identity_email";
const NAME_COOKIE = "spillio_identity_name";
const COOKIE_MAX_AGE = 60 * 60 * 24 * 400;

export type SpillIdentity = {
  subject: string;
  displayName: string;
  email: string | null;
  source: "upstream" | "local";
};

export async function currentIdentity(): Promise<SpillIdentity | null> {
  const headerIdentity = await identityFromHeaders();
  if (headerIdentity) {
    return headerIdentity;
  }

  if (authMode() === "proxy") {
    return null;
  }

  const cookieStore = await cookies();
  const email = normalizeEmail(cookieStore.get(EMAIL_COOKIE)?.value ?? "");
  if (!email) {
    return null;
  }

  const displayName = displayNameFrom(
    cookieStore.get(NAME_COOKIE)?.value,
    email,
  );
  return {
    subject: subjectForEmail(email),
    displayName,
    email,
    source: "local",
  };
}

export async function apiIdentityHeaders() {
  const identity = await currentIdentity();
  if (!identity) {
    throw new Error("identity required");
  }
  return {
    "x-spillio-user-subject": identity.subject,
    "x-spillio-user-name": identity.displayName,
    ...(identity.email ? { "x-spillio-user-email": identity.email } : {}),
  };
}

export async function setLocalIdentity(emailInput: string, displayNameInput: string) {
  if (authMode() === "proxy") {
    throw new Error("local identity is disabled");
  }

  const email = normalizeEmail(emailInput);
  if (!email) {
    throw new Error("email is required");
  }

  const displayName = displayNameFrom(displayNameInput, email);
  const cookieStore = await cookies();
  const options = {
    httpOnly: true,
    maxAge: COOKIE_MAX_AGE,
    path: "/",
    sameSite: "lax" as const,
    secure: process.env.NODE_ENV === "production",
  };
  cookieStore.set(EMAIL_COOKIE, email, options);
  cookieStore.set(NAME_COOKIE, displayName, options);
}

export async function clearLocalIdentity() {
  if (authMode() === "proxy") {
    return;
  }

  const cookieStore = await cookies();
  cookieStore.delete(EMAIL_COOKIE);
  cookieStore.delete(NAME_COOKIE);
}

async function identityFromHeaders(): Promise<SpillIdentity | null> {
  const headerStore = await headers();
  const configuredEmailHeader = process.env.SPILLIO_AUTH_EMAIL_HEADER;
  const configuredNameHeader = process.env.SPILLIO_AUTH_NAME_HEADER;

  const email = normalizeEmail(firstHeader(headerStore, [
    configuredEmailHeader,
    "x-spillio-user-email",
    "x-goog-authenticated-user-email",
    "x-forwarded-email",
    "x-auth-request-email",
  ]));
  const rawName = firstHeader(headerStore, [
    configuredNameHeader,
    "x-spillio-user-name",
    "x-forwarded-user",
    "x-auth-request-user",
  ]);

  if (!email) {
    return null;
  }

  const displayName = displayNameFrom(rawName, email);
  return {
    subject: subjectForEmail(email),
    displayName,
    email,
    source: "upstream",
  };
}

function firstHeader(headerStore: Headers, names: Array<string | undefined>) {
  for (const name of names) {
    if (!name) continue;
    const value = headerStore.get(name);
    if (value?.trim()) {
      return value.trim();
    }
  }
  return "";
}

function normalizeEmail(value: string) {
  const raw = value.trim().toLowerCase();
  const email = raw.includes(":") ? raw.split(":").at(-1) ?? "" : raw;
  return email.includes("@") ? email : "";
}

function displayNameFrom(value: string | undefined, fallback: string) {
  const trimmed = value?.trim();
  if (trimmed) {
    return trimmed;
  }
  const localPart = fallback.includes("@") ? fallback.split("@")[0] : fallback;
  return localPart || "Spill user";
}

function subjectForEmail(email: string) {
  return `email:${sha256(email)}`;
}

function sha256(value: string) {
  return createHash("sha256").update(value).digest("hex");
}

export function localIdentityEnabled() {
  return authMode() === "local";
}

function authMode() {
  const configured = process.env.SPILLIO_AUTH_MODE?.trim().toLowerCase();
  if (configured === "local" || configured === "proxy") {
    return configured;
  }
  return process.env.NODE_ENV === "production" ? "proxy" : "local";
}
