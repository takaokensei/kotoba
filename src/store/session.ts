import { createContext, useContext } from "react";
import type { PracticeSessionState, PracticeState } from "../lib/types";

const defaultState: PracticeSessionState = {
  state: "IDLE",
  currentWord: null,
  attempt: null,
  feedback: null,
  error: null,
};

export const SessionContext = createContext<{
  session: PracticeSessionState;
  setState: (state: PracticeState) => void;
}>({
  session: defaultState,
  setState: () => undefined,
});

export function useSessionStore() {
  return useContext(SessionContext);
}
