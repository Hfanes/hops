export function LoadingState({ message }: { message: string }) {
  return (
    <main className="app-shell">
      <section className="loading-state">
        <h1>Hops</h1>
        <p>{message}</p>
      </section>
    </main>
  );
}
