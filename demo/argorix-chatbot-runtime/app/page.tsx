import ChatWindow from "@/components/ChatWindow";

export default function Page() {
  return (
    <main className="app">
      <header className="app-header">
        <h1>ArgorixLang Chatbot Runtime Demo</h1>
        <p>
          A chatbot governed by Argorix Lang v1.0 — not an OpenAI wrapper. Every
          turn is validated by a contract, policy, evidence bundle and
          fail-closed runtime before any external call is permitted.
        </p>
      </header>
      <ChatWindow />
    </main>
  );
}
