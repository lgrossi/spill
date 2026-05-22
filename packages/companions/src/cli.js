#!/usr/bin/env node
import { readFile } from "node:fs/promises";
import process from "node:process";
import { buildIngestionPayload, buildPrompt, buildReviewEnvelope } from "./payload.js";

async function main(argv) {
  const [command, ...rest] = argv;
  const options = parseArgs(rest);

  if (command === "prompt") {
    const contextText = options.contextFile ? await readFile(options.contextFile, "utf8") : "";
    process.stdout.write(buildPrompt({
      includeLocalContext: options.includeLocalContext,
      includeSessionContext: options.includeSessionContext,
      contextText,
    }));
    return;
  }

  if (command === "draft") {
    const payload = buildIngestionPayload(options);
    process.stdout.write(`${JSON.stringify(buildReviewEnvelope(payload), null, 2)}\n`);
    return;
  }

  if (command === "send") {
    if (options.reject) {
      process.stderr.write("Rejected; nothing sent.\n");
      return;
    }
    if (!options.approve) {
      throw new Error("send requires --approve after review/edit");
    }
    const payload = await readJson(options.file);
    const retroId = requireOption(options.retroId, "retroId");
    const apiUrl = options.apiUrl ?? "http://127.0.0.1:4000";
    const subject = requireOption(options.userSubject, "userSubject");
    const response = await fetch(`${apiUrl}/api/retros/${retroId}/ingest`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        "x-spillio-user-subject": subject,
        "x-spillio-user-name": options.userName ?? subject,
      },
      body: JSON.stringify(payload.payload ?? payload),
    });
    if (!response.ok) {
      const body = await response.text();
      throw new Error(`ingest failed with ${response.status}: ${body}`);
    }
    process.stdout.write(`${JSON.stringify(await response.json(), null, 2)}\n`);
    return;
  }

  process.stdout.write(`Usage:
  spillio-companion prompt [--include-local-context --context-file path] [--include-session-context]
  spillio-companion draft --source pi|claude_code --kind mood|wentWell|wentWrong --text "..." [--placement user_deck|retro_draft --target-column-id uuid]
  spillio-companion send --file payload.json --retro-id uuid --user-subject subject --approve [--api-url url]
`);
}

function parseArgs(args) {
  const options = {};
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (!arg.startsWith("--")) {
      continue;
    }
    const key = toCamel(arg.slice(2));
    const next = args[index + 1];
    if (!next || next.startsWith("--")) {
      options[key] = true;
    } else {
      options[key] = next;
      index += 1;
    }
  }
  return options;
}

async function readJson(file) {
  const content = file ? await readFile(file, "utf8") : await readStdin();
  return JSON.parse(content);
}

async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString("utf8");
}

function requireOption(value, name) {
  if (!value) {
    throw new Error(`${name} is required`);
  }
  return value;
}

function toCamel(value) {
  return value.replaceAll(/-([a-z])/g, (_, letter) => letter.toUpperCase());
}

main(process.argv.slice(2)).catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
});
