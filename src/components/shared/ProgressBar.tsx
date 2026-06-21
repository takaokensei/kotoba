export function ProgressBar({ percent, label }: { percent: number; label?: string }) {
  return (
    <div role="progressbar" aria-valuenow={percent} aria-valuemin={0} aria-valuemax={100}>
      {label && <span>{label}</span>}
      <div style={{ background: "#eee", borderRadius: 4, height: 8, marginTop: 4 }}>
        <div
          style={{
            width: `${percent}%`,
            background: "#4a90d9",
            height: "100%",
            borderRadius: 4,
            transition: "width 0.2s",
          }}
        />
      </div>
    </div>
  );
}
