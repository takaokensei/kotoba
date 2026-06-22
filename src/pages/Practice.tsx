import { useCallback, useEffect, useRef } from "react";
import { AudioControls } from "../components/practice/AudioControls";
import { FeedbackPanel } from "../components/practice/FeedbackPanel";
import { ScoreDisplay } from "../components/practice/ScoreDisplay";
import { WordDisplay } from "../components/practice/WordDisplay";
import { ErrorBanner } from "../components/shared/ErrorBanner";
import { LoadingSpinner } from "../components/shared/LoadingSpinner";
import { usePracticeSession } from "../hooks/usePracticeSession";
import { useAudioCapture } from "../hooks/useAudioCapture";
import {
  getNextWord,
  generateTutorFeedback,
  scoreAttempt,
  speakWord,
} from "../lib/invoke";

export function PracticePage() {
  const { session, setState, setSession } = usePracticeSession();
  const audioCapture = useAudioCapture();
  const recordingPromiseRef = useRef<Promise<string> | null>(null);
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

  const handlePlayTts = useCallback(async () => {
    if (!session.currentWord) return;
    setState("PLAYING_TTS");
    try {
      await speakWord(session.currentWord.id);
    } catch (e) {
      setSession((prev) => ({
        ...prev,
        error: e instanceof Error ? e.message : String(e),
      }));
    } finally {
      // Return to AWAITING_INPUT whether TTS succeeded or failed
      setState("AWAITING_INPUT");
    }
  }, [session.currentWord, setSession, setState]);

  useEffect(() => {
    loadWord();
  }, [loadWord]);

  const processTranscription = useCallback(async (transcript: string) => {
    if (!session.currentWord) return;
    setState("SCORING");
    try {
      const finalTranscript = transcript.trim();
      const attempt = await scoreAttempt(session.currentWord.id, finalTranscript);
      setState("FETCHING_FEEDBACK");
      
      let feedbackText = "";
      let llmUnavailable = false;
      try {
        feedbackText = await generateTutorFeedback(session.currentWord.id, finalTranscript, attempt.score);
      } catch (e) {
        feedbackText = "O tutor local está offline. Certifique-se de que o Ollama está rodando no seu computador com o modelo carregado para receber feedback personalizado.";
        llmUnavailable = true;
      }

      const feedback = {
        text: feedbackText,
        corrections: [],
        llmUnavailable,
      };

      setSession((prev) => ({ ...prev, attempt, feedback, state: "SHOWING_RESULT" }));
    } catch (e) {
      setSession((prev) => ({
        ...prev,
        error: e instanceof Error ? e.message : String(e),
        state: "ERROR",
      }));
    }
  }, [session.currentWord, setSession, setState]);

  const handleStartRecording = useCallback(async () => {
    setState("RECORDING");
    try {
      // Start recording with a max duration of 15 seconds
      const promise = audioCapture.startRecording(15000);
      recordingPromiseRef.current = promise;
      
      const transcript = await promise;
      
      // Auto-stop: If it completes natively (max duration reached), trigger processing
      setSession((prev) => {
        if (prev.state === "RECORDING") {
          processTranscription(transcript);
          return { ...prev, state: "TRANSCRIBING" };
        }
        return prev;
      });
    } catch (e) {
      setSession((prev) => ({
        ...prev,
        error: e instanceof Error ? e.message : String(e),
        state: "ERROR",
      }));
    }
  }, [audioCapture, processTranscription, setSession, setState]);

  const handleStopRecording = useCallback(async () => {
    setState("TRANSCRIBING");
    await audioCapture.stopRecording();
    
    if (recordingPromiseRef.current) {
      try {
        const transcript = await recordingPromiseRef.current;
        await processTranscription(transcript);
      } catch (e) {
        setSession((prev) => ({
          ...prev,
          error: e instanceof Error ? e.message : String(e),
          state: "ERROR",
        }));
      } finally {
        recordingPromiseRef.current = null;
      }
    }
  }, [audioCapture, processTranscription, setSession, setState]);

  const handleCancelRecording = useCallback(async () => {
    setState("AWAITING_INPUT");
    await audioCapture.cancelRecording();
    recordingPromiseRef.current = null;
  }, [audioCapture, setState]);

  const wordLabel = session.currentWord?.word ?? "";

  return (
    <main style={{ maxWidth: 640, margin: "0 auto", padding: "2rem" }}>
      <h1>Prática</h1>

      {session.error && <ErrorBanner message={session.error} />}

      {session.currentWord && <WordDisplay word={session.currentWord} />}

      {(session.state === "SCORING" || session.state === "TRANSCRIBING" || session.state === "PLAYING_TTS") && (
        <LoadingSpinner label={
          session.state === "TRANSCRIBING"
            ? "Transcrevendo áudio..."
            : session.state === "SCORING"
              ? "Calculando score…"
              : "Reproduzindo pronúncia..."
        } />
      )}

      {session.state === "SHOWING_RESULT" && session.attempt && (
        <ScoreDisplay
          score={session.attempt.score}
          breakdown={session.attempt.scoreBreakdown}
        />
      )}

      <AudioControls
        state={session.state}
        wordLabel={wordLabel}
        onPlayTts={handlePlayTts}
        onStartRecording={handleStartRecording}
        onStopRecording={handleStopRecording}
        onCancelRecording={handleCancelRecording}
        onNext={loadWord}
      />

      {(session.state === "FETCHING_FEEDBACK" || (session.state === "SHOWING_RESULT" && session.feedback)) && (
        <FeedbackPanel
          feedback={session.feedback}
          isLoading={session.state === "FETCHING_FEEDBACK"}
        />
      )}
    </main>
  );
}
