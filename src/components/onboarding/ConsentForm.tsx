interface Props {
  audioPersisted: boolean;
  onChange: (value: boolean) => void;
}

export function ConsentForm({ audioPersisted, onChange }: Props) {
  return (
    <section aria-label="Consentimento de privacidade">
      <p>
        Kotoba processa áudio 100% localmente. Nenhum dado é enviado para servidores
        externos.
      </p>
      <label>
        <input
          type="checkbox"
          checked={audioPersisted}
          onChange={(e) => onChange(e.target.checked)}
        />{" "}
        Salvar gravações de áudio no disco (opt-in, padrão desligado)
      </label>
    </section>
  );
}
