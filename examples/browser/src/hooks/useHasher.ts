import { useState, useEffect } from 'react';

type Sha3Variant = 'sha3-224' | 'sha3-256' | 'sha3-384' | 'sha3-512' | 'shake128' | 'shake256';

interface Sha3WasmHasher {
  hashSingle(input: Uint8Array): Promise<Uint8Array>;
  hashBatch(inputs: Uint8Array[]): Promise<Uint8Array[]>;
  hashBatchWithLength(inputs: Uint8Array[], outputLength: number): Promise<Uint8Array[]>;
  getVariant(): string;
  getOutputSize(): number;
}

export function useHasher(variant: Sha3Variant) {
  const [hasher, setHasher] = useState<Sha3WasmHasher | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let mounted = true;

    const initHasher = async () => {
      try {
        setLoading(true);
        setError(null);

        // Import and initialize the WASM module
        const module = await import('../../../../pkg/sha3_wasm.js');
        await module.default();

        if (!mounted) return;

        // Create the hasher instance
        const hasherInstance = await new module.Sha3WasmHasher(variant);

        if (!mounted) return;

        setHasher(hasherInstance);
      } catch (err) {
        if (!mounted) return;
        console.error('Failed to initialize hasher:', err);
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    };

    initHasher();

    return () => {
      mounted = false;
    };
  }, [variant]);

  return { hasher, error, loading };
}
