import type { ScoreBreakdown } from "../../lib/types";

function scoreColor(score: number): string {
  if (score >= 80) return "#15803d";
  if (score >= 60) return "#ca8a04";
  return "#dc2626";
}

export function ScoreGauge({ score }: { score: number }) {
  return (
    <div
      aria-label={`Score: ${Math.round(score)} de 100`}
      style={{ fontSize: "3rem", fontWeight: 700, color: scoreColor(score) }}
    >
      {Math.round(score)}
    </div>
  );
}

export function BreakdownChart({ breakdown }: { breakdown: ScoreBreakdown }) {
  const items = [
    { label: "Texto", value: breakdown.text },
    ...(breakdown.phonetic != null ? [{ label: "Fonema", value: breakdown.phonetic }] : []),
    ...(breakdown.pitch != null ? [{ label: "Pitch", value: breakdown.pitch }] : []),
  ];

  return (
    <div aria-label="Detalhamento do score">
      {items.map((item) => (
        <div key={item.label} style={{ marginBottom: "0.5rem" }}>
          <span>{item.label}: </span>
          <span>{Math.round(item.value * 100)}%</span>
        </div>
      ))}
    </div>
  );
}

export function ScoreDisplay({
  score,
  breakdown,
}: {
  score: number;
  breakdown: ScoreBreakdown;
}) {
  return (
    <section aria-label="Resultado da pronúncia">
      <ScoreGauge score={score} />
      <BreakdownChart breakdown={breakdown} />
    </section>
  );
}
