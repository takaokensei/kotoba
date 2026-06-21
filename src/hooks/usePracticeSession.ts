import { useCallback, useState } from "react";
import type { PracticeSessionState, PracticeState } from "../lib/types";

export function usePracticeSession() {
  const [session, setSession] = useState<PracticeSessionState>({
    state: "IDLE",
    currentWord: null,
    attempt: null,
    feedback: null,
    error: null,
  });

  const setState = useCallback((state: PracticeState) => {
    setSession((prev) => ({ ...prev, state }));
  }, []);

  return { session, setState, setSession };
}
