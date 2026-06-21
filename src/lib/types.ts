// ─── Vocabulário ───────────────────────────────────────────────
export interface PracticeWord {
  id: string;
  word: string;
  reading?: string;
  translation: string;
  language: "ja" | "en";
  difficulty: 1 | 2 | 3;
  pitchPattern?: "heiban" | "atamadaka" | "nakadaka" | "odaka";
}

// ─── Resultado de tentativa ─────────────────────────────────────
export interface ScoreBreakdown {
  text: number;
  phonetic?: number;
  pgop?: number;
  pitch?: number;
}

export interface AttemptResult {
  id: string;
  transcription: string;
  score: number;
  scoreBreakdown: ScoreBreakdown;
  scoringVersion: string;
}

// ─── Feedback do tutor ──────────────────────────────────────────
export interface Correction {
  type: "phoneme" | "pitch" | "rhythm" | "vowel_length" | "geminate";
  where: string;
  what: string;
  how: string;
}

export interface TutorFeedback {
  text: string;
  corrections: Correction[];
  llmUnavailable: boolean;
}

// ─── Máquina de estados de prática ─────────────────────────────
export type PracticeState =
  | "IDLE"
  | "PLAYING_TTS"
  | "AWAITING_INPUT"
  | "RECORDING"
  | "TRANSCRIBING"
  | "SCORING"
  | "FETCHING_FEEDBACK"
  | "SHOWING_RESULT"
  | "ERROR";

export interface PracticeSessionState {
  state: PracticeState;
  currentWord: PracticeWord | null;
  attempt: AttemptResult | null;
  feedback: TutorFeedback | null;
  error: string | null;
}

// ─── Model Manifest ─────────────────────────────────────────────
export interface ModelInfo {
  name: string;
  version: string;
  path: string;
  downloadedAt: string;
  sizeMb?: number;
  latestKnownVersion?: string;
  updateAvailable?: boolean;
  lastUpdateCheckAt?: string;
}

export interface ModelCatalogEntry {
  name: string;
  version: string;
  sizeMbEstimate: number;
  required: boolean;
  installed: boolean;
}

export type OnboardingStep = "prerequisites" | "models" | "consent" | "complete";

export interface PrerequisiteStatus {
  ollamaAvailable: boolean;
  ollamaModels: string[];
  ollamaWakeAttempted: boolean;
}

export interface AttemptRow {
  id: string;
  vocabularyId: string;
  spokenTranscript: string;
  score: number;
  scoreBreakdown: string;
  scoringVersion: string;
  audioPersisted: boolean;
  createdAt: string;
}

export interface AppSettings {
  audioPersisted: boolean;
  practiceLanguage: string;
}
