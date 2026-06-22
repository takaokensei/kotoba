import { useEffect, useState } from "react";
import { LoadingSpinner } from "../components/shared/LoadingSpinner";
import { listRecentAttempts } from "../lib/invoke";
import type { AttemptRow } from "../lib/types";

// Helper to parse inline markdown (e.g., **bold**)
function parseInline(text: string) {
  const parts = text.split(/(\*\*.*?\*\*)/g);
  return parts.map((part, idx) => {
    if (part.startsWith("**") && part.endsWith("**")) {
      return (
        <strong key={idx} style={{ fontWeight: 700, color: "#0f172a" }}>
          {part.slice(2, -2)}
        </strong>
      );
    }
    return part;
  });
}

// Helper to parse block markdown (bullet points, paragraphs)
function parseMarkdown(text: string) {
  const lines = text.split("\n");
  let inList = false;
  const listItems: React.ReactNode[] = [];
  const elements: React.ReactNode[] = [];

  lines.forEach((line, idx) => {
    const trimmed = line.trim();
    const isListItem = trimmed.startsWith("- ") || trimmed.startsWith("* ");

    if (isListItem) {
      if (!inList) {
        inList = true;
      }
      const content = trimmed.substring(2);
      listItems.push(
        <li key={`li-${idx}`} style={{ marginBottom: "0.25rem", color: "#334155" }}>
          {parseInline(content)}
        </li>
      );
    } else {
      if (inList) {
        elements.push(
          <ul key={`ul-${idx}`} style={{ margin: "0 0 1rem 1.25rem", padding: 0, listStyleType: "disc" }}>
            {[...listItems]}
          </ul>
        );
        listItems.length = 0;
        inList = false;
      }

      if (trimmed === "") {
        elements.push(<div key={`spacer-${idx}`} style={{ height: "0.5rem" }} />);
      } else {
        elements.push(
          <p key={`p-${idx}`} style={{ margin: "0 0 0.75rem 0", color: "#334155", lineHeight: "1.6" }}>
            {parseInline(line)}
          </p>
        );
      }
    }
  });

  if (inList) {
    elements.push(
      <ul key="ul-final" style={{ margin: "0 0 1rem 1.25rem", padding: 0, listStyleType: "disc" }}>
        {listItems}
      </ul>
    );
  }

  return elements;
}

// ─── Component: SessionList ─────────────────────────────────────
interface SessionListProps {
  attempts: AttemptRow[];
  selectedId: string | null;
  onSelect: (attempt: AttemptRow) => void;
}

function SessionList({ attempts, selectedId, onSelect }: SessionListProps) {
  const formatTime = (isoString: string) => {
    try {
      const date = new Date(isoString);
      return date.toLocaleDateString("pt-BR", {
        day: "2-digit",
        month: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
      });
    } catch {
      return isoString.substring(0, 16).replace("T", " ");
    }
  };

  const getScoreColor = (score: number) => {
    if (score >= 80) return "#16a34a"; // Green
    if (score >= 50) return "#ca8a04"; // Yellow/Gold
    return "#dc2626"; // Red
  };

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "0.75rem",
        maxHeight: "65vh",
        overflowY: "auto",
        paddingRight: "0.5rem",
      }}
    >
      {attempts.map((attempt) => {
        const isSelected = attempt.id === selectedId;
        const scoreColor = getScoreColor(attempt.score);

        return (
          <button
            key={attempt.id}
            onClick={() => onSelect(attempt)}
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "flex-start",
              width: "100%",
              padding: "1rem",
              background: isSelected ? "#eff6ff" : "#ffffff",
              border: isSelected ? "1.5px solid #2563eb" : "1.5px solid #e2e8f0",
              borderRadius: "12px",
              cursor: "pointer",
              textAlign: "left",
              transition: "all 0.2s ease",
              boxShadow: isSelected
                ? "0 4px 6px -1px rgba(37, 99, 235, 0.1)"
                : "0 1px 3px rgba(0, 0, 0, 0.02)",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", width: "100%", marginBottom: "0.4rem" }}>
              <span style={{ fontWeight: 700, color: "#1e293b", fontSize: "1rem" }}>
                {attempt.word}
              </span>
              <span
                style={{
                  fontWeight: 700,
                  color: scoreColor,
                  background: `${scoreColor}15`,
                  padding: "0.15rem 0.5rem",
                  borderRadius: "6px",
                  fontSize: "0.85rem",
                }}
              >
                {Math.round(attempt.score)}%
              </span>
            </div>
            {attempt.reading && (
              <span style={{ fontSize: "0.85rem", color: "#64748b", marginBottom: "0.25rem" }}>
                {attempt.reading}
              </span>
            )}
            <div style={{ display: "flex", justifyContent: "space-between", width: "100%", marginTop: "0.25rem", fontSize: "0.8rem", color: "#94a3b8" }}>
              <span style={{ textOverflow: "ellipsis", overflow: "hidden", whiteSpace: "nowrap", maxWidth: "140px" }}>
                "{attempt.spokenTranscript || "Sem fala"}"
              </span>
              <span>{formatTime(attempt.createdAt)}</span>
            </div>
          </button>
        );
      })}
    </div>
  );
}

// ─── Component: AttemptDetail ───────────────────────────────────
interface AttemptDetailProps {
  attempt: AttemptRow | null;
}

function AttemptDetail({ attempt }: AttemptDetailProps) {
  if (!attempt) {
    return (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          height: "100%",
          padding: "2rem",
          background: "#f8fafc",
          border: "1.5px dashed #cbd5e1",
          borderRadius: "16px",
          color: "#64748b",
          textAlign: "center",
        }}
      >
        <span role="img" aria-label="sheet" style={{ fontSize: "2.5rem", marginBottom: "1rem" }}>📝</span>
        <p style={{ margin: 0, fontSize: "0.95rem" }}>
          Selecione uma tentativa da lista para ver o feedback detalhado do tutor.
        </p>
      </div>
    );
  }

  const getScoreColor = (score: number) => {
    if (score >= 80) return "#16a34a";
    if (score >= 50) return "#ca8a04";
    return "#dc2626";
  };

  const scoreColor = getScoreColor(attempt.score);

  return (
    <div
      style={{
        background: "#ffffff",
        border: "1.5px solid #e2e8f0",
        borderRadius: "16px",
        padding: "1.5rem",
        boxShadow: "0 4px 6px -1px rgba(0, 0, 0, 0.05)",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", flexWrap: "wrap", gap: "1rem", marginBottom: "1.25rem" }}>
        <div>
          <h2 style={{ margin: 0, fontSize: "1.5rem", fontWeight: 800, color: "#0f172a" }}>
            {attempt.word}
          </h2>
          {attempt.reading && (
            <p style={{ margin: "0.25rem 0 0 0", fontSize: "1rem", color: "#64748b" }}>
              Leitura: {attempt.reading}
            </p>
          )}
          <p style={{ margin: "0.25rem 0 0 0", fontSize: "0.9rem", color: "#64748b", fontStyle: "italic" }}>
            Tradução: {attempt.translation}
          </p>
        </div>
        <div style={{ textAlign: "right" }}>
          <div
            style={{
              fontSize: "1.75rem",
              fontWeight: 800,
              color: scoreColor,
              background: `${scoreColor}12`,
              padding: "0.4rem 1rem",
              borderRadius: "12px",
              display: "inline-block",
            }}
          >
            {Math.round(attempt.score)}%
          </div>
          <p style={{ margin: "0.35rem 0 0 0", fontSize: "0.75rem", color: "#94a3b8" }}>
            Versão: {attempt.scoringVersion}
          </p>
        </div>
      </div>

      <div style={{ borderTop: "1px solid #f1f5f9", paddingTop: "1.25rem", marginBottom: "1.25rem" }}>
        <h3 style={{ margin: "0 0 0.5rem 0", fontSize: "0.95rem", fontWeight: 700, color: "#334155" }}>
          Sua Pronúncia Transcrita
        </h3>
        <p
          style={{
            margin: 0,
            padding: "0.75rem 1rem",
            background: "#f8fafc",
            border: "1px solid #e2e8f0",
            borderRadius: "10px",
            fontSize: "1.1rem",
            color: "#0f172a",
            fontWeight: 500,
          }}
        >
          {attempt.spokenTranscript ? `"${attempt.spokenTranscript}"` : "Nenhuma fala detectada"}
        </p>
      </div>

      <div style={{ borderTop: "1px solid #f1f5f9", paddingTop: "1.25rem" }}>
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "0.75rem" }}>
          <span role="img" aria-label="brain" style={{ fontSize: "1.2rem" }}>🧠</span>
          <h3 style={{ margin: 0, fontSize: "0.95rem", fontWeight: 700, color: "#334155" }}>
            Insights do Tutor Salvos
          </h3>
        </div>

        {attempt.tutorFeedback ? (
          <div
            style={{
              fontSize: "0.95rem",
              background: "#fafafa",
              border: "1px solid #f1f5f9",
              borderRadius: "10px",
              padding: "1rem",
            }}
          >
            {parseMarkdown(attempt.tutorFeedback)}
          </div>
        ) : (
          <p style={{ margin: 0, fontSize: "0.9rem", color: "#94a3b8", fontStyle: "italic" }}>
            Nenhum feedback do tutor foi gerado para esta tentativa (tutor local estava offline no momento).
          </p>
        )}
      </div>
    </div>
  );
}

// ─── Component: HistoryPage ─────────────────────────────────────
export function HistoryPage() {
  const [attempts, setAttempts] = useState<AttemptRow[]>([]);
  const [selectedAttempt, setSelectedAttempt] = useState<AttemptRow | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    listRecentAttempts(50)
      .then((data) => {
        setAttempts(data);
        if (data.length > 0) {
          // Select the first attempt by default
          setSelectedAttempt(data[0]);
        }
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  return (
    <main style={{ maxWidth: 960, margin: "0 auto", padding: "2rem" }}>
      <h1 style={{ fontSize: "2rem", fontWeight: 800, color: "#0f172a", marginBottom: "1.5rem" }}>
        Histórico de Prática
      </h1>

      {loading ? (
        <div style={{ display: "flex", justifyContent: "center", padding: "3rem" }}>
          <LoadingSpinner label="Carregando histórico de tentativas..." />
        </div>
      ) : attempts.length === 0 ? (
        <div
          style={{
            textAlign: "center",
            padding: "4rem 2rem",
            background: "#f8fafc",
            borderRadius: "16px",
            border: "1.5px dashed #e2e8f0",
            color: "#64748b",
          }}
        >
          <span role="img" aria-label="box" style={{ fontSize: "3rem", marginBottom: "1rem", display: "block" }}>📦</span>
          <h2 style={{ fontSize: "1.2rem", fontWeight: 700, color: "#334155", margin: "0 0 0.5rem 0" }}>
            Nenhuma tentativa encontrada
          </h2>
          <p style={{ margin: 0, fontSize: "0.95rem" }}>
            Vá para a tela de Prática para iniciar suas gravações de áudio e receber feedback!
          </p>
        </div>
      ) : (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr",
            gap: "1.5rem",
          }}
          className="history-grid"
        >
          {/* Responsive CSS for split view on larger screens */}
          <style>{`
            @media (min-width: 768px) {
              .history-grid {
                grid-template-columns: 320px 1fr !important;
              }
            }
          `}</style>

          <div>
            <h3
              style={{
                margin: "0 0 1rem 0",
                fontSize: "1rem",
                fontWeight: 700,
                color: "#475569",
              }}
            >
              Tentativas Recentes
            </h3>
            <SessionList
              attempts={attempts}
              selectedId={selectedAttempt?.id ?? null}
              onSelect={setSelectedAttempt}
            />
          </div>

          <div>
            <h3
              style={{
                margin: "0 0 1rem 0",
                fontSize: "1rem",
                fontWeight: 700,
                color: "#475569",
              }}
            >
              Detalhes do Feedback
            </h3>
            <AttemptDetail attempt={selectedAttempt} />
          </div>
        </div>
      )}
    </main>
  );
}
