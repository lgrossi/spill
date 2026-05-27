import { setIdentityAction } from "../lib/actions";
import { AppChrome, Btn, Tile, spillColors } from "./spill-ui";

export function IdentityGate({ returnTo = "/" }: { returnTo?: string }) {
  return (
    <AppChrome>
      <div className="flex flex-1 items-center justify-center p-6">
        <Tile className="w-full max-w-lg border-spill-action/50 bg-spill-action/10">
          <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-action">identify yourself</p>
          <h1 className="mt-3 text-[28px] font-extrabold tracking-[-0.03em] text-spill-fg">Use an email, no account needed.</h1>
          <p className="mt-2 text-[13.5px] leading-6 text-[var(--fg-2)]">
            Local mode is self-claimed and not secure. Production deployments should use trusted upstream auth headers with <code>SPILLIO_AUTH_MODE=proxy</code>.
          </p>
          <form action={setIdentityAction} className="mt-5 grid gap-3">
            <input name="return_to" type="hidden" value={returnTo} />
            <label className="grid gap-1 text-[11px] font-bold uppercase tracking-[0.1em] text-spill-muted">
              Email
              <input
                autoComplete="email"
                className="min-h-10 rounded-[8px] border border-spill-line bg-[var(--panel-hi)] px-3 py-2 text-[13px] font-semibold normal-case tracking-normal text-spill-fg outline-none focus:border-spill-action focus:shadow-[var(--focus)]"
                name="email"
                placeholder="you@example.com"
                required
                type="email"
              />
            </label>
            <label className="grid gap-1 text-[11px] font-bold uppercase tracking-[0.1em] text-spill-muted">
              Display name
              <input
                autoComplete="name"
                className="min-h-10 rounded-[8px] border border-spill-line bg-[var(--panel-hi)] px-3 py-2 text-[13px] font-semibold normal-case tracking-normal text-spill-fg outline-none focus:border-spill-action focus:shadow-[var(--focus)]"
                name="display_name"
                placeholder="Your display name"
              />
            </label>
            <div className="mt-1">
              <Btn accent={spillColors.action} kind="primary" type="submit">continue</Btn>
            </div>
          </form>
        </Tile>
      </div>
    </AppChrome>
  );
}

export function IdentityUnavailable() {
  return (
    <AppChrome>
      <div className="flex flex-1 items-center justify-center p-6">
        <Tile className="w-full max-w-lg border-spill-wrong/60 bg-spill-wrong/10">
          <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-wrong">identity unavailable</p>
          <h1 className="mt-3 text-[28px] font-extrabold tracking-[-0.03em] text-spill-fg">Spill needs a verified identity header.</h1>
          <p className="mt-2 text-[13.5px] leading-6 text-[var(--fg-2)]">
            In proxy/IAP mode, access Spill through the configured auth layer. Email defines board ownership and visibility. Header names are configurable with <code>SPILLIO_AUTH_EMAIL_HEADER</code> and <code>SPILLIO_AUTH_NAME_HEADER</code>.
          </p>
        </Tile>
      </div>
    </AppChrome>
  );
}

export function BoardAccessDenied() {
  return (
    <AppChrome>
      <div className="flex flex-1 items-center justify-center p-6">
        <Tile className="w-full max-w-lg border-spill-wrong/60 bg-spill-wrong/10">
          <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-wrong">access denied</p>
          <h1 className="mt-3 text-[28px] font-extrabold tracking-[-0.03em] text-spill-fg">You're not on the guest list.</h1>
          <p className="mt-2 text-[13.5px] leading-6 text-[var(--fg-2)]">
            This board is invite-only. Ask the host to add you, then try again.
          </p>
        </Tile>
      </div>
    </AppChrome>
  );
}
