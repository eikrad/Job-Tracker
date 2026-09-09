import { readStoredString, removeStoredKey } from "../storage/localStoragePref";
import { llmKeySet, llmKeyStatus } from "../tauriApi";

/** localStorage keys that hold secrets (migrated to the OS keyring). */
export const LOCAL_SECRET_KEYS = [
  { storageKey: "geminiApiKey", provider: "gemini" },
  { storageKey: "mistralApiKey", provider: "mistral" },
  { storageKey: "scalewayApiKey", provider: "scaleway" },
  { storageKey: "serpApiKey", provider: "serpapi" },
  { storageKey: "braveSearchApiKey", provider: "brave" },
  { storageKey: "googleAccessToken", provider: "google_access_token" },
] as const;

export type SecretMigrationResult = {
  migrated: string[];
  keptLocal: string[];
  warnings: string[];
};

type StatusFn = (provider: string) => Promise<{ configured: boolean; backend: string }>;

/**
 * Copy secrets from localStorage into the keyring store.
 * Only deletes the localStorage entry after status read-back confirms configured.
 * Never overwrites an already-configured store entry.
 */
export async function migrateLocalStorageSecrets(
  deps?: Partial<{
    getItem: (key: string) => string | null;
    removeItem: (key: string) => void;
    setKey: typeof llmKeySet;
    status: StatusFn;
  }>,
): Promise<SecretMigrationResult> {
  const getItem = deps?.getItem ?? readStoredString;
  const removeItem = deps?.removeItem ?? removeStoredKey;
  const setKey = deps?.setKey ?? llmKeySet;
  const status = deps?.status ?? llmKeyStatus;

  const migrated: string[] = [];
  const keptLocal: string[] = [];
  const warnings: string[] = [];

  for (const { storageKey, provider } of LOCAL_SECRET_KEYS) {
    const local = getItem(storageKey)?.trim() ?? "";
    if (!local) continue;

    let already: boolean;
    try {
      already = (await status(provider)).configured;
    } catch (e) {
      warnings.push(`${provider}: status check failed (${String(e)}); keeping localStorage`);
      keptLocal.push(provider);
      continue;
    }

    if (already) {
      removeItem(storageKey);
      migrated.push(provider);
      continue;
    }

    try {
      await setKey(provider, local);
      const confirmed = (await status(provider)).configured;
      if (!confirmed) {
        warnings.push(`${provider}: keyring write did not stick; keeping localStorage`);
        keptLocal.push(provider);
        continue;
      }
      removeItem(storageKey);
      migrated.push(provider);
    } catch (e) {
      warnings.push(`${provider}: ${String(e)}; keeping localStorage`);
      keptLocal.push(provider);
    }
  }

  return { migrated, keptLocal, warnings };
}
