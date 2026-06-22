import { useState } from "react";
import { useModelStatus } from "../../hooks/useModelStatus";
import { downloadModel, listenDownloadProgress } from "../../lib/invoke";
import { ProgressBar } from "../shared/ProgressBar";
import { ErrorBanner } from "../shared/ErrorBanner";

export function ModelManager() {
  const { models, loading, refresh } = useModelStatus();
  const [downloading, setDownloading] = useState<string | null>(null);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);

  if (loading) {
    return (
      <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", padding: "1rem 0" }}>
        <div className="pulse-loading" style={{ color: "#4f46e5", fontWeight: 500 }}>
          Carregando catálogo de modelos...
        </div>
      </div>
    );
  }

  if (models.length === 0) {
    return (
      <p style={{ color: "#6b7280", padding: "1rem 0" }}>
        Nenhum modelo disponível no catálogo.
      </p>
    );
  }

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

  const getDisplayName = (name: string) => {
    switch (name) {
      case "mecab-unidic":
        return "MeCab + UniDic (Japonês)";
      case "whisper-tiny":
        return "Whisper Speech-to-Text (Tiny)";
      case "piper-en":
        return "Piper English Text-to-Speech (US)";
      case "piper-ja":
        return "Piper Japanese Text-to-Speech";
      default:
        return name;
    }
  };

  const getModelDescription = (name: string) => {
    switch (name) {
      case "mecab-unidic":
        return "Mecanismo de análise morfológica essencial para phonemização de japonês e extração de pitch accent.";
      case "whisper-tiny":
        return "Modelo de reconhecimento de fala local ultra-rápido de baixo consumo de memória.";
      case "piper-en":
        return "Síntese de voz em inglês rápida e de alta qualidade executada totalmente local.";
      case "piper-ja":
        return "Síntese de voz em japonês para feedback fonético de áudio nativo.";
      default:
        return "";
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "1.25rem", maxWidth: "650px" }}>
      <p style={{ color: "#4b5563", fontSize: "0.95rem", lineHeight: 1.6, margin: 0 }}>
        Gerencie os modelos de inteligência artificial instalados localmente. Modelos obrigatórios
        são necessários para o funcionamento básico da aplicação.
      </p>

      {error && <ErrorBanner message={error} />}

      <ul
        aria-label="Modelos de IA"
        style={{
          listStyle: "none",
          padding: 0,
          margin: 0,
          display: "flex",
          flexDirection: "column",
          gap: "1rem"
        }}
      >
        {models.map((model) => {
          const isModelDownloading = downloading === model.name;
          return (
            <li
              key={model.name}
              style={{
                background: "linear-gradient(135deg, #ffffff 0%, #f9fafb 100%)",
                border: model.installed ? "1px solid #e5e7eb" : "1px solid #d1d5db",
                borderRadius: "12px",
                padding: "1.25rem 1.5rem",
                boxShadow: "0 4px 6px -1px rgba(0, 0, 0, 0.05), 0 2px 4px -1px rgba(0, 0, 0, 0.03)",
                transition: "all 0.2s ease-in-out",
                position: "relative",
                overflow: "hidden"
              }}
            >
              {/* Top Accent Bar for Installed Status */}
              <div
                style={{
                  position: "absolute",
                  left: 0,
                  top: 0,
                  bottom: 0,
                  width: "4px",
                  background: model.installed ? "#10b981" : "#9ca3af"
                }}
              />

              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: "1rem" }}>
                <div style={{ flex: 1 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "0.50rem", flexWrap: "wrap" }}>
                    <h3 style={{ margin: 0, fontSize: "1.05rem", fontWeight: 600, color: "#111827" }}>
                      {getDisplayName(model.name)}
                    </h3>
                    {model.required && (
                      <span
                        style={{
                          fontSize: "0.7rem",
                          fontWeight: 700,
                          textTransform: "uppercase",
                          letterSpacing: "0.05em",
                          background: "#fef3c7",
                          color: "#92400e",
                          padding: "0.15rem 0.5rem",
                          borderRadius: "4px"
                        }}
                      >
                        Obrigatório
                      </span>
                    )}
                  </div>
                  <p style={{ margin: "0.35rem 0 0.5rem 0", fontSize: "0.85rem", color: "#6b7280", lineHeight: 1.4 }}>
                    {getModelDescription(model.name)}
                  </p>
                  <div style={{ display: "flex", gap: "1rem", fontSize: "0.8rem", color: "#9ca3af" }}>
                    <span>ID: <code>{model.name}</code></span>
                    <span>•</span>
                    <span>Versão: v{model.version}</span>
                    <span>•</span>
                    <span>Tamanho: ~{model.sizeMbEstimate} MB</span>
                  </div>
                </div>

                <div style={{ display: "flex", alignItems: "center", height: "100%" }}>
                  {model.installed ? (
                    <span
                      aria-label={`${model.name} instalado`}
                      style={{
                        color: "#065f46",
                        background: "#d1fae5",
                        fontSize: "0.8rem",
                        fontWeight: 600,
                        padding: "0.35rem 0.75rem",
                        borderRadius: "9999px",
                        display: "inline-flex",
                        alignItems: "center",
                        gap: "0.25rem"
                      }}
                    >
                      ✓ Ativo
                    </span>
                  ) : (
                    <button
                      type="button"
                      disabled={downloading !== null}
                      aria-label={`Instalar ${model.name}`}
                      onClick={() => handleDownload(model.name)}
                      style={{
                        background: downloading !== null ? "#9ca3af" : "linear-gradient(135deg, #3b82f6 0%, #1d4ed8 100%)",
                        color: "#ffffff",
                        border: "none",
                        padding: "0.5rem 1rem",
                        borderRadius: "8px",
                        fontWeight: 600,
                        fontSize: "0.85rem",
                        cursor: downloading !== null ? "not-allowed" : "pointer",
                        boxShadow: downloading !== null ? "none" : "0 4px 6px -1px rgba(59, 130, 246, 0.2)",
                        transition: "all 0.2s ease-in-out"
                      }}
                    >
                      {isModelDownloading ? "Baixando…" : "Instalar"}
                    </button>
                  )}
                </div>
              </div>

              {isModelDownloading && (
                <div style={{ marginTop: "1rem", borderTop: "1px solid #f3f4f6", paddingTop: "0.75rem" }}>
                  <ProgressBar percent={progress} label={`Fazendo download de arquivos da nuvem...`} />
                </div>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
