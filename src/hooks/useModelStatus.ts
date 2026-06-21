import { useEffect, useState } from "react";
import { getModelManifest } from "../lib/invoke";
import type { ModelInfo } from "../lib/types";

export function useModelStatus() {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    getModelManifest()
      .then(setModels)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  return { models, loading };
}
