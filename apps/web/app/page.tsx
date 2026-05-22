const apiBaseUrl = process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://127.0.0.1:4000";

export default function Home() {
  return (
    <main
      style={{
        minHeight: "100vh",
        display: "grid",
        placeItems: "center",
        padding: "48px",
      }}
    >
      <section
        style={{
          maxWidth: "760px",
          background: "var(--card)",
          border: "1px solid rgba(32, 26, 23, 0.12)",
          borderRadius: "28px",
          padding: "40px",
          boxShadow: "0 24px 80px rgba(70, 42, 20, 0.12)",
        }}
      >
        <p style={{ color: "var(--accent)", fontWeight: 700, margin: 0 }}>
          spillio runtime foundation
        </p>
        <h1 style={{ fontSize: "56px", lineHeight: 1, margin: "16px 0" }}>
          SpillItOut
        </h1>
        <p style={{ color: "var(--muted)", fontSize: "20px", lineHeight: 1.5 }}>
          Board-first team retros with private writing, GIF-native cards, voting,
          action follow-through, and optional AI helpers.
        </p>
        <div
          style={{
            marginTop: "28px",
            padding: "16px",
            borderRadius: "18px",
            background: "#fff1e8",
            color: "var(--muted)",
          }}
        >
          API base URL: <code>{apiBaseUrl}</code>
        </div>
      </section>
    </main>
  );
}
