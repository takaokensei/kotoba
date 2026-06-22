import { useEffect, useState, useCallback } from "react";
import { listAvailableModels } from "../lib/invoke";
import type { ModelCatalogEntry } from "../lib/types";

export function useModelStatus() {
  const [models, setModels] = useState<ModelCatalogEntry[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const catalog = await listAvailableModels();
      setModels(catalog);
    } catch (err) {
      console.error(err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { models, loading, refresh };
}
