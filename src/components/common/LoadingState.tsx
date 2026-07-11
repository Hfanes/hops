export function LoadingState({
  shellClassName,
  message,
}: {
  shellClassName: string;
  message: string;
}) {
  return (
    <main className={shellClassName}>
      <section className="loading-state">
        <h1>Hops</h1>
        <p>{message}</p>
      </section>
    </main>
  );
}
