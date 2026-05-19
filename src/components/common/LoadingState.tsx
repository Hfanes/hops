export function LoadingState({
  shellClassName,
  message,
}: {
  shellClassName: string;
  message: string;
}) {
  return (
    <main className={shellClassName}>
      <section className="min-h-screen bg-[var(--h-bg)] p-[18px]">
        <h1>Hops</h1>
        <p>{message}</p>
      </section>
    </main>
  );
}
