import { ModelManager } from "../components/settings/ModelManager";

export function SettingsPage() {
  return (
    <main style={{ maxWidth: 640, margin: "0 auto", padding: "2rem" }}>
      <h1>Configurações</h1>
      <section aria-label="Gerenciador de modelos">
        <h2>Modelos</h2>
        <ModelManager />
      </section>
      <section aria-label="Privacidade">
        <h2>Privacidade</h2>
        <p>Áudio não é persistido por padrão (ADR-009).</p>
      </section>
    </main>
  );
}
