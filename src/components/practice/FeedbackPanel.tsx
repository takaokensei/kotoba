import type { TutorFeedback } from "../../lib/types";

interface FeedbackPanelProps {
  feedback: TutorFeedback | null;
  isLoading: boolean;
}

export function FeedbackPanel({ feedback, isLoading }: FeedbackPanelProps) {
  // Helper to parse inline markdown (e.g., **bold**)
  const parseInline = (text: string) => {
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
  };

  // Helper to parse block markdown (bullet points, paragraphs)
  const parseMarkdown = (text: string) => {
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
  };

  return (
    <section
      aria-label="Tutor Insights"
      style={{
        marginTop: "1.5rem",
        padding: "1.5rem",
        background: "#ffffff",
        border: "1px solid #e2e8f0",
        borderRadius: "12px",
        boxShadow: "0 4px 6px -1px rgb(0 0 0 / 0.05), 0 2px 4px -2px rgb(0 0 0 / 0.05)",
        fontFamily: "inherit",
        wordBreak: "break-word",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "1rem" }}>
        <span role="img" aria-label="brain" style={{ fontSize: "1.25rem" }}>🧠</span>
        <h2 style={{ margin: 0, fontSize: "1.1rem", fontWeight: 700, color: "#1e293b" }}>
          Feedback do Tutor
        </h2>
      </div>

      {isLoading ? (
        <div className="pulse-loading" style={{ display: "flex", flexDirection: "column", gap: "0.75rem", padding: "0.5rem 0" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
            <div
              style={{
                width: "1.25rem",
                height: "1.25rem",
                border: "2px solid #3b82f6",
                borderTopColor: "transparent",
                borderRadius: "50%",
                animation: "spin 1s linear infinite",
              }}
            />
            <span style={{ fontSize: "0.95rem", color: "#2563eb", fontWeight: 500 }}>
              O Tutor está analisando sua pronúncia através do modelo local...
            </span>
          </div>
          <div style={{ height: "12px", background: "#f1f5f9", borderRadius: "6px", width: "90%" }} />
          <div style={{ height: "12px", background: "#f1f5f9", borderRadius: "6px", width: "75%" }} />
        </div>
      ) : feedback ? (
        <div>
          <div style={{ fontSize: "0.95rem" }}>{parseMarkdown(feedback.text)}</div>

          {feedback.llmUnavailable && (
            <div
              style={{
                marginTop: "1rem",
                padding: "0.5rem 0.75rem",
                background: "#fef2f2",
                border: "1px solid #fee2e2",
                borderRadius: "6px",
                color: "#991b1b",
                fontSize: "0.8rem",
                display: "inline-flex",
                alignItems: "center",
                gap: "0.35rem",
              }}
            >
              <span role="img" aria-label="warning">⚠️</span>
              Tutor offline — score determinístico exibido.
            </div>
          )}
        </div>
      ) : null}
    </section>
  );
}
