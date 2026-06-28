/**
 * inputGuard.ts
 * -------------
 * A lightweight, declared input boundary that ENFORCES the mitigations the
 * contract's `threat_model ChatbotThreatModel` only *declares*:
 *
 *   threat-prompt-injection -> "deny external execution and require policy review"
 *   threat-secret-leakage   -> "env_access denied and secret_material denied"
 *
 * When a turn matches one of these heuristics, the runtime fails closed: the
 * request is BLOCKED with `action: review` and NO provider call is made.
 *
 * IMPORTANT (honesty): this is a heuristic guard for demonstration, not a
 * security guarantee. The contract declares `security_claims none`; so does
 * this module. Its real value is showing that governance acts at the INPUT
 * boundary, not just the network/secret boundary — and that even if a payload
 * slipped through, tools/shell/network remain disabled by the runtime profile.
 */

export type GuardCategory = "prompt_injection" | "secret_exfiltration" | null;

export interface GuardVerdict {
  flagged: boolean;
  category: GuardCategory;
  matched: string | null; // the (redacted) phrase that tripped the guard
  mappedThreat: string | null; // threat id in the threat_model
  action: "review" | "allow"; // policy `on violation` action
}

interface Rule {
  category: Exclude<GuardCategory, null>;
  mappedThreat: string;
  re: RegExp;
}

const RULES: Rule[] = [
  // ---- prompt injection / jailbreak --------------------------------------
  {
    category: "prompt_injection",
    mappedThreat: "threat-prompt-injection",
    re: /\b(ignore|disregard|forget|override)\b.{0,40}\b(previous|prior|above|earlier|all)\b.{0,20}\b(instruction|instructions|prompt|prompts|rules?|context)\b/i,
  },
  {
    category: "prompt_injection",
    mappedThreat: "threat-prompt-injection",
    re: /\b(you are now|act as|pretend to be|from now on you are)\b/i,
  },
  {
    category: "prompt_injection",
    mappedThreat: "threat-prompt-injection",
    re: /\b(jailbreak|do anything now|\bDAN\b|developer mode|unfiltered)\b/i,
  },
  {
    category: "prompt_injection",
    mappedThreat: "threat-prompt-injection",
    re: /\b(reveal|show|print|repeat|leak)\b.{0,30}\b(system\s*prompt|your instructions|your rules)\b/i,
  },
  // ---- secret / key exfiltration -----------------------------------------
  {
    category: "secret_exfiltration",
    mappedThreat: "threat-secret-leakage",
    re: /\b(reveal|show|print|give me|leak|what is)\b.{0,40}\b(api[\s_-]?key|secret|token|password|credential|env(ironment)?\s*var)/i,
  },
  {
    category: "secret_exfiltration",
    mappedThreat: "threat-secret-leakage",
    re: /OPENAI_API_KEY|process\.env|env:OPENAI/i,
  },
];

export function inspect(message: string): GuardVerdict {
  for (const rule of RULES) {
    const m = rule.re.exec(message);
    if (m) {
      // Surface only a short, sanitised snippet — never echo the full payload.
      const snippet = m[0].slice(0, 60);
      return {
        flagged: true,
        category: rule.category,
        matched: snippet,
        mappedThreat: rule.mappedThreat,
        action: "review",
      };
    }
  }
  return {
    flagged: false,
    category: null,
    matched: null,
    mappedThreat: null,
    action: "allow",
  };
}
