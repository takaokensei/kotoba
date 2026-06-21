import { useCallback, useEffect } from "react";
import { AudioControls } from "../components/practice/AudioControls";
import { FeedbackPanel } from "../components/practice/FeedbackPanel";
import { ScoreDisplay } from "../components/practice/ScoreDisplay";
import { WordDisplay } from "../components/practice/WordDisplay";
import { ErrorBanner } from "../components/shared/ErrorBanner";
import { LoadingSpinner } from "../components/shared/LoadingSpinner";
import { usePracticeSession } from "../hooks/usePracticeSession";
import {
  getNextWord,
  getTutorFeedback,
  scoreAttempt,
} from "../lib/invoke";

export function PracticePage() {
  const { session, setState, setSession } = usePracticeSession();
  const language = "ja" as const;

  const loadWord = useCallback(async () => {
    setState("IDLE");
    try {
      const word = await getNextWord(language);
      setSession((prev) => ({ ...prev, currentWord: word, attempt: null, feedback: null, error: null }));
      setState("AWAITING_INPUT");
    } catch (e) {
      setSession((prev) => ({
        ...prev,
        error: e instanceof Error ? e.message : String(e),
      }));
      setState("ERROR");
    }
  }, [setSession, setState]);

  useEffect(() => {
    loadWord();
  }, [loadWord]);

  async function handleScoreDemo() {
    if (!session.currentWord) return;
    setState("SCORING");
    try {
      const attempt = await scoreAttempt(session.currentWord.id, session.currentWord.reading ?? session.currentWord.word);
      const feedback = await getTutorFeedback(session.currentWord.id, attempt);
      setSession((prev) => ({ ...prev, attempt, feedback }));
      setState("SHOWING_RESULT");
    } catch (e) {
      setSession((prev) => ({
        ...prev,
        error: e instanceof Error ? e.message : String(e),
      }));
      setState("ERROR");
    }
  }

  const wordLabel = session.currentWord?.word ?? "";

  return (
    <main style={{ maxWidth: 640, margin: "0 auto", padding: "2rem" }}>
      <h1>Prática</h1>

      {session.error && <ErrorBanner message={session.error} />}

      {session.currentWord && <WordDisplay word={session.currentWord} />}

      {(session.state === "SCORING" || session.state === "FETCHING_FEEDBACK") && (
        <LoadingSpinner label="Calculando score…" />
      )}

      {session.state === "SHOWING_RESULT" && session.attempt && (
        <>
          <ScoreDisplay
            score={session.attempt.score}
            breakdown={session.attempt.scoreBreakdown}
          />
          {session.feedback && <FeedbackPanel feedback={session.feedback} />}
        </>
      )}

      <AudioControls
        state={session.state}
        wordLabel={wordLabel}
        onPlayTts={() => setState("PLAYING_TTS")}
        onStartRecording={() => setState("RECORDING")}
        onStopRecording={() => {
          setState("TRANSCRIBING");
          handleScoreDemo();
        }}
        onCancelRecording={() => setState("AWAITING_INPUT")}
        onNext={loadWord}
      />

      {session.state === "AWAITING_INPUT" && (
        <p style={{ fontSize: "0.875rem", color: "#666", marginTop: "1rem" }}>
          MVP: gravação/STT chegam na Task 1.3. Use &quot;Gravar&quot; para simular scoring.
        </p>
      )}
    </main>
  );
}
