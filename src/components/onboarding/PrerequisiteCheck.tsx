import type { PrerequisiteStatus } from "../../lib/types";

export function PrerequisiteCheck({ status }: { status: PrerequisiteStatus | null }) {
  if (!status) return null;

  return (
    <section aria-label="Verificação de pré-requisitos">
      <p>
        Ollama:{" "}
        {status.ollamaAvailable ? "disponível" : "indisponível (opcional no MVP)"}
      </p>
      {status.ollamaWakeAttempted && !status.ollamaAvailable && (
        <p style={{ fontSize: "0.875rem", color: "#666" }}>
          Tentamos iniciar o daemon automaticamente.
        </p>
      )}
    </section>
  );
}
