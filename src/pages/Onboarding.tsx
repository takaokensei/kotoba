import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { ConsentForm } from "../components/onboarding/ConsentForm";
import { ModelDownloader } from "../components/onboarding/ModelDownloader";
import { PrerequisiteCheck } from "../components/onboarding/PrerequisiteCheck";
import { ErrorBanner } from "../components/shared/ErrorBanner";
import { LoadingSpinner } from "../components/shared/LoadingSpinner";
import {
  checkPrerequisites,
  isOnboardingRequired,
  saveConsent,
} from "../lib/invoke";
import type { OnboardingStep, PrerequisiteStatus } from "../lib/types";

export function OnboardingPage() {
  const navigate = useNavigate();
  const [step, setStep] = useState<OnboardingStep>("prerequisites");
  const [prereq, setPrereq] = useState<PrerequisiteStatus | null>(null);
  const [audioPersisted, setAudioPersisted] = useState(false);
  const [modelsReady, setModelsReady] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    isOnboardingRequired()
      .then((required) => {
        if (!required) {
          navigate("/practice", { replace: true });
          return;
        }
        return checkPrerequisites().then(setPrereq);
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [navigate]);

  async function finishOnboarding() {
    setError(null);
    try {
      await saveConsent(audioPersisted);
      setStep("complete");
      navigate("/practice", { replace: true });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  if (loading) return <LoadingSpinner label="Verificando setup…" />;

  return (
    <main style={{ maxWidth: 560, margin: "0 auto", padding: "2rem" }}>
      <h1>Bem-vindo ao Kotoba</h1>
      <p>Configuração inicial — processamento 100% local.</p>

      {error && <ErrorBanner message={error} />}

      {step === "prerequisites" && (
        <>
          <PrerequisiteCheck status={prereq} />
          <button type="button" onClick={() => setStep("models")}>
            Continuar
          </button>
        </>
      )}

      {step === "models" && (
        <>
          <ModelDownloader onRequiredReady={setModelsReady} />
          <button
            type="button"
            disabled={!modelsReady}
            aria-label="Continuar para consentimento de privacidade"
            onClick={() => setStep("consent")}
          >
            {modelsReady ? "Continuar" : "Baixe os modelos obrigatórios para continuar"}
          </button>
        </>
      )}

      {step === "consent" && (
        <>
          <ConsentForm audioPersisted={audioPersisted} onChange={setAudioPersisted} />
          <button type="button" onClick={finishOnboarding}>
            Concluir setup
          </button>
        </>
      )}
    </main>
  );
}
