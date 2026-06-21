import { useEffect, useState } from "react";
import { LoadingSpinner } from "../components/shared/LoadingSpinner";
import { listRecentAttempts } from "../lib/invoke";
import type { AttemptRow } from "../lib/types";

export function HistoryPage() {
  const [attempts, setAttempts] = useState<AttemptRow[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    listRecentAttempts(20)
      .then(setAttempts)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  return (
    <main style={{ maxWidth: 640, margin: "0 auto", padding: "2rem" }}>
      <h1>Histórico</h1>
      {loading && <LoadingSpinner />}
      {!loading && attempts.length === 0 && <p>Nenhuma tentativa registrada ainda.</p>}
      <ul aria-label="Tentativas recentes">
        {attempts.map((a) => (
          <li key={a.id}>
            Score {Math.round(a.score)} — {a.spokenTranscript} ({a.createdAt})
          </li>
        ))}
      </ul>
    </main>
  );
}
