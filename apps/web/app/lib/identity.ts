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

export function canMintTokenForIdentity(identity: SpillIdentity) {
  return authMode() === "proxy" && identity.source === "upstream";
}

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
  const configuredEmailHeader = envNonempty("SPILLIO_AUTH_EMAIL_HEADER");
  const configuredNameHeader = envNonempty("SPILLIO_AUTH_NAME_HEADER");

  // In proxy deployments clients can send arbitrary request headers, so only
  // the trusted upstream identity header (injected by the IAP/proxy) may
  // establish identity. The permissive list is dev-only.
  const emailHeaderCandidates =
    authMode() === "proxy"
      ? [configuredEmailHeader ?? "x-goog-authenticated-user-email"]
      : [
          configuredEmailHeader,
          "x-spillio-user-email",
          "x-goog-authenticated-user-email",
          "x-forwarded-email",
          "x-auth-request-email",
        ];
  const email = normalizeEmail(firstHeader(headerStore, emailHeaderCandidates));
  const nameHeaderCandidates =
    authMode() === "proxy"
      ? [configuredNameHeader]
      : [
          configuredNameHeader,
          "x-spillio-user-name",
          "x-forwarded-user",
          "x-auth-request-user",
        ];
  const rawName = firstHeader(headerStore, nameHeaderCandidates);

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

function envNonempty(name: string) {
  const value = process.env[name]?.trim();
  return value ? value : undefined;
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
