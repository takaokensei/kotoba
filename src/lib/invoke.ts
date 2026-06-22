import { invoke } from "@tauri-apps/api/core";
import { listen, emit } from "@tauri-apps/api/event";
import type {
  AppSettings,
  AttemptResult,
  AttemptRow,
  ModelCatalogEntry,
  ModelInfo,
  PracticeWord,
  PrerequisiteStatus,
  TutorFeedback,
} from "./types";

// ─── Prática ────────────────────────────────────────────────────
export const getNextWord = (language: "ja" | "en") =>
  invoke<PracticeWord>("get_next_word", { language });

export const speakWord = (wordId: string) =>
  invoke<void>("speak_word", { wordId });

export const recordAndTranscribe = (maxDurationMs: number) =>
  invoke<string>("record_and_transcribe", { maxDurationMs });

export const scoreAttempt = (vocabularyId: string, transcript: string) =>
  invoke<AttemptResult>("score_attempt", { vocabularyId, transcript });

export const getTutorFeedback = (
  vocabularyId: string,
  attemptResult: AttemptResult,
) =>
  invoke<TutorFeedback>("get_tutor_feedback", {
    vocabularyId,
    attemptResult,
  });

export const generateTutorFeedback = (
  wordId: string,
  attemptId: string,
  userTranscription: string,
  score: number,
) =>
  invoke<string>("generate_tutor_feedback", {
    wordId,
    attemptId,
    userTranscription,
    score,
  });

export const listRecentAttempts = (limit?: number) =>
  invoke<AttemptRow[]>("list_recent_attempts", { limit });

// ─── Onboarding & Models ────────────────────────────────────────
export const checkPrerequisites = () =>
  invoke<PrerequisiteStatus>("check_prerequisites");

export const isOnboardingRequired = () =>
  invoke<boolean>("is_onboarding_required");

export const listAvailableModels = () =>
  invoke<ModelCatalogEntry[]>("list_available_models");

export const downloadModel = (modelName: string) =>
  invoke<ModelInfo>("download_model", { modelName });

export const listenDownloadProgress = (
  modelName: string,
  onProgress: (pct: number) => void,
) =>
  listen<{ modelName: string; percent: number }>(
    "model-download-progress",
    (event) => {
      if (event.payload.modelName === modelName) {
        onProgress(event.payload.percent);
      }
    },
  );

export const getModelManifest = () =>
  invoke<ModelInfo[]>("get_model_manifest");

export const checkForUpdates = () =>
  invoke<ModelInfo[]>("check_for_updates");

export const deleteModel = (modelName: string) =>
  invoke<void>("delete_model", { modelName });

export const saveConsent = (audioPersisted: boolean) =>
  invoke<void>("save_consent", { audioPersisted });

// ─── Settings ───────────────────────────────────────────────────
export const getSettings = () => invoke<AppSettings>("get_settings");

export const updateSettings = (settings: AppSettings) =>
  invoke<void>("update_settings", { settings });

// ─── Gravação de Áudio ──────────────────────────────────────────
export const stopRecording = () => emit("stop-recording");
export const cancelRecording = () => emit("cancel-recording");
