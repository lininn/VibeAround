import { MessageSquare, Download, ExternalLink, Loader2 } from "lucide-react";
import { QRCodeSVG } from "qrcode.react";

import { PLUGIN_REGISTRY } from "../plugin-registry";
import type { StepChannelsProps, ConfigSchemaProperty } from "../types";

/** Determine if a config field should use password input. */
function isSecretField(key: string): boolean {
  const lower = key.toLowerCase();
  return lower.includes("token") || lower.includes("secret") || lower.includes("password") || lower.includes("key");
}

/** Generate a human-readable label from a JSON schema property. */
function fieldLabel(key: string, prop: ConfigSchemaProperty): string {
  if (prop.description) return prop.description;
  return key.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

export function StepChannels({
  discoveredPlugins,
  enabledChannels,
  channelConfigs,
  installingPlugins,
  installErrors,
  authStates,
  onToggleChannel,
  onConfigChange,
  onInstallPlugin,
  onStartAuth,
  onCancelAuth,
}: StepChannelsProps) {
  // Build lookup that matches by both plugin.json id AND directory name on disk,
  // so plugins whose manifest id differs from the registry id are still found.
  const discoveredMap = new Map<string, (typeof discoveredPlugins)[number]>();
  for (const p of discoveredPlugins) {
    discoveredMap.set(p.id, p);
    if (p.dirName && p.dirName !== p.id) {
      discoveredMap.set(p.dirName, p);
    }
  }

  return (
    <div className="space-y-5">
      <div>
        <h2 className="text-base font-semibold flex items-center gap-2">
          <MessageSquare className="w-4 h-4 text-primary" />
          IM Channels
        </h2>
        <p className="text-xs text-muted-foreground mt-1">
          Connect messaging bots to vibe code from your phone. Install plugins
          from the registry, then configure and enable them.
        </p>
      </div>

      {PLUGIN_REGISTRY.map((entry) => {
        const discovered = discoveredMap.get(entry.id);
        const installing = installingPlugins.has(entry.id);
        const isReady = !!discovered;
        const enabled = enabledChannels.has(entry.id);
        const config = channelConfigs[entry.id] ?? {};
        const authState = authStates[entry.id];
        const installError = installErrors[entry.id];

        return (
          <PluginCard
            key={entry.id}
            pluginId={entry.id}
            name={entry.name}
            description={entry.description}
            githubUrl={entry.github}
            isReady={isReady}
            installing={installing}
            enabled={enabled}
            discovered={discovered}
            config={config}
            authState={authState}
            installError={installError}
            onToggle={(v) => onToggleChannel(entry.id, v)}
            onConfigChange={(k, v) => onConfigChange(entry.id, k, v)}
            onInstall={() => onInstallPlugin(entry.id, entry.github)}
            onStartAuth={() => onStartAuth(entry.id)}
            onCancelAuth={() => onCancelAuth(entry.id)}
          />
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// PluginCard — renders install state, config form, or auth flow
// ---------------------------------------------------------------------------

interface PluginCardProps {
  pluginId: string;
  name: string;
  description: string;
  githubUrl: string;
  isReady: boolean;
  installing: boolean;
  enabled: boolean;
  discovered?: StepChannelsProps["discoveredPlugins"][number];
  config: Record<string, string>;
  authState?: StepChannelsProps["authStates"][string];
  installError?: string;
  onToggle: (enabled: boolean) => void;
  onConfigChange: (key: string, value: string) => void;
  onInstall: () => void;
  onStartAuth: () => void;
  onCancelAuth: () => void;
}

function PluginCard({
  pluginId: _pluginId,
  name,
  description,
  githubUrl,
  isReady,
  installing,
  enabled,
  discovered,
  config,
  authState,
  installError,
  onToggle,
  onConfigChange,
  onInstall,
  onStartAuth,
  onCancelAuth,
}: PluginCardProps) {
  const supportsAuth = discovered?.supportsQrcodeLogin ?? false;
  const schema = discovered?.configSchema;
  const properties = schema?.properties ?? {};
  const required = new Set(schema?.required ?? []);
  const visibleFields = Object.entries(properties).filter(
    ([, prop]) => !prop.hidden
  );

  return (
    <section className="rounded-xl border border-border bg-card overflow-hidden scroll-mt-4">
      <div className="flex items-start justify-between gap-4 px-4 py-4">
        <div className="space-y-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium">{name}</span>
            <a
              href={githubUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="text-muted-foreground hover:text-primary transition-colors"
              title="View on GitHub"
            >
              <ExternalLink className="w-3 h-3" />
            </a>
          </div>
          <div className="text-xs text-muted-foreground max-w-xl">{description}</div>
          {installError && (
            <div className="text-xs text-destructive mt-1">{installError}</div>
          )}
        </div>

        {isReady ? (
          <button
            type="button"
            onClick={() => onToggle(!enabled)}
            className={`relative inline-flex h-6 w-11 shrink-0 items-center rounded-full border transition-colors ${
              enabled ? "border-primary bg-primary" : "border-border bg-muted"
            }`}
            aria-pressed={enabled}
            aria-label={`Toggle ${name}`}
          >
            <span
              className={`inline-block h-5 w-5 transform rounded-full bg-white transition-transform ${
                enabled ? "translate-x-5" : "translate-x-0.5"
              }`}
            />
          </button>
        ) : installing ? (
          <button
            key="installing"
            disabled
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-primary text-primary-foreground text-xs font-medium opacity-50 shrink-0"
          >
            <Loader2 className="w-3 h-3 animate-spin" />
            Installing…
          </button>
        ) : (
          <button
            key="install"
            onClick={onInstall}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-primary text-primary-foreground text-xs font-medium hover:opacity-90 shrink-0"
          >
            <Download className="w-3 h-3" />
            Install
          </button>
        )}
      </div>

      {/* Config form + auth (only when installed AND enabled) */}
      {isReady && enabled && (
        <div className="border-t border-border px-4 py-4 space-y-3">
          {/* Dynamic config fields from configSchema */}
          {visibleFields.length > 0 && (
            <div className="space-y-2">
              {visibleFields.map(([key, prop]) => (
                <label key={key} className="block">
                  <span className="text-xs text-muted-foreground">
                    {fieldLabel(key, prop)}
                    {required.has(key) && <span className="text-destructive ml-0.5">*</span>}
                  </span>
                  <input
                    type={isSecretField(key) ? "password" : "text"}
                    value={config[key] ?? prop.default ?? ""}
                    onChange={(e) => onConfigChange(key, e.target.value)}
                    placeholder={prop.default ?? ""}
                    className="mt-1 w-full rounded-md border border-input bg-background px-3 py-1.5 text-sm outline-none focus:ring-1 focus:ring-ring placeholder:text-muted-foreground/40"
                  />
                </label>
              ))}
            </div>
          )}

          {/* Auth flow (QR login) */}
          {supportsAuth && (
            <AuthFlowSection
              authState={authState}
              onStart={onStartAuth}
              onCancel={onCancelAuth}
            />
          )}
        </div>
      )}
    </section>
  );
}

// ---------------------------------------------------------------------------
// AuthFlowSection — QR code login / pairing code
// ---------------------------------------------------------------------------

function AuthFlowSection({
  authState,
  onStart,
  onCancel,
}: {
  authState?: { status: string; message: string; qrCodeUrl?: string };
  onStart: () => void;
  onCancel: () => void;
}) {
  const status = authState?.status ?? "idle";
  const isBusy = status === "generating" || status === "waiting";

  return (
    <div className="rounded-lg border border-border p-3 space-y-3 bg-muted/20">
      <div className="flex items-center justify-between gap-3">
        <div>
          <div className="text-sm font-medium">QR Login</div>
          <div className="text-xs text-muted-foreground mt-1">
            Generate a QR code, scan it with the app, then wait for authorization.
          </div>
        </div>
        <div className="flex items-center gap-2">
          {isBusy && (
            <button
              onClick={onCancel}
              className="px-3 py-2 rounded-md border border-border text-xs font-medium hover:bg-accent transition-colors"
            >
              Cancel
            </button>
          )}
          <button
            onClick={onStart}
            disabled={isBusy}
            className="px-3 py-2 rounded-md bg-primary text-primary-foreground text-xs font-medium hover:opacity-90 disabled:opacity-50 transition-opacity"
          >
            {status === "connected"
              ? "Reconnect"
              : isBusy
                ? "Waiting…"
                : "Connect"}
          </button>
        </div>
      </div>

      {authState?.message && (
        <div
          className={`text-xs rounded-md px-3 py-2 ${
            status === "error"
              ? "bg-destructive/10 text-destructive"
              : status === "connected"
                ? "bg-primary/10 text-primary"
                : "bg-background text-muted-foreground"
          }`}
        >
          {authState.message}
        </div>
      )}

      {authState?.qrCodeUrl && (
        <div className="flex flex-col items-center gap-2 pt-1 scroll-mt-6">
          <div className="rounded-lg border bg-white p-3 shadow-sm">
            <QRCodeSVG
              value={authState.qrCodeUrl}
              size={176}
              bgColor="#ffffff"
              fgColor="#111111"
              level="M"
              includeMargin
              title="QR code"
            />
          </div>
          <div className="text-[11px] text-muted-foreground text-center">
            Scan with the app and confirm on your phone.
          </div>
        </div>
      )}
    </div>
  );
}
