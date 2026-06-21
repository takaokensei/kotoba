export function ErrorBanner({ message }: { message: string }) {
  return (
    <div role="alert" style={{ background: "#fde8e8", color: "#9b1c1c", padding: "0.75rem 1rem", borderRadius: 6 }}>
      {message}
    </div>
  );
}
