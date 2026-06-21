import { useCallback, useEffect, useState } from "react";
import {
  downloadModel,
  listenDownloadProgress,
  listAvailableModels,
} from "../../lib/invoke";
import type { ModelCatalogEntry } from "../../lib/types";
import { ProgressBar } from "../shared/ProgressBar";
import { ErrorBanner } from "../shared/ErrorBanner";

interface ModelDownloaderProps {
  onRequiredReady: (ready: boolean) => void;
}

export function ModelDownloader({ onRequiredReady }: ModelDownloaderProps) {
  const [models, setModels] = useState<ModelCatalogEntry[]>([]);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const catalog = await listAvailableModels();
    setModels(catalog);
    const ready = catalog.filter((m) => m.required).every((m) => m.installed);
    onRequiredReady(ready);
  }, [onRequiredReady]);

  useEffect(() => {
    refresh().catch((e) => setError(String(e)));
  }, [refresh]);

  async function handleDownload(name: string) {
    setError(null);
    setDownloading(name);
    setProgress(0);

    const unlisten = await listenDownloadProgress(name, setProgress);

    try {
      await downloadModel(name);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      unlisten();
      setDownloading(null);
      setProgress(0);
    }
  }

  return (
    <section aria-label="Download de modelos">
      <p>
        Baixe os modelos essenciais para transcrição (Whisper) e síntese de voz
        (Piper). Os arquivos ficam apenas no seu computador.
      </p>

      {error && <ErrorBanner message={error} />}

      <ul style={{ listStyle: "none", padding: 0 }}>
        {models.map((model) => (
          <li
            key={model.name}
            style={{
              border: "1px solid #ddd",
              borderRadius: 8,
              padding: "1rem",
              marginBottom: "0.75rem",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem" }}>
              <div>
                <strong>{model.name}</strong>
                {model.required && (
                  <span style={{ marginLeft: 8, fontSize: "0.75rem", color: "#b45309" }}>
                    obrigatório
                  </span>
                )}
                <div style={{ fontSize: "0.875rem", color: "#666" }}>
                  ~{model.sizeMbEstimate} MB
                </div>
              </div>
              {model.installed ? (
                <span aria-label={`${model.name} instalado`} style={{ color: "#15803d" }}>
                  ✓ Instalado
                </span>
              ) : (
                <button
                  type="button"
                  disabled={downloading !== null}
                  aria-label={`Baixar ${model.name}`}
                  onClick={() => handleDownload(model.name)}
                >
                  {downloading === model.name ? "Baixando…" : "Baixar"}
                </button>
              )}
            </div>
            {downloading === model.name && (
              <ProgressBar percent={progress} label={`Baixando ${model.name}`} />
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}
