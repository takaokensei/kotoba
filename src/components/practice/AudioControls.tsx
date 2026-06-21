import type { PracticeState } from "../../lib/types";

interface Props {
  state: PracticeState;
  wordLabel: string;
  onPlayTts: () => void;
  onStartRecording: () => void;
  onStopRecording: () => void;
  onCancelRecording: () => void;
  onNext: () => void;
}

export function AudioControls({
  state,
  wordLabel,
  onPlayTts,
  onStartRecording,
  onStopRecording,
  onCancelRecording,
  onNext,
}: Props) {
  return (
    <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
      {(state === "IDLE" || state === "AWAITING_INPUT") && (
        <>
          <button
            type="button"
            aria-label={`Reproduzir pronúncia de ${wordLabel}`}
            onClick={onPlayTts}
          >
            Ouvir
          </button>
          <button
            type="button"
            aria-label={`Gravar pronúncia de ${wordLabel}`}
            onClick={onStartRecording}
          >
            Gravar
          </button>
        </>
      )}
      {state === "RECORDING" && (
        <>
          <button type="button" aria-label="Parar gravação" onClick={onStopRecording}>
            Parar
          </button>
          <button type="button" aria-label="Cancelar gravação" onClick={onCancelRecording}>
            Cancelar
          </button>
        </>
      )}
      {state === "SHOWING_RESULT" && (
        <button type="button" aria-label="Próxima palavra" onClick={onNext}>
          Próxima
        </button>
      )}
    </div>
  );
}
