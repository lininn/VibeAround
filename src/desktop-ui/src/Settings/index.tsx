import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RefreshCw, Settings as SettingsIcon, WandSparkles } from "lucide-react";
import { useI18n } from "@va/i18n";

import { StepChannels } from "../Onboarding/components/StepChannels";
import { useChannelAuth } from "../Onboarding/hooks/useChannelAuth";
import type {
  ChannelVerboseConfig,
  ConfigSchemaProperty,
  DiscoveredChannelPlugin,
  PluginRegistryEntry,
  Settings as AppSettings,
} from "../Onboarding/types";
import { Button } from "@/components/ui/button";
import { PageHeader, PageShell, StatusBanner } from "@/components/page";

interface SettingsPageProps {
  onServicesRestarted?: () => void;
}

type Notice = {
  variant: "success" | "warning" | "error";
  message: string;
};

export function SettingsPage({ onServicesRestarted }: SettingsPageProps) {
  const { t } = useI18n();
  const [settings, setSettings] = useState<AppSettings>({});
  const [pluginRegistry, setPluginRegistry] = useState<PluginRegistryEntry[]>([]);
  const [discoveredPlugins, setDiscoveredPlugins] = useState<
    DiscoveredChannelPlugin[]
  >([]);
  const [enabledChannels, setEnabledChannels] = useState<Set<string>>(
    () => new Set(),
  );
  const [channelConfigs, setChannelConfigs] = useState<
    Record<string, Record<string, string>>
  >({});
  const [channelVerbose, setChannelVerbose] = useState<
    Record<string, ChannelVerboseConfig>
  >({});
  const [installingPlugins, setInstallingPlugins] = useState<Set<string>>(
    () => new Set(),
  );
  const [loading, setLoading] = useState(true);
  const [settingsLoaded, setSettingsLoaded] = useState(false);
  const [saving, setSaving] = useState<"idle" | "save" | "restart">("idle");
  const [notice, setNotice] = useState<Notice | null>(null);

  const hydrate = useCallback(
    (
      loadedSettings: AppSettings,
      registry: PluginRegistryEntry[],
      discovered: DiscoveredChannelPlugin[],
    ) => {
      const knownIds = new Set([
        ...registry.map((plugin) => plugin.id),
        ...discovered.map((plugin) => plugin.id),
      ]);
      const channels = isRecord(loadedSettings.channels)
        ? loadedSettings.channels
        : {};
      const enabled = new Set<string>();
      const configs: Record<string, Record<string, string>> = {};
      const verbose: Record<string, ChannelVerboseConfig> = {};

      for (const [id, channelConfig] of Object.entries(channels)) {
        if (!knownIds.has(id) || !isRecord(channelConfig)) continue;
        enabled.add(id);
        const configMap: Record<string, string> = {};
        for (const [key, value] of Object.entries(channelConfig)) {
          if (key !== "verbose" && typeof value === "string") {
            configMap[key] = value;
          }
        }
        configs[id] = configMap;
        verbose[id] = parseChannelVerbose(channelConfig.verbose);
      }

      setEnabledChannels(enabled);
      setChannelConfigs(configs);
      setChannelVerbose(verbose);
    },
    [],
  );

  const load = useCallback(async () => {
    setLoading(true);
    setNotice(null);
    try {
      const [loadedSettings, registry, discovered] = await Promise.all([
        invoke<AppSettings>("get_settings"),
        invoke<PluginRegistryEntry[]>("list_plugin_registry"),
        invoke<DiscoveredChannelPlugin[]>("list_channel_plugins"),
      ]);
      setSettings(loadedSettings);
      setPluginRegistry(registry);
      setDiscoveredPlugins(discovered);
      hydrate(loadedSettings, registry, discovered);
      setSettingsLoaded(true);
    } catch (error) {
      setSettingsLoaded(false);
      setNotice({
        variant: "error",
        message: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setLoading(false);
    }
  }, [hydrate]);

  useEffect(() => {
    void load();
  }, [load]);

  const updateChannelConfig = useCallback(
    (pluginId: string, key: string, value: string) => {
      setChannelConfigs((prev) => ({
        ...prev,
        [pluginId]: { ...(prev[pluginId] ?? {}), [key]: value },
      }));
    },
    [],
  );

  const updateChannelVerbose = useCallback(
    (
      pluginId: string,
      key: keyof ChannelVerboseConfig,
      value: boolean,
    ) => {
      setChannelVerbose((prev) => ({
        ...prev,
        [pluginId]: {
          ...(prev[pluginId] ?? defaultChannelVerbose()),
          [key]: value,
        },
      }));
    },
    [],
  );

  const toggleChannel = useCallback((pluginId: string, enabled: boolean) => {
    setEnabledChannels((prev) => {
      const next = new Set(prev);
      if (enabled) next.add(pluginId);
      else next.delete(pluginId);
      return next;
    });
    if (enabled) {
      setChannelVerbose((prev) =>
        prev[pluginId] ? prev : { ...prev, [pluginId]: defaultChannelVerbose() },
      );
    }
  }, []);

  const installPlugin = useCallback(
    async (pluginId: string, githubUrl: string) => {
      setInstallingPlugins((prev) => new Set(prev).add(pluginId));
      setNotice(null);
      try {
        await invoke("install_plugin", { request: { pluginId, githubUrl } });
        const plugins = await invoke<DiscoveredChannelPlugin[]>(
          "list_channel_plugins",
        );
        setDiscoveredPlugins(plugins);
      } catch (error) {
        setNotice({
          variant: "error",
          message: error instanceof Error ? error.message : String(error),
        });
      } finally {
        setInstallingPlugins((prev) => {
          const next = new Set(prev);
          next.delete(pluginId);
          return next;
        });
      }
    },
    [],
  );

  const { authStates, startAuth, cancelAuth } = useChannelAuth({
    active: true,
    discoveredPlugins,
    channelConfigs,
    onConfigChange: updateChannelConfig,
  });

  const canSubmit = useMemo(
    () => settingsLoaded && !loading && saving === "idle",
    [settingsLoaded, loading, saving],
  );

  const save = useCallback(
    async (restart: boolean) => {
      setSaving(restart ? "restart" : "save");
      setNotice(null);
      try {
        const nextSettings = buildChannelSettings({
          settings,
          pluginRegistry,
          discoveredPlugins,
          enabledChannels,
          channelConfigs,
          channelVerbose,
        });
        await invoke("save_settings", { settings: nextSettings });
        setSettings(nextSettings);

        if (restart) {
          await invoke("restart_services");
          onServicesRestarted?.();
          setNotice({ variant: "success", message: "Settings applied." });
        } else {
          setNotice({ variant: "success", message: "Settings saved." });
        }
      } catch (error) {
        setNotice({
          variant: "error",
          message: error instanceof Error ? error.message : String(error),
        });
      } finally {
        setSaving("idle");
      }
    },
    [
      settings,
      pluginRegistry,
      discoveredPlugins,
      enabledChannels,
      channelConfigs,
      channelVerbose,
      onServicesRestarted,
    ],
  );

  return (
    <PageShell className="space-y-3">
      <PageHeader
        icon={<SettingsIcon className="h-4 w-4 text-primary" />}
        title={t("Settings")}
        actions={
          <>
            <Button
              type="button"
              variant="ghost"
              size="xs"
              onClick={() => void load()}
              disabled={loading}
            >
              <RefreshCw className="h-3 w-3" />
              {t("Refresh")}
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="xs"
              onClick={() => window.location.replace("/onboarding")}
            >
              <WandSparkles className="h-3 w-3" />
              {t("Open Config Wizard")}
            </Button>
          </>
        }
      />

      {notice && (
        <StatusBanner variant={notice.variant}>{t(notice.message)}</StatusBanner>
      )}

      <div className="flex flex-wrap items-center justify-end gap-2">
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={!canSubmit}
          onClick={() => void save(false)}
        >
          {saving === "save" ? t("Saving…") : t("Save")}
        </Button>
        <Button
          type="button"
          size="sm"
          disabled={!canSubmit}
          onClick={() => void save(true)}
        >
          <RefreshCw className="h-3 w-3" />
          {saving === "restart"
            ? t("Restarting services…")
            : t("Save & Restart Services")}
        </Button>
      </div>

      {loading ? (
        <p className="px-1 py-6 text-center text-xs text-muted-foreground">
          {t("Loading…")}
        </p>
      ) : (
        <StepChannels
          pluginRegistry={pluginRegistry}
          discoveredPlugins={discoveredPlugins}
          enabledChannels={enabledChannels}
          channelConfigs={channelConfigs}
          channelVerbose={channelVerbose}
          installingPlugins={installingPlugins}
          authStates={authStates}
          onToggleChannel={toggleChannel}
          onConfigChange={updateChannelConfig}
          onVerboseChange={updateChannelVerbose}
          onInstallPlugin={installPlugin}
          onStartAuth={(pluginId) => void startAuth(pluginId)}
          onCancelAuth={(pluginId) => void cancelAuth(pluginId)}
        />
      )}
    </PageShell>
  );
}

function buildChannelSettings({
  settings,
  pluginRegistry,
  discoveredPlugins,
  enabledChannels,
  channelConfigs,
  channelVerbose,
}: {
  settings: AppSettings;
  pluginRegistry: PluginRegistryEntry[];
  discoveredPlugins: DiscoveredChannelPlugin[];
  enabledChannels: Set<string>;
  channelConfigs: Record<string, Record<string, string>>;
  channelVerbose: Record<string, ChannelVerboseConfig>;
}): AppSettings {
  const result: AppSettings = { ...settings };
  const existingChannels = isRecord(settings.channels) ? settings.channels : {};
  const knownPluginIds = new Set([
    ...pluginRegistry.map((plugin) => plugin.id),
    ...discoveredPlugins.map((plugin) => plugin.id),
  ]);
  const discoveredMap = new Map(
    discoveredPlugins.map((plugin) => [plugin.id, plugin]),
  );
  const channels: Record<string, Record<string, unknown>> = {};

  for (const [id, value] of Object.entries(existingChannels)) {
    if (!knownPluginIds.has(id) && isRecord(value)) {
      channels[id] = { ...value };
    }
  }

  for (const id of knownPluginIds) {
    if (!enabledChannels.has(id)) continue;
    const existing = isRecord(existingChannels[id])
      ? existingChannels[id]
      : {};
    const config: Record<string, unknown> = { ...existing };
    const schemaProps = discoveredMap.get(id)?.configSchema?.properties ?? {};
    const editableKeys = new Set([
      ...Object.entries(schemaProps)
        .filter(([, prop]) => !prop.hidden)
        .map(([key]) => key),
      ...Object.keys(channelConfigs[id] ?? {}),
    ]);

    for (const key of editableKeys) {
      if (key === "verbose") continue;
      const value = channelConfigs[id]?.[key] ?? "";
      const prop = schemaProps[key] as ConfigSchemaProperty | undefined;
      if (value || prop?.default) {
        config[key] = value || prop?.default;
      } else {
        delete config[key];
      }
    }

    const verbose = channelVerbose[id] ?? parseChannelVerbose(config.verbose);
    config.verbose = {
      show_thinking: verbose.show_thinking,
      show_tool_use: verbose.show_tool_use,
    };
    channels[id] = config;
  }

  if (Object.keys(channels).length > 0) {
    result.channels = channels;
  } else {
    delete result.channels;
  }

  return result;
}

function defaultChannelVerbose(): ChannelVerboseConfig {
  return {
    show_thinking: false,
    show_tool_use: false,
  };
}

function parseChannelVerbose(value: unknown): ChannelVerboseConfig {
  if (!isRecord(value)) return defaultChannelVerbose();
  return {
    show_thinking:
      typeof value.show_thinking === "boolean" ? value.show_thinking : false,
    show_tool_use:
      typeof value.show_tool_use === "boolean" ? value.show_tool_use : false,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
