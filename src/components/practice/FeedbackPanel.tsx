import type { TutorFeedback } from "../../lib/types";

export function FeedbackPanel({ feedback }: { feedback: TutorFeedback }) {
  return (
    <section aria-label="Feedback do tutor">
      <p>{feedback.text}</p>
      {feedback.llmUnavailable && (
        <p style={{ color: "#666", fontSize: "0.875rem" }}>
          Tutor offline — score determinístico exibido.
        </p>
      )}
    </section>
  );
}
