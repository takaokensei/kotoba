import { useState, useCallback } from "react";
import { recordAndTranscribe, stopRecording, cancelRecording } from "../lib/invoke";

/** Audio capture hook — implemented in Sprint 1 Task 1.3 */
export function useAudioCapture() {
  const [isRecording, setIsRecording] = useState(false);

  const start = useCallback(async (maxDurationMs: number = 15000): Promise<string> => {
    setIsRecording(true);
    try {
      const transcript = await recordAndTranscribe(maxDurationMs);
      return transcript;
    } finally {
      setIsRecording(false);
    }
  }, []);

  const stop = useCallback(async () => {
    await stopRecording();
  }, []);

  const cancel = useCallback(async () => {
    await cancelRecording();
  }, []);

  return {
    isRecording,
    startRecording: start,
    stopRecording: stop,
    cancelRecording: cancel,
  };
}
