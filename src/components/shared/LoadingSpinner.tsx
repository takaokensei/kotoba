export function LoadingSpinner({ label = "Carregando…" }: { label?: string }) {
  return (
    <div role="status" aria-live="polite" style={{ padding: "1rem", textAlign: "center" }}>
      {label}
    </div>
  );
}
