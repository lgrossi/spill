// Real-design main — assembles polished screens onto a Design Canvas
const { DesignCanvas, DCSection } = window;

function App() {
  return (
    <DesignCanvas>
      <DCSection
        id="real-flow"
        title="🍵 Spill. · Applied Design"
        subtitle="Daylight Cork, applied. Same flow as the locked wireframes, dialed to production: real typography rhythm, paper texture, layered shadows, refined card vocabulary, proper component states."
      >
        {window.real_Overview()}
        {window.real_NewBoard()}
        {window.real_Writing()}
        {window.real_Cluster()}
        {window.real_Voting()}
        {window.real_Action()}
        {window.real_Summary()}
      </DCSection>

      <DCSection
        id="real-system"
        title="Design system reference"
        subtitle="Tokens, surfaces, components — the working pieces behind the screens above."
      >
        {window.real_DesignSystem()}
      </DCSection>
    </DesignCanvas>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<App />);
