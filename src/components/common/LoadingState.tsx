export function LoadingState({
  shellClassName,
  message,
}: {
  shellClassName: string;
  message: string;
}) {
  return (
    <main className={shellClassName}>
      <section className="h-full overflow-auto border border-[var(--h-border)] bg-[var(--h-bg)] p-[18px] md:shadow-[4px_4px_0_var(--h-shadow)]">
        <h1>Hops</h1>
        <p>{message}</p>
      </section>
    </main>
  );
}
