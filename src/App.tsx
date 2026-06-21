import { useEffect, useState } from "react";
import { BrowserRouter, NavLink, Navigate, Route, Routes } from "react-router-dom";
import { LoadingSpinner } from "./components/shared/LoadingSpinner";
import { isOnboardingRequired } from "./lib/invoke";
import { HistoryPage } from "./pages/History";
import { OnboardingPage } from "./pages/Onboarding";
import { PracticePage } from "./pages/Practice";
import { SettingsPage } from "./pages/Settings";
import "./App.css";

function AppShell() {
  const [checking, setChecking] = useState(true);
  const [needsOnboarding, setNeedsOnboarding] = useState(true);

  useEffect(() => {
    isOnboardingRequired()
      .then(setNeedsOnboarding)
      .catch(() => setNeedsOnboarding(true))
      .finally(() => setChecking(false));
  }, []);

  if (checking) {
    return (
      <div className="app-shell">
        <LoadingSpinner label="Iniciando Kotoba…" />
      </div>
    );
  }

  return (
    <div className="app-shell">
      <nav aria-label="Navegação principal" className="app-nav">
        <NavLink to="/practice">Prática</NavLink>
        <NavLink to="/history">Histórico</NavLink>
        <NavLink to="/settings">Configurações</NavLink>
      </nav>
      <Routes>
        <Route
          path="/"
          element={
            <Navigate to={needsOnboarding ? "/onboarding" : "/practice"} replace />
          }
        />
        <Route path="/onboarding" element={<OnboardingPage />} />
        <Route path="/practice" element={<PracticePage />} />
        <Route path="/history" element={<HistoryPage />} />
        <Route path="/settings" element={<SettingsPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </div>
  );
}

export default function App() {
  return (
    <BrowserRouter>
      <AppShell />
    </BrowserRouter>
  );
}
