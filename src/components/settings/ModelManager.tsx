import { useModelStatus } from "../../hooks/useModelStatus";

export function ModelManager() {
  const { models, loading } = useModelStatus();

  if (loading) return <p>Carregando modelos…</p>;

  if (models.length === 0) {
    return <p>Nenhum modelo instalado. Conclua o onboarding primeiro.</p>;
  }

  return (
    <ul aria-label="Modelos instalados">
      {models.map((m) => (
        <li key={m.name}>
          {m.name} — v{m.version}
        </li>
      ))}
    </ul>
  );
}
