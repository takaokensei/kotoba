import type { PracticeWord } from "../../lib/types";

export function WordDisplay({ word }: { word: PracticeWord }) {
  return (
    <article aria-label={`Palavra alvo: ${word.word}`}>
      <h2 style={{ fontSize: "2rem", margin: 0 }}>{word.word}</h2>
      {word.reading && <p style={{ color: "#666" }}>{word.reading}</p>}
      <p>{word.translation}</p>
      {word.pitchPattern && (
        <p aria-label={`Padrão de pitch accent: ${word.pitchPattern}`}>
          Pitch: {word.pitchPattern}
        </p>
      )}
    </article>
  );
}
