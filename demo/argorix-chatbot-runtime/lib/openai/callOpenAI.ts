/**
 * callOpenAI.ts
 * -------------
 * Server-side ONLY sandboxed provider adapter.
 *
 * This module is the single place allowed to read OPENAI_API_KEY, and it is
 * imported exclusively by the /api/chat route handler (a Node.js server module).
 * It must never be imported by a client component.
 *
 * It is only ever invoked AFTER Argorix has returned a `planned` decision for
 * the sandboxed_provider_adapter, and only when ARGORIX_SANDBOXED_EXTERNAL=true.
 *
 * The API key is sent only in the Authorization header to the configured
 * endpoint. It is never returned, logged, or written to any artifact.
 */

export type OpenAiOutcome =
  | { kind: "answer"; text: string }
  | { kind: "no_key" } // sandboxed_external requested but no key configured
  | { kind: "error"; message: string };

export async function callOpenAI(userMessage: string): Promise<OpenAiOutcome> {
  const apiKey = process.env.OPENAI_API_KEY?.trim();
  if (!apiKey) {
    return { kind: "no_key" };
  }

  const baseUrl = (
    process.env.OPENAI_BASE_URL?.trim() || "https://api.openai.com/v1"
  ).replace(/\/+$/, "");
  const model = process.env.OPENAI_MODEL?.trim() || "gpt-5.2";

  // allowed_operations in the contract is ["responses.create"] -> Responses API.
  const url = `${baseUrl}/responses`;

  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 30_000);

    const res = await fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${apiKey}`,
      },
      body: JSON.stringify({
        model,
        input: [
          {
            role: "system",
            content:
              "You are an assistant governed by an Argorix Lang v1.0 runtime contract. Be concise.",
          },
          { role: "user", content: userMessage },
        ],
      }),
      signal: controller.signal,
    });
    clearTimeout(timer);

    if (!res.ok) {
      // Never echo the response body verbatim — it can contain echoed headers.
      return {
        kind: "error",
        message: `provider responded with HTTP ${res.status}`,
      };
    }

    const data = (await res.json()) as unknown;
    const text = extractText(data);
    return { kind: "answer", text: text || "(provider returned no text)" };
  } catch (err) {
    const message = err instanceof Error ? err.message : "provider call failed";
    // Defensive: ensure no key fragment leaks through an error string.
    const safe = apiKey ? message.split(apiKey).join("[REDACTED]") : message;
    return { kind: "error", message: safe };
  }
}

/** Best-effort extraction across Responses API shapes. */
function extractText(data: unknown): string {
  if (!data || typeof data !== "object") return "";
  const d = data as Record<string, unknown>;

  if (typeof d.output_text === "string") return d.output_text;

  const output = d.output;
  if (Array.isArray(output)) {
    const parts: string[] = [];
    for (const item of output) {
      const content = (item as Record<string, unknown>)?.content;
      if (Array.isArray(content)) {
        for (const c of content) {
          const t = (c as Record<string, unknown>)?.text;
          if (typeof t === "string") parts.push(t);
        }
      }
    }
    if (parts.length) return parts.join("\n");
  }
  return "";
}
